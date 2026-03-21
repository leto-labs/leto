use futures::future::BoxFuture;

use brain_types::BrainError;

use super::ShellDriver;
use crate::truncation::truncate_middle_with_notice;

const MAX_OUTPUT_BYTES: usize = 16_000;

pub struct ShellDriverNative;

impl ShellDriver for ShellDriverNative {
    fn run_command(
        &self,
        command: &str,
        working_directory: Option<&str>,
    ) -> BoxFuture<'_, Result<String, BrainError>> {
        let command = command.to_owned();
        let working_directory = working_directory.map(|s| s.to_owned());
        Box::pin(async move {
            let mut cmd = tokio::process::Command::new("sh");
            cmd.arg("-c").arg(&command);

            if let Some(ref dir) = working_directory {
                cmd.current_dir(dir);
            }

            let output = cmd.output().await.map_err(|e| BrainError::ToolFailed {
                tool: "shell".into(),
                reason: format!("{e}"),
            })?;

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
