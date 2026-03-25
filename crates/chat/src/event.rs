use std::pin::Pin;

use chrono::{DateTime, Utc};
use futures::Stream;
use serde::{Deserialize, Serialize};

use crate::{Error, InboundMessage, MessageRef, Participant};

/// Normalized inbound event stream emitted by a chat adapter.
pub type ChatEventStream = Pin<Box<dyn Stream<Item = Result<ChatEvent, Error>> + Send>>;

/// Normalized inbound events emitted by concrete chat adapters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatEvent {
    /// The adapter has an active connection or receive loop.
    Connected {
        /// Adapter identifier.
        adapter: String,
    },
    /// The adapter disconnected or stopped receiving events.
    Disconnected {
        /// Adapter identifier.
        adapter: String,
        /// Optional reason for the disconnect.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    /// A new inbound message was received.
    MessageReceived {
        /// Normalized inbound message.
        message: InboundMessage,
    },
    /// An existing inbound message was edited.
    MessageEdited {
        /// Normalized inbound message snapshot after the edit.
        message: InboundMessage,
    },
    /// A message was deleted on the provider surface.
    MessageDeleted {
        /// Deleted message reference.
        message: MessageRef,
        /// Deletion timestamp.
        deleted_at: DateTime<Utc>,
    },
    /// A reaction was added to an existing message.
    ReactionAdded {
        /// Message reference receiving the reaction.
        message: MessageRef,
        /// Participant that added the reaction.
        sender: Participant,
        /// Emoji or provider-specific reaction code.
        emoji: String,
        /// Reaction timestamp.
        added_at: DateTime<Utc>,
    },
}
