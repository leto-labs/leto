use agent_runtime::RuntimeError;
use futures::future::BoxFuture;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::registry::TypedTool;
use crate::truncation::truncate_middle_with_notice;

const MAX_OUTPUT_BYTES: usize = 16_000;

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
    ) -> BoxFuture<'_, Result<String, RuntimeError>>;
}

/// Request payload for the `shell` tool.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ShellRequest {
    /// The shell command to execute.
    pub command: String,
    /// Working directory for the command.
    pub working_directory: Option<String>,
}

/// Combined stdout/stderr output returned by `shell`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
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
    ) -> BoxFuture<'a, Result<Self::Response, RuntimeError>> {
        Box::pin(async move {
            let output = self
                .driver
                .run_command(&request.command, request.working_directory.as_deref())
                .await?;
            Ok(ShellResponse(output))
        })
    }
}

/// Native shell driver using `sh -c`.
pub struct NativeShellDriver;

impl ShellDriver for NativeShellDriver {
    fn run_command(
        &self,
        command: &str,
        working_directory: Option<&str>,
    ) -> BoxFuture<'_, Result<String, RuntimeError>> {
        let command = command.to_owned();
        let working_directory = working_directory.map(ToOwned::to_owned);
        Box::pin(async move {
            let mut cmd = tokio::process::Command::new("sh");
            cmd.arg("-c").arg(&command);

            if let Some(dir) = working_directory.as_ref() {
                cmd.current_dir(dir);
            }

            let output = cmd
                .output()
                .await
                .map_err(|error| RuntimeError::Tool(format!("shell failed to start: {error}")))?;

            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            let mut result = String::new();
            if !stdout.is_empty() {
                result.push_str(&stdout);
            }
            if !stderr.is_empty() {
                if !result.is_empty() {
                    result.push('\n');
                }
                result.push_str("[stderr]\n");
                result.push_str(&stderr);
            }
            if !output.status.success() {
                let code = output.status.code().unwrap_or(-1);
                result.push_str(&format!("\n[exit code: {code}]"));
            }

            Ok(truncate_middle_with_notice(
                &result,
                MAX_OUTPUT_BYTES,
                "shell output",
            ))
        })
    }
}
