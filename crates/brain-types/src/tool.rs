use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};

use crate::BrainError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

pub trait Tool: Send + Sync {
    fn definition(&self) -> ToolDef;
    fn execute(&self, args: serde_json::Value) -> BoxFuture<'_, Result<String, BrainError>>;
}
