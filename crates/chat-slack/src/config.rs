use serde::{Deserialize, Serialize};

/// Slack-specific adapter configuration.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SlackConfig {
    /// Slack bot token.
    pub bot_token: Option<String>,
    /// Slack signing secret used by higher-level webhook integrations.
    pub signing_secret: Option<String>,
}
