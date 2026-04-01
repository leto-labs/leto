use futures::future::BoxFuture;
use provider::ToolDefinition;
use serde::{Deserialize, Serialize};

use crate::ToolError;

/// Structured tool invocation requested by the model/runtime.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    /// Stable provider/runtime call identifier.
    pub id: String,
    /// Stable tool name.
    pub name: String,
    /// Structured JSON input payload.
    pub input: serde_json::Value,
}

/// Structured result returned by a tool executor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolExecutionResult {
    /// JSON payload returned by the tool.
    pub output: serde_json::Value,
    /// Whether the output should be interpreted as an error result.
    pub is_error: bool,
}

impl ToolExecutionResult {
    /// Builds a successful tool result.
    pub fn success(output: serde_json::Value) -> Self {
        Self {
            output,
            is_error: false,
        }
    }

    /// Builds a failed tool result.
    pub fn error(output: serde_json::Value) -> Self {
        Self {
            output,
            is_error: true,
        }
    }
}

/// Optional approval metadata requested before executing a tool call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolApproval {
    /// Human-readable reason explaining why approval is required.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Runtime-owned approval request covering one or more tool calls.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApprovalRequest {
    /// Stable request identifier used to resolve approval later.
    pub id: String,
    /// Tool calls waiting for approval.
    pub calls: Vec<ToolCall>,
    /// Optional reason surfaced to the caller.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Decision resolving a pending approval request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ApprovalDecision {
    /// Allow the pending tool call batch to proceed.
    Allow { request_id: String },
    /// Deny the pending tool call batch.
    Deny {
        request_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
}

/// Shared trait for exposing tools to the runtime and executing them.
pub trait ToolExecutor: Send + Sync {
    /// Returns provider-visible tool definitions for the current runtime.
    fn definitions(&self) -> Vec<ToolDefinition>;

    /// Returns approval metadata when a tool call must be gated.
    fn approval(&self, _call: &ToolCall) -> Option<ToolApproval> {
        None
    }

    /// Executes a requested tool call.
    fn execute<'a>(
        &'a self,
        call: ToolCall,
    ) -> BoxFuture<'a, Result<ToolExecutionResult, ToolError>>;
}
