use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Free-form metadata used by normalized chat types.
pub type Metadata = BTreeMap<String, String>;

/// Stable platform-scoped conversation reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationRef {
    /// Stable adapter identifier such as `telegram` or `slack`.
    pub adapter: String,
    /// Platform-scoped conversation or room identifier.
    pub conversation_id: String,
    /// Optional thread identifier when the platform distinguishes threads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
    /// Free-form platform metadata preserved during normalization.
    #[serde(default, skip_serializing_if = "Metadata::is_empty")]
    pub metadata: Metadata,
}

impl ConversationRef {
    /// Creates a new conversation reference.
    pub fn new(adapter: impl Into<String>, conversation_id: impl Into<String>) -> Self {
        Self {
            adapter: adapter.into(),
            conversation_id: conversation_id.into(),
            thread_id: None,
            metadata: Metadata::new(),
        }
    }

    /// Attaches a thread identifier to the conversation reference.
    pub fn with_thread(mut self, thread_id: impl Into<String>) -> Self {
        self.thread_id = Some(thread_id.into());
        self
    }
}

/// Stable platform-scoped message reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageRef {
    /// Parent conversation for the message.
    pub conversation: ConversationRef,
    /// Platform-scoped message identifier.
    pub message_id: String,
}

impl MessageRef {
    /// Creates a new message reference.
    pub fn new(conversation: ConversationRef, message_id: impl Into<String>) -> Self {
        Self {
            conversation,
            message_id: message_id.into(),
        }
    }
}

/// Identity for a human or bot participant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Participant {
    /// Platform-scoped participant identifier.
    pub id: String,
    /// Optional display name or username.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Whether this participant is a bot account.
    #[serde(default)]
    pub is_bot: bool,
    /// Free-form provider metadata preserved during normalization.
    #[serde(default, skip_serializing_if = "Metadata::is_empty")]
    pub metadata: Metadata,
}

impl Participant {
    /// Creates a participant with the required identifier only.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            display_name: None,
            is_bot: false,
            metadata: Metadata::new(),
        }
    }

    /// Sets the participant display name.
    pub fn with_display_name(mut self, display_name: impl Into<String>) -> Self {
        self.display_name = Some(display_name.into());
        self
    }
}

/// Attachment or media metadata carried with a message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attachment {
    /// Optional platform-scoped attachment identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Optional original filename.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    /// Optional MIME type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    /// Optional remote URL or fetch hint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Optional attachment size in bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    /// Free-form provider metadata preserved during normalization.
    #[serde(default, skip_serializing_if = "Metadata::is_empty")]
    pub metadata: Metadata,
}

/// Normalized inbound chat message payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboundMessage {
    /// Message identity.
    pub message: MessageRef,
    /// Sender identity.
    pub sender: Participant,
    /// Plain text content.
    pub text: String,
    /// Attachments or media metadata.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<Attachment>,
    /// Timestamp assigned by the provider or adapter.
    pub sent_at: DateTime<Utc>,
    /// Optional parent message reference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<MessageRef>,
    /// Free-form provider metadata preserved during normalization.
    #[serde(default, skip_serializing_if = "Metadata::is_empty")]
    pub metadata: Metadata,
}

impl InboundMessage {
    /// Creates a new normalized inbound message using the current time.
    pub fn new(message: MessageRef, sender: Participant, text: impl Into<String>) -> Self {
        Self {
            message,
            sender,
            text: text.into(),
            attachments: Vec::new(),
            sent_at: Utc::now(),
            reply_to: None,
            metadata: Metadata::new(),
        }
    }
}

/// Normalized outbound send payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboundMessage {
    /// Destination conversation.
    pub conversation: ConversationRef,
    /// Plain text content to send.
    pub text: String,
    /// Attachments or media metadata.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<Attachment>,
    /// Optional parent message reference for replies.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<MessageRef>,
    /// Free-form provider metadata preserved during normalization.
    #[serde(default, skip_serializing_if = "Metadata::is_empty")]
    pub metadata: Metadata,
}

impl OutboundMessage {
    /// Creates a new outbound message.
    pub fn new(conversation: ConversationRef, text: impl Into<String>) -> Self {
        Self {
            conversation,
            text: text.into(),
            attachments: Vec::new(),
            reply_to: None,
            metadata: Metadata::new(),
        }
    }
}

/// Normalized edit payload for an existing message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboundEdit {
    /// Message identity to update.
    pub message: MessageRef,
    /// New message text.
    pub text: String,
    /// Replacement attachment metadata when the provider supports it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<Attachment>,
    /// Free-form provider metadata preserved during normalization.
    #[serde(default, skip_serializing_if = "Metadata::is_empty")]
    pub metadata: Metadata,
}

impl OutboundEdit {
    /// Creates a new outbound edit request.
    pub fn new(message: MessageRef, text: impl Into<String>) -> Self {
        Self {
            message,
            text: text.into(),
            attachments: Vec::new(),
            metadata: Metadata::new(),
        }
    }
}

/// Receipt returned after a successful outbound send or edit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliveryReceipt {
    /// Message identity acknowledged by the adapter.
    pub message: MessageRef,
    /// Delivery timestamp assigned by the adapter.
    pub delivered_at: DateTime<Utc>,
    /// Free-form provider metadata preserved during normalization.
    #[serde(default, skip_serializing_if = "Metadata::is_empty")]
    pub metadata: Metadata,
}

impl DeliveryReceipt {
    /// Creates a new delivery receipt using the current time.
    pub fn new(message: MessageRef) -> Self {
        Self {
            message,
            delivered_at: Utc::now(),
            metadata: Metadata::new(),
        }
    }
}
