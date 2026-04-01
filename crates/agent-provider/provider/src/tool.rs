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
    /// Optional JSON Schema describing the tool's output payload.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<serde_json::Value>,
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
            output_schema: None,
        }
    }

    /// Adds an output schema to the tool definition.
    pub fn with_output_schema(mut self, output_schema: serde_json::Value) -> Self {
        self.output_schema = Some(output_schema);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::ToolDefinition;

    #[test]
    fn serializes_optional_output_schema() {
        let definition =
            ToolDefinition::new("echo", "Echo text", serde_json::json!({ "type": "object" }))
                .with_output_schema(serde_json::json!({ "type": "string" }));

        let value = serde_json::to_value(&definition).expect("serialize tool definition");
        assert_eq!(value["output_schema"]["type"], "string");
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
