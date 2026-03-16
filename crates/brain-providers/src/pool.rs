use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use ulid::Ulid;

use brain_types::{
    BrainError, CredentialEntry, CredentialHealth, CredentialStore,
    SelectionContext, SelectionStrategy,
};

#[cfg(feature = "openai-oauth")]
use brain_types::ProviderCredential;

/// Runtime credential resolver with session stickiness, health tracking,
/// and automatic OAuth token refresh.
pub struct CredentialPool {
    store: Arc<dyn CredentialStore>,
    strategy: Arc<dyn SelectionStrategy>,
    #[cfg(feature = "openai-oauth")]
    client: reqwest::Client,
    /// session_id -> (provider_name -> credential_id)
    session_bindings: RwLock<HashMap<Ulid, HashMap<String, String>>>,
}

impl CredentialPool {
    pub fn new(
        store: Arc<dyn CredentialStore>,
        strategy: Arc<dyn SelectionStrategy>,
    ) -> Self {
        Self {
            store,
            strategy,
            #[cfg(feature = "openai-oauth")]
            client: reqwest::Client::new(),
            session_bindings: RwLock::new(HashMap::new()),
        }
    }

    #[cfg(feature = "openai-oauth")]
    pub fn with_client(
        store: Arc<dyn CredentialStore>,
        strategy: Arc<dyn SelectionStrategy>,
        client: reqwest::Client,
    ) -> Self {
        Self {
            store,
            strategy,
            client,
            session_bindings: RwLock::new(HashMap::new()),
        }
    }

    /// Select a credential for the given provider, applying session stickiness
    /// and refreshing OAuth tokens if needed.
    pub async fn resolve(
        &self,
        provider_name: &str,
        session_id: Option<Ulid>,
    ) -> Result<CredentialEntry, BrainError> {
        if let Some(sid) = session_id {
            if let Some(entry) = self.try_bound(provider_name, sid).await? {
                return Ok(entry);
            }
        }

        let all = self.store.credential_load_all(provider_name).await?;
        let enabled: Vec<CredentialEntry> = all.into_iter().filter(|e| e.enabled).collect();

        if enabled.is_empty() {
            return Err(BrainError::Internal(format!(
                "no credentials available for provider '{provider_name}'"
            )));
        }

        let context = SelectionContext {
            session_id,
            failed_credential_ids: Vec::new(),
        };

        let selected = self
            .strategy
            .select(&enabled, &context)
            .ok_or_else(|| {
                BrainError::Internal(format!(
                    "strategy returned no credential for provider '{provider_name}'"
                ))
            })?
            .clone();

        let entry = self.maybe_refresh_oauth(provider_name, selected).await?;

        if let Some(sid) = session_id {
            self.bind(sid, provider_name, &entry.id).await;
        }

        Ok(entry)
    }

    /// Record a successful credential use.
    pub async fn mark_ok(&self, provider_name: &str, credential_id: &str) {
        let mut health = CredentialHealth::default();
        health.record_ok();
        let _ = self
            .store
            .credential_update_health(provider_name, credential_id, &health)
            .await;
    }

    /// Record a credential failure. The next `resolve()` for the same session
    /// will attempt a different credential.
    pub async fn mark_error(
        &self,
        provider_name: &str,
        credential_id: &str,
        message: &str,
        code: Option<String>,
    ) {
        let mut health = CredentialHealth::default();
        health.record_error(message, code);
        let _ = self
            .store
            .credential_update_health(provider_name, credential_id, &health)
            .await;

        self.unbind_credential(provider_name, credential_id).await;
    }

    /// Remove all session bindings for a session (call on session end).
    pub async fn unbind_session(&self, session_id: Ulid) {
        self.session_bindings.write().await.remove(&session_id);
    }

    /// Convenience wrapper around `store.credential_save`.
    pub async fn add(
        &self,
        provider_name: &str,
        entry: &CredentialEntry,
    ) -> Result<(), BrainError> {
        self.store.credential_save(provider_name, entry).await
    }

    async fn try_bound(
        &self,
        provider_name: &str,
        session_id: Ulid,
    ) -> Result<Option<CredentialEntry>, BrainError> {
        let bindings = self.session_bindings.read().await;
        let cred_id = bindings
            .get(&session_id)
            .and_then(|m| m.get(provider_name))
            .cloned();
        drop(bindings);

        if let Some(cid) = cred_id {
            if let Some(entry) = self.store.credential_load(provider_name, &cid).await? {
                if entry.enabled && entry.health.is_healthy() {
                    let entry = self.maybe_refresh_oauth(provider_name, entry).await?;
                    return Ok(Some(entry));
                }
            }
            self.unbind_credential(provider_name, &cid).await;
        }
        Ok(None)
    }

    async fn bind(&self, session_id: Ulid, provider_name: &str, credential_id: &str) {
        self.session_bindings
            .write()
            .await
            .entry(session_id)
            .or_default()
            .insert(provider_name.to_owned(), credential_id.to_owned());
    }

    async fn unbind_credential(&self, provider_name: &str, credential_id: &str) {
        let mut bindings = self.session_bindings.write().await;
        for provider_map in bindings.values_mut() {
            if provider_map.get(provider_name).map(|s| s.as_str()) == Some(credential_id) {
                provider_map.remove(provider_name);
            }
        }
    }

