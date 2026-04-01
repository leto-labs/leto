//! Shared tool SDK and standard registry/runtime-facing contracts.

mod echo;
mod error;
pub mod registry;
mod tool;

pub use echo::{EchoRequest, EchoResponse, EchoTool};
pub use error::ToolError;
pub use registry::{
    ErasedTool, JsonTool, JsonToolHandler, RegistryToolExecutor, ToolRegistrationError, TypedTool,
};
pub use tool::{
    ApprovalDecision, ApprovalRequest, ToolApproval, ToolCall, ToolExecutionResult, ToolExecutor,
};
