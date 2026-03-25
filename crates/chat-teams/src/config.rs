use serde::{Deserialize, Serialize};

/// Microsoft Teams-specific adapter configuration.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TeamsConfig {
    /// Bot token or bearer token.
    pub bot_token: Option<String>,
    /// Azure or Teams application identifier.
    pub app_id: Option<String>,
}