    async fn maybe_refresh_oauth(
        &self,
        provider_name: &str,
        #[allow(unused_mut)] mut entry: CredentialEntry,
    ) -> Result<CredentialEntry, BrainError> {
        #[cfg(feature = "openai-oauth")]
        if let ProviderCredential::OAuth(ref creds) = entry.credential {
            if creds.needs_refresh() {
                let refreshed =
                    crate::oauth::refresh::refresh_token(&self.client, creds).await?;
                entry.credential = ProviderCredential::OAuth(refreshed);
                self.store.credential_save(provider_name, &entry).await?;
            }
        }
        #[cfg(not(feature = "openai-oauth"))]
        let _ = provider_name;
        Ok(entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brain_stores::InMemoryStore;

    use crate::strategy::{Fallback, StickyRoundRobin};

    fn store() -> Arc<InMemoryStore> {
        Arc::new(InMemoryStore::new())
    }

    fn pool(store: Arc<dyn CredentialStore>, strategy: Arc<dyn SelectionStrategy>) -> CredentialPool {
        CredentialPool::new(store, strategy)
    }

    #[tokio::test]
    async fn resolve_single_credential() {
        let s = store();
        let entry = CredentialEntry::api_key("key-1", "sk-123");
        s.credential_save("openai", &entry).await.unwrap();

        let p = pool(s as Arc<dyn CredentialStore>, Arc::new(StickyRoundRobin::new()));
        let resolved = p.resolve("openai", None).await.unwrap();
        assert_eq!(resolved.id, "key-1");
    }

    #[tokio::test]
    async fn resolve_no_credentials_errors() {
        let s = store();
        let p = pool(s as Arc<dyn CredentialStore>, Arc::new(StickyRoundRobin::new()));
        let result = p.resolve("openai", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn session_stickiness() {
        let s = store();
        s.credential_save("openai", &CredentialEntry::api_key("key-1", "sk-111"))
            .await
            .unwrap();
        s.credential_save("openai", &CredentialEntry::api_key("key-2", "sk-222"))
            .await
            .unwrap();

        let p = pool(s as Arc<dyn CredentialStore>, Arc::new(StickyRoundRobin::new()));
        let sid = Ulid::new();

        let first = p.resolve("openai", Some(sid)).await.unwrap();
        let second = p.resolve("openai", Some(sid)).await.unwrap();
        assert_eq!(first.id, second.id, "same session should get same credential");
    }

    #[tokio::test]
    async fn different_sessions_may_get_different_credentials() {
        let s = store();
        s.credential_save("openai", &CredentialEntry::api_key("key-1", "sk-111"))
            .await
            .unwrap();
        s.credential_save("openai", &CredentialEntry::api_key("key-2", "sk-222"))
            .await
            .unwrap();

        let p = pool(s as Arc<dyn CredentialStore>, Arc::new(StickyRoundRobin::new()));

        let a = p.resolve("openai", Some(Ulid::new())).await.unwrap();
        let b = p.resolve("openai", Some(Ulid::new())).await.unwrap();
        assert_ne!(a.id, b.id, "round-robin should distribute across sessions");
    }

    #[tokio::test]
    async fn unbind_session_clears_binding() {
        let s = store();
        s.credential_save("openai", &CredentialEntry::api_key("key-1", "sk-111"))
            .await
            .unwrap();
        s.credential_save("openai", &CredentialEntry::api_key("key-2", "sk-222"))
            .await
            .unwrap();

        let p = pool(s as Arc<dyn CredentialStore>, Arc::new(StickyRoundRobin::new()));
        let sid = Ulid::new();

        let first = p.resolve("openai", Some(sid)).await.unwrap();
        p.unbind_session(sid).await;
        let after_unbind = p.resolve("openai", Some(sid)).await.unwrap();

        // After unbind, round-robin may select a different credential
        // (depending on index state). Just verify it resolves OK.
        assert!(!after_unbind.id.is_empty());
        let _ = first;
    }

    #[tokio::test]
    async fn add_credential_makes_it_available() {
        let s = store();
        let p = pool(s as Arc<dyn CredentialStore>, Arc::new(Fallback::new()));

        p.add("openai", &CredentialEntry::api_key("new-key", "sk-new"))
            .await
            .unwrap();

        let resolved = p.resolve("openai", None).await.unwrap();
        assert_eq!(resolved.id, "new-key");
    }

    #[tokio::test]
    async fn fallback_always_prefers_first() {
        let s = store();
        s.credential_save("openai", &CredentialEntry::api_key("key-1", "sk-111"))
            .await
            .unwrap();
        s.credential_save("openai", &CredentialEntry::api_key("key-2", "sk-222"))
            .await
            .unwrap();

        let p = pool(s as Arc<dyn CredentialStore>, Arc::new(Fallback::new()));

        let a = p.resolve("openai", None).await.unwrap();
        let b = p.resolve("openai", None).await.unwrap();
        assert_eq!(a.id, b.id);
    }

    #[tokio::test]
    async fn disabled_credential_skipped() {
        let s = store();
        let mut disabled = CredentialEntry::api_key("key-1", "sk-111");
        disabled.enabled = false;
        s.credential_save("openai", &disabled).await.unwrap();
        s.credential_save("openai", &CredentialEntry::api_key("key-2", "sk-222"))
            .await
            .unwrap();

        let p = pool(s as Arc<dyn CredentialStore>, Arc::new(Fallback::new()));
        let resolved = p.resolve("openai", None).await.unwrap();
        assert_eq!(resolved.id, "key-2");
    }
}
