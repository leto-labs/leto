use agent_tool::ToolError;
use futures::future::BoxFuture;

use super::{MAX_OUTPUT_BYTES, ShellDriver};

/// Native shell driver using `sh -c`.
pub struct NativeShellDriver;

impl ShellDriver for NativeShellDriver {
    fn run_command(
        &self,
        command: &str,
        working_directory: Option<&str>,
    ) -> BoxFuture<'_, Result<String, ToolError>> {
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
                .map_err(|error| ToolError::new(format!("shell failed to start: {error}")))?;

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

            Ok(crate::truncation::truncate_middle_with_notice(
                &result,
                MAX_OUTPUT_BYTES,
                "shell output",
            ))
        })
    }
}
