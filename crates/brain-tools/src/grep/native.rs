use futures::future::BoxFuture;

use brain_types::BrainError;

use super::GrepDriver;
use crate::truncation::truncate_line;

const MAX_MATCHES: usize = 200;
const MAX_LINE_CHARS: usize = 500;

pub struct GrepDriverNative;

#[derive(Default)]
pub struct GrepDriverAuto;

pub struct GrepDriverRipgrep;

impl GrepDriver for GrepDriverAuto {
    fn grep(
        &self,
        pattern: &str,
        base_path: Option<&str>,
        include: Option<&str>,
    ) -> BoxFuture<'_, Result<String, BrainError>> {
        if ripgrep_available() {
            GrepDriverRipgrep.grep(pattern, base_path, include)
        } else {
            GrepDriverNative.grep(pattern, base_path, include)
        }
    }
}

impl GrepDriver for GrepDriverRipgrep {
    fn grep(
        &self,
        pattern: &str,
        base_path: Option<&str>,
        include: Option<&str>,
    ) -> BoxFuture<'_, Result<String, BrainError>> {
        let pattern = pattern.to_owned();
        let base_path = base_path.unwrap_or(".").to_owned();
        let include = include.map(|value| value.to_owned());
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
                .map_err(|error| BrainError::ToolFailed {
                    tool: "grep".into(),
                    reason: format!("failed to invoke rg: {error}"),
                })?;

            match output.status.code() {
                Some(0) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
                    let lines = stdout
                        .lines()
                        .map(|line| truncate_line(line, MAX_LINE_CHARS))
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
                    Err(BrainError::ToolFailed {
                        tool: "grep".into(),
                        reason: if stderr.is_empty() {
                            format!("rg exited with status {}", output.status)
                        } else {
                            stderr
                        },
                    })
                }
            }
        })
    }
}

impl GrepDriver for GrepDriverNative {
    fn grep(
        &self,
        pattern: &str,
        base_path: Option<&str>,
        include: Option<&str>,
    ) -> BoxFuture<'_, Result<String, BrainError>> {
        let pattern = pattern.to_owned();
        let base_path = base_path.unwrap_or(".").to_owned();
        let include = include.map(|s| s.to_owned());
        Box::pin(async move {
            let re = regex::Regex::new(&pattern).map_err(|e| BrainError::ToolFailed {
                tool: "grep".into(),
                reason: format!("invalid regex: {e}"),
            })?;

            let include_glob = include
                .as_deref()
                .map(glob::Pattern::new)
                .transpose()
                .map_err(|e| BrainError::ToolFailed {
                    tool: "grep".into(),
                    reason: format!("invalid include pattern: {e}"),
                })?;

            let mut results = Vec::new();
            let mut match_count = 0;

            for entry in walkdir::WalkDir::new(&base_path)
                .follow_links(true)
                .into_iter()
                .filter_map(|entry| entry.ok())
            {
                if !entry.file_type().is_file() {
                    continue;
                }

                let path = entry.path();
                if let Some(ref glob_pattern) = include_glob {
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
                        let line = truncate_line(line, MAX_LINE_CHARS);
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn native_grep_finds_matches() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("lib.rs");
        tokio::fs::write(&file, "fn main() {}\nfn helper() {}\n")
            .await
            .unwrap();

        let output = GrepDriverNative
            .grep("fn main", Some(dir.path().to_str().unwrap()), Some("*.rs"))
            .await
            .unwrap();

        assert!(output.contains("1 matches"));
        assert!(output.contains("lib.rs:1:fn main() {}"));
    }

    #[tokio::test]
    async fn auto_driver_works_without_exposing_rg_as_a_tool() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("notes.txt");
        tokio::fs::write(&file, "brain\n").await.unwrap();

        let output = GrepDriverAuto::default()
            .grep("brain", Some(dir.path().to_str().unwrap()), Some("*.txt"))
            .await
            .unwrap();

        assert!(output.contains("notes.txt:1:brain"));
    }
}
