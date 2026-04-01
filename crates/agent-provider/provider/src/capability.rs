//! Capability metadata for shared providers and models.

use serde::{Deserialize, Serialize};

/// The finest-grained stream shape a provider can emit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamGranularity {
    /// Only terminal or fully-buffered results are available.
    Final,
    /// Incremental deltas are available but without explicit block boundaries.
    Delta,
    /// The provider preserves block lifecycle events and incremental deltas.
    Block,
}

/// Describes which shared SDK features a provider or model supports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    /// Whether top-level system messages are accepted.
    pub system_messages: bool,
    /// Whether provider-specific developer messages are accepted distinctly from
    /// system messages.
    pub developer_messages: bool,
    /// Whether plain text input is accepted.
    pub input_text: bool,
    /// Whether image URLs or equivalent remote image references are accepted.
    pub input_image_urls: bool,
    /// Whether the model can emit tool-call blocks.
    pub tool_calls: bool,
    /// Whether tool-result blocks can be sent back to the model.
    pub tool_results: bool,
    /// Whether explicit reasoning or thinking blocks can be represented.
    pub reasoning_blocks: bool,
    /// Whether refusal blocks can be represented distinctly from normal text.
    pub refusal_blocks: bool,
    /// Whether tool-call arguments can be streamed incrementally.
    pub tool_call_argument_deltas: bool,
    /// Whether the provider can request multiple tools in parallel.
    pub parallel_tool_calls: bool,
    /// The most detailed stream shape available from the provider.
    pub stream_granularity: StreamGranularity,
}

impl ProviderCapabilities {
    /// Returns a minimal text-only capability set suitable for local or mock
    /// providers.
    pub const fn text_only() -> Self {
        Self {
            system_messages: true,
            developer_messages: false,
            input_text: true,
            input_image_urls: false,
            tool_calls: false,
            tool_results: false,
            reasoning_blocks: false,
            refusal_blocks: false,
            tool_call_argument_deltas: false,
            parallel_tool_calls: false,
            stream_granularity: StreamGranularity::Block,
        }
    }
}
