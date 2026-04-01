use agent_tool::ToolError;
use futures::future::BoxFuture;

use super::{GrepDriver, MAX_LINE_CHARS, MAX_MATCHES};

/// Native regex-based grep implementation.
pub struct NativeGrepDriver;

/// Auto-selects ripgrep when available, otherwise falls back to the native Rust
/// implementation.
#[derive(Default)]
pub struct AutoGrepDriver;

/// ripgrep-backed grep implementation.
pub struct RipgrepDriver;

impl GrepDriver for AutoGrepDriver {
    fn grep(
        &self,
        pattern: &str,
        base_path: Option<&str>,
        include: Option<&str>,
    ) -> BoxFuture<'_, Result<String, ToolError>> {
        if ripgrep_available() {
            RipgrepDriver.grep(pattern, base_path, include)
        } else {
            NativeGrepDriver.grep(pattern, base_path, include)
        }
    }
}

impl GrepDriver for RipgrepDriver {
    fn grep(
        &self,
        pattern: &str,
        base_path: Option<&str>,
        include: Option<&str>,
    ) -> BoxFuture<'_, Result<String, ToolError>> {
        let pattern = pattern.to_owned();
        let base_path = base_path.unwrap_or(".").to_owned();
        let include = include.map(ToOwned::to_owned);
        Box::pin(async move {
            let mut command = tokio::process::Command::new("rg");
            command
                .arg("--line-number")
                .arg("--with-filename")
                .arg("--color")
                .arg("never")
                .arg("--no-heading")
                .arg("--max-count")
                .arg(MAX_MATCHES.to_string())
                .arg(&pattern)
                .arg(&base_path);

            if let Some(include) = include {
                command.arg("--glob").arg(include);
            }

            let output = command
                .output()
                .await
                .map_err(|error| ToolError::new(format!("failed to invoke rg: {error}")))?;

            match output.status.code() {
                Some(0) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
                    let lines = stdout
                        .lines()
                        .map(|line| crate::truncation::truncate_line(line, MAX_LINE_CHARS))
                        .collect::<Vec<_>>();
                    let match_count = lines.len();
                    let mut formatted = format!("{match_count} matches:\n{}", lines.join("\n"));
                    if match_count >= MAX_MATCHES {
                        formatted.push_str(&format!(
                            "\n... results may be truncated at {MAX_MATCHES} matches ..."
                        ));
                    }
                    Ok(formatted)
                }
                Some(1) => Ok("No matches found.".into()),
                _ => {
                    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
                    Err(ToolError::new(if stderr.is_empty() {
                        format!("rg exited with status {}", output.status)
                    } else {
                        stderr
                    }))
                }
            }
        })
    }
}

impl GrepDriver for NativeGrepDriver {
    fn grep(
        &self,
        pattern: &str,
        base_path: Option<&str>,
        include: Option<&str>,
    ) -> BoxFuture<'_, Result<String, ToolError>> {
        let pattern = pattern.to_owned();
        let base_path = base_path.unwrap_or(".").to_owned();
        let include = include.map(ToOwned::to_owned);
        Box::pin(async move {
            let re = regex::Regex::new(&pattern)
                .map_err(|error| ToolError::new(format!("invalid regex: {error}")))?;
            let include_glob = include
                .as_deref()
                .map(glob::Pattern::new)
                .transpose()
                .map_err(|error| ToolError::new(format!("invalid include pattern: {error}")))?;

            let mut results = Vec::new();
            let mut match_count = 0;

            for entry in walkdir::WalkDir::new(&base_path)
                .follow_links(true)
                .into_iter()
                .filter_map(Result::ok)
            {
                if !entry.file_type().is_file() {
                    continue;
                }

                let path = entry.path();
                if let Some(glob_pattern) = include_glob.as_ref() {
                    let file_name = path
                        .file_name()
                        .map(|name| name.to_string_lossy())
                        .unwrap_or_default();
                    if !glob_pattern.matches(&file_name) {
                        continue;
                    }
                }

                let content = match std::fs::read_to_string(path) {
                    Ok(content) => content,
                    Err(_) => continue,
                };

                for (line_num, line) in content.lines().enumerate() {
                    if re.is_match(line) {
                        let path_str = path.to_string_lossy();
                        let line = crate::truncation::truncate_line(line, MAX_LINE_CHARS);
                        results.push(format!("{}:{}:{}", path_str, line_num + 1, line));
                        match_count += 1;
                        if match_count >= MAX_MATCHES {
                            results.push(format!("... truncated at {MAX_MATCHES} matches"));
                            return Ok(results.join("\n"));
                        }
                    }
                }
            }

            if results.is_empty() {
                Ok("No matches found.".into())
            } else {
                Ok(format!("{match_count} matches:\n{}", results.join("\n")))
            }
        })
    }
}

fn ripgrep_available() -> bool {
    std::process::Command::new("rg")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}
