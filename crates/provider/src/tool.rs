//! Shared tool-definition primitives used by provider requests.

use serde::{Deserialize, Serialize};

/// A tool exposed to the model through the shared request surface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Stable tool name presented to the model.
    pub name: String,
    /// Optional human-readable description of the tool's behavior.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// JSON Schema describing the tool's input payload.
    pub input_schema: serde_json::Value,
}

impl ToolDefinition {
    /// Creates a custom tool definition from a name, description, and JSON
    /// Schema.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: serde_json::Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: Some(description.into()),
            input_schema,
        }
    }
}

/// Shared tool-selection policy requested by the caller.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolChoice {
    /// Let the provider decide whether to call a tool.
    Auto,
    /// Require at least one tool call before the model can finish normally.
    Required,
    /// Force a specific named tool.
    Tool { name: String },
    /// Disable tool use for this request.
    None,
}

impl ToolChoice {
    /// Forces a specific named tool.
    pub fn tool(name: impl Into<String>) -> Self {
        Self::Tool { name: name.into() }
    }
}
