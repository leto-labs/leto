use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use brain_types::{Event, ProviderInfo};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStatus {
    pub providers: Vec<ProviderInfo>,
    pub tools: Vec<String>,
    pub active_sessions: usize,
    pub active_turns: Vec<Ulid>,
}

#[derive(Debug, Deserialize)]
pub struct CreateProjectRequest {
    pub name: Option<String>,
    pub root: Option<std::path::PathBuf>,
}

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub content: String,
}
