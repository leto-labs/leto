//! Webhook event parsing helpers.
//!
//! Official reference:
//! - Overview: <https://platform.openai.com/docs/guides/webhooks>

use crate::Error;

/// Minimal typed representation of an inbound OpenAI webhook event.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct WebhookEvent {
    pub id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
    pub data: serde_json::Value,
}

/// Parses a webhook payload body into a typed [`WebhookEvent`].
///
/// # Errors
///
/// Returns [`Error::Json`] when the payload is not valid webhook JSON.
pub fn parse_webhook_event(payload: &[u8]) -> Result<WebhookEvent, Error> {
    serde_json::from_slice(payload).map_err(Error::Json)
}
