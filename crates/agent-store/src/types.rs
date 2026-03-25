use std::path::{Component, Path, PathBuf};

use chrono::{DateTime, Utc};
use provider::{Message, RequestOptions};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use agent_runtime::RuntimeConfig;

pub type ProjectId = Ulid;
pub type SessionId = Ulid;
pub type MessageId = Ulid;
pub type CredentialStoreKey = (String, String);

/// Durable project-level configuration used by `agent-core`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectConfig {
    /// Default runtime configuration applied to sessions in the project.
    pub runtime: RuntimeConfig,
    /// Optional default provider name.
    pub default_provider: Option<String>,
    /// Optional default model identifier.
    pub default_model: Option<String>,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            runtime: RuntimeConfig::default(),
            default_provider: None,
            default_model: None,
        }
    }
}

/// Durable project record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub name: Option<String>,
    pub root: Option<PathBuf>,
    pub config: ProjectConfig,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Project {
    /// Creates a new project record.
    pub fn new(name: Option<String>, root: Option<PathBuf>, config: ProjectConfig) -> Self {
        let now = Utc::now();
        Self {
            id: Ulid::new(),
            name,
            root,
            config,
            created_at: now,
            updated_at: now,
        }
    }
}

impl Default for Project {
    fn default() -> Self {
        Self::new(None, None, ProjectConfig::default())
    }
}

/// Partial project update payload.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ProjectUpdate {
    pub name: Option<String>,
    pub config: Option<ProjectConfig>,
}

/// Durable session record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub project_id: ProjectId,
    pub title: Option<String>,
    /// Optional provider override for this session.
    pub provider: Option<String>,
    /// Optional model override for this session.
    pub model: Option<String>,
    /// Optional loop override for this session.
    pub loop_name: Option<String>,
    /// Optional request-level override for this session.
    #[serde(default)]
    pub request: RequestOptions,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Session {
    /// Creates a new session for a project.
    pub fn new(project_id: ProjectId) -> Self {
        let now = Utc::now();
        Self {
            id: Ulid::new(),
            project_id,
            title: None,
            provider: None,
            model: None,
            loop_name: None,
            request: RequestOptions::default(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// Partial session update payload.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SessionUpdate {
    pub title: Option<String>,
    pub provider: Option<Option<String>>,
    pub model: Option<Option<String>>,
    pub loop_name: Option<Option<String>>,
    pub request: Option<RequestOptions>,
}

/// Persisted transcript message record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredMessage {
    pub id: MessageId,
    pub session_id: SessionId,
    pub ordinal: u64,
    pub created_at: DateTime<Utc>,
    pub message: Message,
}

impl StoredMessage {
    /// Creates a new stored message for a session.
    pub fn new(session_id: SessionId, ordinal: u64, message: Message) -> Self {
        Self {
            id: Ulid::new(),
            session_id,
            ordinal,
            created_at: Utc::now(),
            message,
        }
    }
}

/// Stored credential value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProviderCredential {
    ApiKey { api_key: String },
    OAuth(OAuthCredentials),
}

/// Stored OAuth credentials.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OAuthCredentials {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_type: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scopes: Vec<String>,
}

/// Credential health information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CredentialHealth {
    pub last_error: Option<CredentialError>,
    pub updated_at: DateTime<Utc>,
}

impl Default for CredentialHealth {
    fn default() -> Self {
        Self {
            last_error: None,
            updated_at: Utc::now(),
        }
    }
}

/// Structured credential error snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CredentialError {
    pub message: String,
    pub recorded_at: DateTime<Utc>,
}

/// Durable credential entry scoped under a provider and credential id.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CredentialEntry {
    pub label: String,
    pub credential: ProviderCredential,
    pub health: CredentialHealth,
}

impl CredentialEntry {
    /// Creates an API-key credential entry.
    pub fn api_key(label: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            credential: ProviderCredential::ApiKey {
                api_key: api_key.into(),
            },
            health: CredentialHealth::default(),
        }
    }
}

/// Normalizes a project root so repeated lookup uses a stable path.
pub fn normalize_project_root(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(segment) => normalized.push(segment),
            Component::RootDir | Component::Prefix(_) => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_message_wraps_provider_message() {
        let stored = StoredMessage::new(Ulid::new(), 0, Message::user_text("hi"));
        assert_eq!(stored.message.plain_text_lossy(), "hi");
    }

    #[test]
    fn normalize_project_root_elides_dot_segments() {
        let normalized = normalize_project_root(Path::new("/tmp/work/./src/../repo"));
        assert_eq!(normalized, PathBuf::from("/tmp/work/repo"));
    }
}
