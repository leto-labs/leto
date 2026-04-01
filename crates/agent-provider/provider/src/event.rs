//! Shared streamed event types emitted by [`crate::Provider`] implementations.

use serde::{Deserialize, Serialize};

use crate::Usage;

/// Identifies the semantic kind of an output block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BlockKind {
    /// Visible assistant text.
    Text,
    /// Tool invocation emitted by the model.
    ToolCall {
        /// Optional tool name when known at block start time.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        /// Optional stable call identifier when the provider supplies one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        call_id: Option<String>,
    },
    /// Reasoning or thinking content.
    Reasoning,
    /// Refusal content distinct from normal text.
    Refusal,
    /// Provider-specific block kind that does not map cleanly to the shared
    /// vocabulary.
    Unknown {
        /// Provider-specific kind label.
        kind: String,
    },
}

/// Metadata describing a block that has started streaming.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    /// Stable block identifier within the stream.
    pub id: String,
    /// Output index assigned by the provider when available.
    pub output_index: u32,
    /// Semantic block kind.
    pub kind: BlockKind,
    /// Optional provider-specific item identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_id: Option<String>,
}

/// Incremental content emitted for a previously-started block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BlockDelta {
    /// Incremental visible text.
    Text { text: String },
    /// Incremental JSON payload, typically for tool-call arguments.
    Json { partial_json: String },
    /// Incremental reasoning text.
    Reasoning { text: String },
    /// Incremental refusal text.
    Refusal { text: String },
    /// Incremental signature or checksum data.
    Signature { signature: String },
    /// Provider-specific raw payload.
    Unknown { raw: serde_json::Value },
}

/// Shared completion reason used by the provider SDK.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    /// The provider stopped normally.
    Stop,
    /// Generation stopped because the output budget was exhausted.
    MaxTokens,
    /// Generation stopped because a stop sequence matched.
    StopSequence,
    /// The model finished by requesting tool execution.
    ToolCall,
    /// The model paused and expects a follow-up request or tool continuation.
    Pause,
    /// The model refused to continue.
    Refusal,
    /// The response ended without a normal terminal condition.
    Incomplete,
    /// The response terminated because of an error.
    Error,
    /// Provider-specific completion reason.
    Unknown(String),
}

/// Shared stream event emitted by [`crate::Provider::stream`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    /// Marks the beginning of a streamed response.
    ResponseStart {
        /// Optional provider response identifier.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        response_id: Option<String>,
        /// Optional model identifier actually used by the provider.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        model: Option<String>,
    },
    /// Signals that a block has started.
    BlockStart {
        /// Block metadata.
        block: Block,
    },
    /// Emits an incremental delta for an active block.
    BlockDelta {
        /// Identifier of the block receiving the delta.
        id: String,
        /// Delta payload.
        delta: BlockDelta,
    },
    /// Signals that a block has finished.
    BlockStop {
        /// Identifier of the completed block.
        id: String,
    },
    /// Emits usage information when the provider makes it available.
    Usage {
        /// Token-usage payload.
        usage: Usage,
    },
    /// Signals overall stream completion.
    Completed {
        /// Optional provider response identifier.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        response_id: Option<String>,
        /// Optional completion reason.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        finish_reason: Option<FinishReason>,
    },
}

impl Event {
    /// Convenience constructor for text deltas.
    pub fn text_delta(id: impl Into<String>, text: impl Into<String>) -> Self {
        Self::BlockDelta {
            id: id.into(),
            delta: BlockDelta::Text { text: text.into() },
        }
    }
}
