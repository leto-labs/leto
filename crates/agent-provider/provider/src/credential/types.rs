use std::collections::BTreeMap;

use serde_json::Value;

use crate::credential::CredentialHealth;

/// Transport-ready auth material stored in the shared credential pool.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CredentialMaterial {
    /// Headers applied to outbound requests.
    pub headers: BTreeMap<String, String>,
    /// Optional query parameters reserved for provider integrations that need them.
    pub query_params: BTreeMap<String, String>,
    /// Optional provider-specific metadata.
    pub metadata: BTreeMap<String, Value>,
}

impl CredentialMaterial {
    /// Creates empty credential material.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds or replaces one outbound header.
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    /// Adds or replaces one query parameter.
    pub fn with_query_param(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.query_params.insert(name.into(), value.into());
        self
    }

    /// Adds or replaces one provider-specific metadata field.
    pub fn with_metadata(mut self, name: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(name.into(), value);
        self
    }
}

/// One stored credential entry tracked by the shared credential pool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialEntry {
    /// Stable credential identifier.
    pub id: String,
    /// Whether the credential is available for selection.
    pub enabled: bool,
    /// Request-ready credential material.
    pub material: CredentialMaterial,
    /// In-memory health summary used by selection strategies.
    pub health: CredentialHealth,
}

impl CredentialEntry {
    /// Creates a new enabled credential entry.
    pub fn new(id: impl Into<String>, material: CredentialMaterial) -> Self {
        Self {
            id: id.into(),
            enabled: true,
            material,
            health: CredentialHealth::default(),
        }
    }

    /// Creates a bearer-token credential suitable for OpenAI-compatible APIs.
    pub fn bearer(id: impl Into<String>, token: impl Into<String>) -> Self {
        Self::new(
            id,
            CredentialMaterial::new()
                .with_header("Authorization", format!("Bearer {}", token.into())),
        )
    }

    /// Creates an OpenAI OAuth-style bearer credential with optional account id metadata.
    pub fn openai_oauth(
        id: impl Into<String>,
        access_token: impl Into<String>,
        account_id: Option<String>,
    ) -> Self {
        let mut material = CredentialMaterial::new()
            .with_header("Authorization", format!("Bearer {}", access_token.into()));
        if let Some(account_id) = account_id {
            material = material.with_metadata("account_id", Value::String(account_id));
        }
        Self::new(id, material)
    }

    /// Enables or disables the entry.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Credential material returned after one pool resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedCredential {
    /// Stable credential identifier selected by the pool.
    pub credential_id: String,
    /// Headers applied to outbound requests.
    pub headers: BTreeMap<String, String>,
    /// Optional query parameters reserved for provider integrations that need them.
    pub query_params: BTreeMap<String, String>,
    /// Optional provider-specific metadata.
    pub metadata: BTreeMap<String, Value>,
}

impl From<&CredentialEntry> for ResolvedCredential {
    fn from(entry: &CredentialEntry) -> Self {
        Self {
            credential_id: entry.id.clone(),
            headers: entry.material.headers.clone(),
            query_params: entry.material.query_params.clone(),
            metadata: entry.material.metadata.clone(),
        }
    }
}
