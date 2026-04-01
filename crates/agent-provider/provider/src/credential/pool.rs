use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::credential::{
    CredentialEntry, CredentialFailure, ResolvedCredential, SelectionContext, SelectionStrategy,
};

/// Error returned by the shared in-memory credential pool.
#[derive(Debug, thiserror::Error)]
pub enum CredentialPoolError {
    /// No enabled credentials were configured for the requested provider name.
    #[error("no credentials configured for provider {provider_name}")]
    NoCredentialsConfigured { provider_name: String },
    /// No credential matched the current selection context.
    #[error("no credential could be selected for provider {provider_name}")]
    NoCredentialSelected { provider_name: String },
}

/// Shared in-memory credential pool usable by standalone provider crates.
pub struct CredentialPool {
    strategy: Arc<dyn SelectionStrategy>,
    entries: RwLock<HashMap<String, Vec<CredentialEntry>>>,
    session_bindings: RwLock<HashMap<String, HashMap<String, String>>>,
}

impl CredentialPool {
    /// Creates an empty credential pool backed by one selection strategy.
    pub fn new(strategy: Arc<dyn SelectionStrategy>) -> Self {
        Self {
            strategy,
            entries: RwLock::new(HashMap::new()),
            session_bindings: RwLock::new(HashMap::new()),
        }
    }

    /// Inserts one credential entry under the given provider namespace.
    pub async fn insert(&self, provider_name: impl Into<String>, entry: CredentialEntry) {
        self.entries
            .write()
            .await
            .entry(provider_name.into())
            .or_default()
            .push(entry);
    }

    /// Replaces all entries registered under one provider namespace.
    pub async fn set_entries(
        &self,
        provider_name: impl Into<String>,
        entries: Vec<CredentialEntry>,
    ) {
        self.entries
            .write()
            .await
            .insert(provider_name.into(), entries);
    }

    /// Returns a snapshot of the entries registered under one provider namespace.
    pub async fn entries(&self, provider_name: &str) -> Vec<CredentialEntry> {
        self.entries
            .read()
            .await
            .get(provider_name)
            .cloned()
            .unwrap_or_default()
    }

    /// Resolves one credential for the requested provider and optional session.
    pub async fn resolve(
        &self,
        provider_name: &str,
        session_id: Option<&str>,
    ) -> Result<ResolvedCredential, CredentialPoolError> {
        if let Some(session_id) = session_id
            && let Some(bound) = self.try_bound(provider_name, session_id).await
        {
            return Ok(bound);
        }

        let entries = self.entries.read().await;
        let provider_entries = entries.get(provider_name).cloned().unwrap_or_default();
        drop(entries);

        let enabled: Vec<CredentialEntry> = provider_entries
            .into_iter()
            .filter(|entry| entry.enabled)
            .collect();

        if enabled.is_empty() {
            return Err(CredentialPoolError::NoCredentialsConfigured {
                provider_name: provider_name.to_owned(),
            });
        }

        let context = SelectionContext {
            session_id: session_id.map(str::to_owned),
            failed_credential_ids: Vec::new(),
        };

        let selected = self.strategy.select(&enabled, &context).ok_or_else(|| {
            CredentialPoolError::NoCredentialSelected {
                provider_name: provider_name.to_owned(),
            }
        })?;

        if let Some(session_id) = session_id {
            self.bind(session_id, provider_name, &selected.id).await;
        }

        Ok(ResolvedCredential::from(selected))
    }

    /// Records a successful use for one credential.
    pub async fn mark_ok(&self, provider_name: &str, credential_id: &str) {
        let mut entries = self.entries.write().await;
        if let Some(provider_entries) = entries.get_mut(provider_name)
            && let Some(entry) = provider_entries
                .iter_mut()
                .find(|entry| entry.id == credential_id)
        {
            entry.health.record_ok();
        }
    }

