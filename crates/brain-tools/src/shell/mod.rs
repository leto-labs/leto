#[cfg(feature = "native")]
pub mod native;

use futures::future::BoxFuture;

use brain_types::{BrainError, Tool, ToolDef};

pub trait ShellDriver: Send + Sync {
    fn run_command(
        &self,
        command: &str,
        working_directory: Option<&str>,
    ) -> BoxFuture<'_, Result<String, BrainError>>;
}

pub struct ShellTool<T: ShellDriver> {
    driver: T,
}

impl<T: ShellDriver> ShellTool<T> {
    pub fn new(driver: T) -> Self {
        Self { driver }
    }
}

impl<T: ShellDriver> Tool for ShellTool<T> {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "shell".into(),
            description: "Execute a shell command and return its output.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell command to execute"
                    },
                    "working_directory": {
                        "type": "string",
                        "description": "Working directory for the command (optional)"
                    }
                },
                "required": ["command"]
            }),
        }
    }

    fn execute(&self, args: serde_json::Value) -> BoxFuture<'_, Result<String, BrainError>> {
        Box::pin(async move {
            let command = args
                .get("command")
                .and_then(|v| v.as_str())
                .ok_or_else(|| BrainError::ToolFailed {
                    tool: "shell".into(),
                    reason: "missing required parameter 'command'".into(),
                })?;
            let working_directory = args.get("working_directory").and_then(|v| v.as_str());

            tracing::info!(command, ?working_directory, "shell invoked");
            self.driver.run_command(command, working_directory).await
        })
    }
}
