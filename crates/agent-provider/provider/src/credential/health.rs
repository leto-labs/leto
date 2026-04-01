use std::time::{SystemTime, UNIX_EPOCH};

/// Failure details recorded against one credential.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialFailure {
    /// Human-readable error summary.
    pub message: String,
    /// Optional provider-specific error code.
    pub code: Option<String>,
}

impl CredentialFailure {
    /// Creates a new failure record without provider-specific code.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: None,
        }
    }

    /// Attaches a provider-specific error code.
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }
}

/// Persistable metadata describing the most recent credential failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialFailureRecord {
    /// Human-readable error summary.
    pub message: String,
    /// Optional provider-specific error code.
    pub code: Option<String>,
    /// Millisecond timestamp recorded when the failure occurred.
    pub recorded_at_ms: u128,
}

/// In-memory health summary tracked by the shared credential pool.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CredentialHealth {
    /// Millisecond timestamp of the most recent successful use.
    pub last_ok_ms: Option<u128>,
    /// Most recent failure, if any.
    pub last_error: Option<CredentialFailureRecord>,
    /// Consecutive failure count since the last successful use.
    pub consecutive_errors: u32,
}

impl CredentialHealth {
    /// Returns true when the credential is considered healthy for selection.
    pub fn is_healthy(&self) -> bool {
        self.consecutive_errors == 0
    }

    /// Records a successful credential use and clears failure state.
    pub fn record_ok(&mut self) {
        self.last_ok_ms = Some(now_ms());
        self.last_error = None;
        self.consecutive_errors = 0;
    }

    /// Records a failed credential use.
    pub fn record_error(&mut self, failure: CredentialFailure) {
        self.consecutive_errors = self.consecutive_errors.saturating_add(1);
        self.last_error = Some(CredentialFailureRecord {
            message: failure.message,
            code: failure.code,
            recorded_at_ms: now_ms(),
        });
    }
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
