use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use chat::{Attachment, ChatEvent, ConversationRef, InboundMessage, MessageRef, Participant};

/// Slack file metadata preserved during normalization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlackFile {
    /// Slack file identifier.
    pub id: String,
    /// Optional original filename.
    pub filename: Option<String>,
    /// Optional MIME type.
    pub content_type: Option<String>,
    /// Optional permalink or fetch hint.
    pub url: Option<String>,
    /// Optional size in bytes.
    pub size_bytes: Option<u64>,
}

/// Simplified Slack message event used by the adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlackEvent {
    /// Slack channel identifier.
    pub channel_id: String,
    /// Slack message timestamp identifier.
    pub ts: String,
    /// Slack sender identifier.
    pub user_id: String,
    /// Message text.
    pub text: String,
    /// Optional display name.
    pub username: Option<String>,
    /// Optional parent thread timestamp.
    pub thread_ts: Option<String>,
    /// Whether the message represents an edit.
    pub edited: bool,
    /// File metadata attached to the message.
    pub files: Vec<SlackFile>,
    /// Timestamp associated with the event.
    pub sent_at: DateTime<Utc>,
}

impl From<SlackEvent> for ChatEvent {
    fn from(event: SlackEvent) -> Self {
        let mut conversation = ConversationRef::new("slack", event.channel_id);
        if let Some(thread_ts) = event.thread_ts.clone() {
            conversation = conversation.with_thread(thread_ts);
        }

        let sender = event
            .username
            .as_ref()
            .map(|username| Participant::new(event.user_id.clone()).with_display_name(username))
            .unwrap_or_else(|| Participant::new(event.user_id.clone()));
        let attachments = event
            .files
            .into_iter()
            .map(|file| Attachment {
                id: Some(file.id),
                filename: file.filename,
                content_type: file.content_type,
                url: file.url,
                size_bytes: file.size_bytes,
                metadata: Default::default(),
            })
            .collect();
        let message = InboundMessage {
            message: MessageRef::new(conversation, event.ts),
            sender,
            text: event.text,
            attachments,
            sent_at: event.sent_at,
            reply_to: None,
            metadata: Default::default(),
        };

        if event.edited {
            ChatEvent::MessageEdited { message }
        } else {
            ChatEvent::MessageReceived { message }
        }
    }
}
