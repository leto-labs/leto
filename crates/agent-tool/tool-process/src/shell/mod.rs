use agent_tool::{ToolError, TypedTool};
use futures::future::BoxFuture;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[cfg(feature = "component")]
pub(crate) mod component;
#[cfg(feature = "native")]
pub mod native;

/// Executes a shell command and returns its combined output.
pub struct ShellTool<D: ShellDriver> {
    driver: D,
}

impl<D: ShellDriver> ShellTool<D> {
    /// Creates a shell tool backed by the provided driver.
    pub fn new(driver: D) -> Self {
        Self { driver }
    }
}

/// Semantic driver for shell command execution.
pub trait ShellDriver: Send + Sync + 'static {
    /// Runs a shell command with an optional working directory.
    fn run_command(
        &self,
        command: &str,
        working_directory: Option<&str>,
    ) -> BoxFuture<'_, Result<String, ToolError>>;
}

/// Request payload for the `shell` tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ShellRequest {
    /// The shell command to execute.
    pub command: String,
    /// Working directory for the command.
    pub working_directory: Option<String>,
}

/// Combined stdout/stderr output returned by `shell`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
#[schemars(
    description = "Combined shell output, including stderr and exit-code notices when present."
)]
pub struct ShellResponse(pub String);

impl<D: ShellDriver> TypedTool for ShellTool<D> {
    type Request = ShellRequest;
    type Response = ShellResponse;

    fn name(&self) -> &'static str {
        "shell"
    }

    fn description(&self) -> &'static str {
        "Execute a shell command and return its output."
    }

    fn execute_typed<'a>(
        &'a self,
        request: Self::Request,
    ) -> BoxFuture<'a, Result<Self::Response, ToolError>> {
        Box::pin(async move {
            let output = self
                .driver
                .run_command(&request.command, request.working_directory.as_deref())
                .await?;
            Ok(ShellResponse(output))
        })
    }
}

#[cfg(feature = "native")]
pub(crate) const MAX_OUTPUT_BYTES: usize = 16_000;
