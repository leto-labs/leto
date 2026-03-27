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
    /// Optional project-level system prompt persisted as durable transcript
    /// bootstrap guidance.
    pub system_prompt: Option<String>,
    /// Optional default loop name for sessions in the project.
    pub default_loop: Option<String>,
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
            system_prompt: None,
            default_loop: None,
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
    pub refresh_token: String,
    pub client_id: String,
    pub token_endpoint: String,
    pub account_id: Option<String>,
    pub token_type: Option<String>,
    pub expires_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scopes: Vec<String>,
}

const REFRESH_BUFFER_SECS: i64 = 60;

impl OAuthCredentials {
    /// Returns true when the access token is expired or within the proactive
    /// refresh window.
    pub fn needs_refresh(&self) -> bool {
        Utc::now() >= self.expires_at - chrono::Duration::seconds(REFRESH_BUFFER_SECS)
    }
}

/// Credential health information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CredentialHealth {
    pub last_ok: Option<DateTime<Utc>>,
    pub last_error: Option<CredentialError>,
    pub consecutive_errors: u32,
    pub updated_at: DateTime<Utc>,
}

impl Default for CredentialHealth {
    fn default() -> Self {
        Self {
            last_ok: None,
            last_error: None,
            consecutive_errors: 0,
            updated_at: Utc::now(),
        }
    }
}

impl CredentialHealth {
    /// Returns true when the credential has no outstanding recorded failure.
    pub fn is_healthy(&self) -> bool {
        self.consecutive_errors == 0
            || self
                .last_ok
                .map(|ok| {
                    self.last_error
                        .as_ref()
                        .map_or(true, |err| ok > err.recorded_at)
                })
                .unwrap_or(false)
    }

    /// Records a successful use and clears prior error state.
    pub fn record_ok(&mut self) {
        self.last_ok = Some(Utc::now());
        self.last_error = None;
        self.consecutive_errors = 0;
        self.updated_at = Utc::now();
    }

    /// Records a failed use and increments the consecutive error count.
    pub fn record_error(&mut self, message: impl Into<String>, code: Option<String>) {
        self.last_error = Some(CredentialError {
            message: message.into(),
            code,
            recorded_at: Utc::now(),
        });
        self.consecutive_errors += 1;
        self.updated_at = Utc::now();
    }
}

/// Structured credential error snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CredentialError {
    pub message: String,
    pub code: Option<String>,
    pub recorded_at: DateTime<Utc>,
}

/// Durable credential entry scoped under a provider and credential id.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CredentialEntry {
    pub id: String,
    pub label: String,
    pub credential: ProviderCredential,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub health: CredentialHealth,
}

impl CredentialEntry {
    /// Creates an API-key credential entry.
    pub fn api_key(label: impl Into<String>, api_key: impl Into<String>) -> Self {
        let label = label.into();
        Self {
            id: label.clone(),
            label,
            credential: ProviderCredential::ApiKey {
                api_key: api_key.into(),
            },
            enabled: true,
            created_at: Utc::now(),
            health: CredentialHealth::default(),
        }
    }

    /// Creates an OAuth credential entry.
    pub fn oauth(label: impl Into<String>, credential: OAuthCredentials) -> Self {
        let label = label.into();
        let id = credential
            .account_id
            .clone()
            .unwrap_or_else(|| Ulid::new().to_string());
        Self {
            id,
            label,
            credential: ProviderCredential::OAuth(credential),
            enabled: true,
            created_at: Utc::now(),
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
    fn oauth_credentials_near_expiry_need_refresh() {
        let credentials = OAuthCredentials {
            access_token: "access".into(),
            refresh_token: "refresh".into(),
            client_id: "client".into(),
            token_endpoint: "https://example.com/token".into(),
            account_id: Some("acct_123".into()),
            token_type: Some("Bearer".into()),
            expires_at: Utc::now() + chrono::Duration::seconds(30),
            scopes: vec!["openid".into()],
        };
        assert!(credentials.needs_refresh());
    }

    #[test]
    fn credential_health_tracks_failures_and_recovery() {
        let mut health = CredentialHealth::default();
        assert!(health.is_healthy());
        health.record_error("bad gateway", Some("502".into()));
        assert!(!health.is_healthy());
        assert_eq!(health.consecutive_errors, 1);
        assert_eq!(
            health
                .last_error
                .as_ref()
                .and_then(|err| err.code.as_deref()),
            Some("502")
        );
        health.record_ok();
        assert!(health.is_healthy());
        assert_eq!(health.consecutive_errors, 0);
    }

    #[test]
    fn oauth_entry_uses_account_id_as_stable_identifier() {
        let credentials = OAuthCredentials {
            access_token: "access".into(),
            refresh_token: "refresh".into(),
            client_id: "client".into(),
            token_endpoint: "https://example.com/token".into(),
            account_id: Some("acct_123".into()),
            token_type: None,
            expires_at: Utc::now() + chrono::Duration::seconds(600),
            scopes: Vec::new(),
        };
        let entry = CredentialEntry::oauth("OpenAI", credentials);
        assert_eq!(entry.id, "acct_123");
        assert!(entry.enabled);
    }

    #[test]
    fn normalize_project_root_elides_dot_segments() {
        let normalized = normalize_project_root(Path::new("/tmp/work/./src/../repo"));
        assert_eq!(normalized, PathBuf::from("/tmp/work/repo"));
    }

    #[test]
    fn project_config_roundtrips_prompt_and_loop_defaults() {
        let config = ProjectConfig {
            system_prompt: Some("You are helpful.".into()),
            default_loop: Some("planner".into()),
            runtime: RuntimeConfig::default(),
            default_provider: Some("openai".into()),
            default_model: Some("gpt-5".into()),
        };

        let json = serde_json::to_string(&config).unwrap();
        let decoded: ProjectConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.system_prompt.as_deref(), Some("You are helpful."));
        assert_eq!(decoded.default_loop.as_deref(), Some("planner"));
        assert_eq!(decoded.default_provider.as_deref(), Some("openai"));
        assert_eq!(decoded.default_model.as_deref(), Some("gpt-5"));
    }
}
