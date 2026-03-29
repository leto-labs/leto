use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use brain_types::{Event, ProviderInfo};

/// Lightweight liveness payload for the legacy `brain-server` HTTP surface.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthResponse {
    pub status: String,
}

impl HealthResponse {
    /// Builds the current healthy response payload.
    #[must_use]
    pub fn current() -> Self {
        Self {
            status: "ok".to_owned(),
        }
    }
}

/// Runtime event enriched with server-side session context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerEvent {
    pub session_id: Ulid,
    pub event: Event,
    pub timestamp: DateTime<Utc>,
}

impl ServerEvent {
    pub fn new(session_id: Ulid, event: Event) -> Self {
        Self {
            session_id,
            event,
            timestamp: Utc::now(),
        }
    }
}

/// Summary of the legacy server's current runtime state.
#[derive(Debug, Clone, Serialize)]
pub struct ServerStatus {
    pub providers: Vec<ProviderInfo>,
    pub tools: Vec<String>,
    pub active_sessions: usize,
    pub active_turns: Vec<Ulid>,
}

/// Request body for creating a project over HTTP.
#[derive(Debug, Deserialize)]
pub(crate) struct CreateProjectRequest {
    pub name: Option<String>,
    pub root: Option<std::path::PathBuf>,
}

/// Request body for starting a new turn from plain text content.
#[derive(Debug, Deserialize)]
pub(crate) struct SendMessageRequest {
    pub content: String,
}
