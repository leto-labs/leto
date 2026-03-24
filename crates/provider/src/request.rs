//! Shared request and transcript types for provider-neutral inference.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{ToolChoice, ToolDefinition};

/// Role carried by a message in the shared transcript model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    /// High-priority system instruction.
    System,
    /// Provider-specific developer instruction distinct from the system role.
    Developer,
    /// End-user input.
    User,
    /// Model-authored assistant content.
    Assistant,
}

/// Shared content block carried inside a transcript message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Plain UTF-8 text.
    Text {
        /// Visible text content.
        text: String,
    },
    /// Remote image reference.
    ImageUrl {
        /// Image URL or data URL.
        url: String,
    },
    /// Tool invocation emitted by the model or replayed from history.
    ToolCall {
        /// Provider- or caller-assigned tool-call identifier.
        id: String,
        /// Tool name.
        name: String,
        /// Structured tool input payload.
        input: serde_json::Value,
    },
    /// Result of executing a prior tool call.
    ToolResult {
        /// Identifier of the tool call this result belongs to.
        call_id: String,
        /// Structured tool output payload.
        output: serde_json::Value,
        /// Optional error flag for providers that distinguish failing tool
        /// results.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
    /// Reasoning or thinking text that should remain structurally distinct.
    Reasoning {
        /// Reasoning text.
        text: String,
    },
    /// Refusal text returned by the provider.
    Refusal {
        /// Refusal text.
        text: String,
    },
}

impl ContentBlock {
    /// Creates a plain text content block.
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }

    /// Creates an image URL content block.
    pub fn image_url(url: impl Into<String>) -> Self {
        Self::ImageUrl { url: url.into() }
    }

    /// Creates a tool-call content block.
    pub fn tool_call(
        id: impl Into<String>,
        name: impl Into<String>,
        input: serde_json::Value,
    ) -> Self {
        Self::ToolCall {
            id: id.into(),
            name: name.into(),
            input,
        }
    }

    /// Creates a tool-result content block.
    pub fn tool_result(call_id: impl Into<String>, output: serde_json::Value) -> Self {
        Self::ToolResult {
            call_id: call_id.into(),
            output,
            is_error: None,
        }
    }
}

/// A single message in the shared transcript model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    /// Message role.
    pub role: MessageRole,
    /// Ordered content blocks carried by the message.
    pub content: Vec<ContentBlock>,
}

impl Message {
    /// Creates a message from a role and a list of content blocks.
    pub fn new(role: MessageRole, content: Vec<ContentBlock>) -> Self {
        Self { role, content }
    }

    /// Creates a system message containing a single text block.
    pub fn system_text(text: impl Into<String>) -> Self {
        Self::new(MessageRole::System, vec![ContentBlock::text(text)])
    }

    /// Creates a developer message containing a single text block.
    pub fn developer_text(text: impl Into<String>) -> Self {
        Self::new(MessageRole::Developer, vec![ContentBlock::text(text)])
    }

    /// Creates a user message containing a single text block.
    pub fn user_text(text: impl Into<String>) -> Self {
        Self::new(MessageRole::User, vec![ContentBlock::text(text)])
    }

    /// Creates an assistant message containing a single text block.
    pub fn assistant_text(text: impl Into<String>) -> Self {
        Self::new(MessageRole::Assistant, vec![ContentBlock::text(text)])
    }

    /// Returns a lossy text-only representation of the message content.
    pub fn plain_text_lossy(&self) -> String {
        self.content
            .iter()
            .filter_map(|block| match block {
                ContentBlock::Text { text }
                | ContentBlock::Reasoning { text }
                | ContentBlock::Refusal { text } => Some(text.as_str()),
                ContentBlock::ImageUrl { .. }
                | ContentBlock::ToolCall { .. }
                | ContentBlock::ToolResult { .. } => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Shared reasoning configuration used by providers that expose explicit
/// thinking controls.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ReasoningConfig {
    /// Provider-specific effort preset such as `low`, `medium`, or `high`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    /// Optional summary display mode or reasoning summary hint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Optional reasoning budget in tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget_tokens: Option<u32>,
}

/// Shared inference options carried alongside a request transcript.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RequestOptions {
    /// Maximum number of output tokens to generate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
    /// Sampling temperature.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// Nucleus sampling parameter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    /// Top-k sampling parameter for providers that support it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    /// Stop sequences that should end generation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stop_sequences: Vec<String>,
    /// Tool-selection policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    /// Whether parallel tool calls are allowed when the provider supports them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
    /// Optional reasoning controls.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<ReasoningConfig>,
    /// Provider-agnostic metadata forwarded to adapters that support it.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, serde_json::Value>,
}

/// Shared request object passed into [`crate::Provider::stream`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Request {
    /// Optional model identifier. Providers fall back to their defaults when
    /// this is omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Ordered transcript messages.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub messages: Vec<Message>,
    /// Tools exposed to the provider.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ToolDefinition>,
    /// Shared inference options.
    pub options: RequestOptions,
}

impl Request {
    /// Creates a request containing a single user text message.
    pub fn user_text(text: impl Into<String>) -> Self {
        Self {
            messages: vec![Message::user_text(text)],
            ..Self::default()
        }
    }

    /// Extracts the last user message as a lossy plain-text string.
    pub fn last_user_text_lossy(&self) -> Option<String> {
        self.messages.iter().rev().find_map(|message| {
            (message.role == MessageRole::User).then(|| message.plain_text_lossy())
        })
    }
}
