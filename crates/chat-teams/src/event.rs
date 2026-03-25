use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use chat::{Attachment, ChatEvent, ConversationRef, InboundMessage, MessageRef, Participant};

/// Teams attachment metadata preserved during normalization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamsAttachment {
    /// Optional Teams attachment identifier.
    pub id: Option<String>,
    /// Optional content type.
    pub content_type: Option<String>,
    /// Optional content URL.
    pub content_url: Option<String>,
    /// Optional display name.
    pub name: Option<String>,
    /// Optional size in bytes.
    pub size_bytes: Option<u64>,
}

/// Simplified Teams activity payload used by the adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamsActivity {
    /// Teams conversation identifier.
    pub conversation_id: String,
    /// Teams activity identifier.
    pub activity_id: String,
    /// Sender identifier.
    pub from_id: String,
    /// Optional sender display name.
    pub from_name: Option<String>,
    /// Activity text.
    pub text: String,
    /// Optional replied-to activity identifier.
    pub reply_to_id: Option<String>,
    /// Whether the activity represents an edit.
    pub edited: bool,
    /// Attachment metadata.
    pub attachments: Vec<TeamsAttachment>,
    /// Timestamp associated with the activity.
    pub sent_at: DateTime<Utc>,
}

impl From<TeamsActivity> for ChatEvent {
    fn from(activity: TeamsActivity) -> Self {
        let conversation = ConversationRef::new("teams", activity.conversation_id);
        let reply_to = activity
            .reply_to_id
            .map(|id| MessageRef::new(conversation.clone(), id));
        let sender = activity
            .from_name
            .as_ref()
            .map(|name| Participant::new(activity.from_id.clone()).with_display_name(name))
            .unwrap_or_else(|| Participant::new(activity.from_id.clone()));
        let attachments = activity
            .attachments
            .into_iter()
            .map(|attachment| Attachment {
                id: attachment.id,
                filename: attachment.name,
                content_type: attachment.content_type,
                url: attachment.content_url,
                size_bytes: attachment.size_bytes,
                metadata: Default::default(),
            })
            .collect();
        let message = InboundMessage {
            message: MessageRef::new(conversation, activity.activity_id),
            sender,
            text: activity.text,
            attachments,
            sent_at: activity.sent_at,
            reply_to,
            metadata: Default::default(),
        };

        if activity.edited {
            ChatEvent::MessageEdited { message }
        } else {
            ChatEvent::MessageReceived { message }
        }
    }
}
