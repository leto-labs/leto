use serde::{Deserialize, Serialize};

use crate::config::TokenUsage;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum ChatChunk {
    Delta {
        content: String,
    },
    ToolCallDelta {
        id: String,
        name: String,
        arguments_delta: String,
    },
    ToolCall {
        id: String,
        name: String,
        arguments: serde_json::Value,
    },
    Done {
        usage: Option<TokenUsage>,
    },
}
