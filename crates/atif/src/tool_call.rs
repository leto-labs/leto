use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::JsonObject;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolCall {
    pub tool_call_id: String,
    pub function_name: String,
    pub arguments: JsonObject,
}

impl ToolCall {
    pub fn from_value_object(
        tool_call_id: String,
        function_name: String,
        arguments: Value,
    ) -> Option<Self> {
        match arguments {
            Value::Object(arguments) => Some(Self {
                tool_call_id,
                function_name,
                arguments,
            }),
            _ => None,
        }
    }
}
