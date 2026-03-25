use serde::{Deserialize, Serialize};

/// Telegram-specific adapter configuration.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TelegramConfig {
    /// Bot token used for authenticated Telegram Bot API access.
    pub bot_token: Option<String>,
    /// Optional webhook secret used by higher-level integrations.
    pub webhook_secret: Option<String>,
}
