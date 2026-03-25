use agent_runtime::RuntimeError;
use futures::future::BoxFuture;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::registry::TypedTool;

/// Echoes back the supplied message. Useful for tests and simple sanity
/// checks.
pub struct EchoTool;

/// Request payload for the `echo` tool.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EchoRequest {
    /// The message to echo back verbatim.
    pub message: String,
}

/// Echoed message text.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(transparent)]
#[schemars(description = "Echoed message text.")]
pub struct EchoResponse(pub String);

impl TypedTool for EchoTool {
    type Request = EchoRequest;
    type Response = EchoResponse;

    fn name(&self) -> &'static str {
        "echo"
    }

    fn description(&self) -> &'static str {
        "Echoes back the input message. For testing."
    }

    fn execute_typed<'a>(
        &'a self,
        request: Self::Request,
    ) -> BoxFuture<'a, Result<Self::Response, RuntimeError>> {
        Box::pin(async move { Ok(EchoResponse(request.message)) })
    }
}
