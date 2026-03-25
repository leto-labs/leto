use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use chat::{Attachment, ChatEvent, ConversationRef, InboundMessage, MessageRef, Participant};

/// Telegram attachment metadata preserved during normalization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TelegramAttachment {
    /// Telegram file identifier.
    pub file_id: String,
    /// Optional filename.
    pub filename: Option<String>,
    /// Optional MIME type.
    pub content_type: Option<String>,
    /// Optional remote file URL or fetch hint.
    pub url: Option<String>,
    /// Optional size in bytes.
    pub size_bytes: Option<u64>,
}

/// Simplified Telegram update payload used by the adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TelegramUpdate {
    /// Telegram chat identifier.
    pub chat_id: i64,
    /// Telegram message identifier.
    pub message_id: i64,
    /// Telegram sender identifier.
    pub sender_id: i64,
    /// Optional Telegram username.
    pub sender_username: Option<String>,
    /// Message body text.
    pub text: String,
    /// Optional forum topic / thread identifier.
    pub thread_id: Option<i64>,
    /// Whether this update reflects an edit to an existing message.
    pub edited: bool,
    /// Attachment metadata.
    pub attachments: Vec<TelegramAttachment>,
    /// Timestamp associated with the update.
    pub sent_at: DateTime<Utc>,
    /// Optional replied-to message identifier.
    pub reply_to_message_id: Option<i64>,
}

impl From<TelegramUpdate> for ChatEvent {
    fn from(update: TelegramUpdate) -> Self {
        let mut conversation = ConversationRef::new("telegram", update.chat_id.to_string());
        if let Some(thread_id) = update.thread_id {
            conversation = conversation.with_thread(thread_id.to_string());
        }

        let reply_to = update
            .reply_to_message_id
            .map(|message_id| MessageRef::new(conversation.clone(), message_id.to_string()));
        let sender = update
            .sender_username
            .as_ref()
            .map(|username| {
                Participant::new(update.sender_id.to_string()).with_display_name(username)
            })
            .unwrap_or_else(|| Participant::new(update.sender_id.to_string()));
        let attachments = update
            .attachments
            .into_iter()
            .map(|attachment| Attachment {
                id: Some(attachment.file_id),
                filename: attachment.filename,
                content_type: attachment.content_type,
                url: attachment.url,
                size_bytes: attachment.size_bytes,
                metadata: Default::default(),
            })
            .collect();
        let message = InboundMessage {
            message: MessageRef::new(conversation, update.message_id.to_string()),
            sender,
            text: update.text,
            attachments,
            sent_at: update.sent_at,
            reply_to,
            metadata: Default::default(),
        };

        if update.edited {
            ChatEvent::MessageEdited { message }
        } else {
            ChatEvent::MessageReceived { message }
        }
    }
}
