use serde::{Deserialize, Serialize};

/// Optional features supported by a concrete chat adapter.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ChatCapabilities {
    /// Whether the adapter can receive inbound messages.
    pub receive_messages: bool,
    /// Whether the adapter can send outbound messages.
    pub send_messages: bool,
    /// Whether the adapter supports editing previously sent messages.
    pub message_edits: bool,
    /// Whether the adapter supports threaded replies.
    pub threaded_replies: bool,
    /// Whether the adapter supports typing indicators.
    pub typing_indicators: bool,
    /// Whether the adapter supports reactions.
    pub reactions: bool,
    /// Whether the adapter supports attachments or media metadata.
    pub attachments: bool,
    /// Whether the adapter can run through webhooks.
    pub webhook_mode: bool,
    /// Whether the adapter can run through polling.
    pub polling_mode: bool,
    /// Whether the adapter exposes a health check.
    pub health_checks: bool,
}
