use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Stored credential for a provider — either a static API key or
/// OAuth 2.0 tokens that expire and can be refreshed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProviderCredential {
    ApiKey { api_key: String },
    OAuth(OAuthCredentials),
}

/// OAuth 2.0 credentials for a provider. Serializable for persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthCredentials {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: DateTime<Utc>,
    pub client_id: String,
    pub token_endpoint: String,
    pub account_id: Option<String>,
}

const REFRESH_BUFFER_SECS: i64 = 60;

impl OAuthCredentials {
    /// Returns true if the access token is expired or within 60 seconds of expiry.
    pub fn needs_refresh(&self) -> bool {
        Utc::now() >= self.expires_at - chrono::Duration::seconds(REFRESH_BUFFER_SECS)
    }
}

/// A credential with metadata for health tracking, rotation, and identification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialEntry {
    pub id: String,
    pub credential: ProviderCredential,
    pub health: CredentialHealth,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

impl CredentialEntry {
    pub fn api_key(id: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            credential: ProviderCredential::ApiKey {
                api_key: key.into(),
            },
            health: CredentialHealth::default(),
            enabled: true,
            created_at: Utc::now(),
        }
    }

    pub fn oauth(creds: OAuthCredentials) -> Self {
        let id = creds
            .account_id
            .clone()
            .unwrap_or_else(|| ulid::Ulid::new().to_string());
        Self {
            id,
            credential: ProviderCredential::OAuth(creds),
            health: CredentialHealth::default(),
            enabled: true,
            created_at: Utc::now(),
        }
    }
}

/// Health metadata for a credential — tracks successes, failures, and consecutive error count.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialHealth {
    pub last_ok: Option<DateTime<Utc>>,
    pub last_error: Option<CredentialError>,
    pub consecutive_errors: u32,
}

impl Default for CredentialHealth {
    fn default() -> Self {
        Self {
            last_ok: None,
            last_error: None,
            consecutive_errors: 0,
        }
    }
}

impl CredentialHealth {
    pub fn is_healthy(&self) -> bool {
        self.consecutive_errors == 0
            || self
                .last_ok
                .map(|ok| self.last_error.as_ref().map_or(true, |err| ok > err.at))
                .unwrap_or(true)
    }

    pub fn record_ok(&mut self) {
        self.last_ok = Some(Utc::now());
        self.last_error = None;
        self.consecutive_errors = 0;
    }

    pub fn record_error(&mut self, message: impl Into<String>, code: Option<String>) {
        self.last_error = Some(CredentialError {
            at: Utc::now(),
            message: message.into(),
            code,
        });
        self.consecutive_errors += 1;
    }
}

/// A recorded error for a credential.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialError {
    pub at: DateTime<Utc>,
    pub message: String,
    pub code: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn make_creds(expires_at: DateTime<Utc>) -> OAuthCredentials {
        OAuthCredentials {
            access_token: "access".into(),
            refresh_token: "refresh".into(),
            expires_at,
            client_id: "client".into(),
            token_endpoint: "https://example.com/token".into(),
            account_id: None,
        }
    }

    #[test]
    fn token_not_expired() {
        let creds = make_creds(Utc::now() + Duration::seconds(300));
        assert!(!creds.needs_refresh());
    }

    #[test]
    fn token_near_expiry() {
        let creds = make_creds(Utc::now() + Duration::seconds(30));
        assert!(creds.needs_refresh());
    }

    #[test]
    fn token_expired() {
        let creds = make_creds(Utc::now() - Duration::seconds(60));
        assert!(creds.needs_refresh());
    }

    #[test]
    fn api_key_serializes_with_tag() {
        let cred = ProviderCredential::ApiKey {
            api_key: "sk-123".into(),
        };
        let json = serde_json::to_string(&cred).unwrap();
        assert!(json.contains(r#""type":"api_key""#));

        let roundtrip: ProviderCredential = serde_json::from_str(&json).unwrap();
        match roundtrip {
            ProviderCredential::ApiKey { api_key } => assert_eq!(api_key, "sk-123"),
            _ => panic!("expected ApiKey"),
        }
    }

    #[test]
    fn oauth_serializes_with_tag() {
        let cred = ProviderCredential::OAuth(make_creds(Utc::now()));
        let json = serde_json::to_string(&cred).unwrap();
        assert!(json.contains(r#""type":"o_auth""#));

        let roundtrip: ProviderCredential = serde_json::from_str(&json).unwrap();
        match roundtrip {
            ProviderCredential::OAuth(c) => assert_eq!(c.access_token, "access"),
            _ => panic!("expected OAuth"),
        }
    }

    #[test]
    fn credential_entry_api_key() {
        let entry = CredentialEntry::api_key("personal", "sk-123");
        assert_eq!(entry.id, "personal");
        assert!(entry.enabled);
        assert!(entry.health.is_healthy());
        match &entry.credential {
            ProviderCredential::ApiKey { api_key } => assert_eq!(api_key, "sk-123"),
            _ => panic!("expected ApiKey"),
        }
    }

    #[test]
    fn credential_entry_oauth() {
        let mut creds = make_creds(Utc::now() + Duration::seconds(300));
        creds.account_id = Some("acct-1".into());
        let entry = CredentialEntry::oauth(creds);
        assert_eq!(entry.id, "acct-1");
        assert!(entry.enabled);
    }

    #[test]
    fn credential_entry_oauth_auto_id() {
        let creds = make_creds(Utc::now() + Duration::seconds(300));
        assert!(creds.account_id.is_none());
        let entry = CredentialEntry::oauth(creds);
        assert!(!entry.id.is_empty());
    }

    #[test]
    fn health_default_is_healthy() {
        let h = CredentialHealth::default();
        assert!(h.is_healthy());
        assert_eq!(h.consecutive_errors, 0);
    }

    #[test]
    fn health_record_ok_resets_errors() {
        let mut h = CredentialHealth::default();
        h.record_error("fail", Some("429".into()));
        assert_eq!(h.consecutive_errors, 1);
        h.record_ok();
        assert_eq!(h.consecutive_errors, 0);
        assert!(h.last_ok.is_some());
        assert!(h.last_error.is_none());
    }

    #[test]
    fn health_record_error_increments() {
        let mut h = CredentialHealth::default();
        h.record_error("first", None);
        assert_eq!(h.consecutive_errors, 1);
        h.record_error("second", Some("500".into()));
        assert_eq!(h.consecutive_errors, 2);
        assert_eq!(h.last_error.as_ref().unwrap().message, "second");
        assert_eq!(h.last_error.as_ref().unwrap().code.as_deref(), Some("500"));
    }

    #[test]
    fn credential_entry_serde_roundtrip() {
        let entry = CredentialEntry::api_key("test", "sk-abc");
        let json = serde_json::to_string(&entry).unwrap();
        let roundtrip: CredentialEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip.id, "test");
        assert!(roundtrip.enabled);
    }
}