    /// Records a failure for one credential and unbinds any sticky sessions using it.
    pub async fn mark_error(
        &self,
        provider_name: &str,
        credential_id: &str,
        failure: CredentialFailure,
    ) {
        let mut entries = self.entries.write().await;
        if let Some(provider_entries) = entries.get_mut(provider_name)
            && let Some(entry) = provider_entries
                .iter_mut()
                .find(|entry| entry.id == credential_id)
        {
            entry.health.record_error(failure);
        }
        drop(entries);
        self.unbind_credential(provider_name, credential_id).await;
    }

    async fn try_bound(&self, provider_name: &str, session_id: &str) -> Option<ResolvedCredential> {
        let bindings = self.session_bindings.read().await;
        let credential_id = bindings
            .get(session_id)
            .and_then(|provider_entries| provider_entries.get(provider_name))
            .cloned();
        drop(bindings);

        let credential_id = credential_id?;
        let entries = self.entries.read().await;
        let entry = entries.get(provider_name).and_then(|provider_entries| {
            provider_entries
                .iter()
                .find(|entry| {
                    entry.id == credential_id && entry.enabled && entry.health.is_healthy()
                })
                .cloned()
        });
        drop(entries);

        if entry.is_none() {
            self.unbind_credential(provider_name, &credential_id).await;
        }

        entry.as_ref().map(ResolvedCredential::from)
    }

    async fn bind(&self, session_id: &str, provider_name: &str, credential_id: &str) {
        self.session_bindings
            .write()
            .await
            .entry(session_id.to_owned())
            .or_default()
            .insert(provider_name.to_owned(), credential_id.to_owned());
    }

    async fn unbind_credential(&self, provider_name: &str, credential_id: &str) {
        let mut bindings = self.session_bindings.write().await;
        for provider_entries in bindings.values_mut() {
            if provider_entries.get(provider_name).map(String::as_str) == Some(credential_id) {
                provider_entries.remove(provider_name);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credential::{CredentialMaterial, Fallback, StickyRoundRobin};

    #[tokio::test]
    async fn sticky_round_robin_reuses_bound_session() {
        let pool = CredentialPool::new(Arc::new(StickyRoundRobin::new()));
        pool.insert(
            "openai",
            CredentialEntry::new("key-1", CredentialMaterial::new()),
        )
        .await;
        pool.insert(
            "openai",
            CredentialEntry::new("key-2", CredentialMaterial::new()),
        )
        .await;

        let first = pool.resolve("openai", Some("session-1")).await.unwrap();
        let second = pool.resolve("openai", Some("session-1")).await.unwrap();

        assert_eq!(first.credential_id, second.credential_id);
    }

    #[tokio::test]
    async fn mark_error_unbinds_and_rotates() {
        let pool = CredentialPool::new(Arc::new(StickyRoundRobin::new()));
        pool.insert(
            "openai",
            CredentialEntry::new("key-1", CredentialMaterial::new()),
        )
        .await;
        pool.insert(
            "openai",
            CredentialEntry::new("key-2", CredentialMaterial::new()),
        )
        .await;

        let first = pool.resolve("openai", Some("session-1")).await.unwrap();
        pool.mark_error(
            "openai",
            &first.credential_id,
            CredentialFailure::new("429"),
        )
        .await;
        let second = pool.resolve("openai", Some("session-1")).await.unwrap();

        assert_ne!(first.credential_id, second.credential_id);
    }

    #[tokio::test]
    async fn disabled_credentials_are_skipped() {
        let pool = CredentialPool::new(Arc::new(Fallback::new()));
        pool.insert(
            "openai",
            CredentialEntry::new("disabled", CredentialMaterial::new()).with_enabled(false),
        )
        .await;
        pool.insert(
            "openai",
            CredentialEntry::new("enabled", CredentialMaterial::new()),
        )
        .await;

        let resolved = pool.resolve("openai", None).await.unwrap();
        assert_eq!(resolved.credential_id, "enabled");
    }
}
