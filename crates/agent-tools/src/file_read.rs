use agent_runtime::RuntimeError;
use futures::future::BoxFuture;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::registry::TypedTool;
use crate::truncation::truncate_line;

const DEFAULT_LIMIT: usize = 200;
const MAX_LIMIT: usize = 400;
const MAX_LINE_CHARS: usize = 500;

/// Reads file contents with optional line offset and limit.
pub struct FileReadTool<D: FileReadDriver> {
    driver: D,
}

impl<D: FileReadDriver> FileReadTool<D> {
    /// Creates a file-read tool backed by the provided driver.
    pub fn new(driver: D) -> Self {
        Self { driver }
    }
}

/// Semantic driver for reading file contents.
pub trait FileReadDriver: Send + Sync + 'static {
    /// Reads a file and returns formatted text content.
    fn read_file(
        &self,
        path: &str,
        offset: Option<usize>,
        limit: Option<usize>,
    ) -> BoxFuture<'_, Result<String, RuntimeError>>;
}

/// Request payload for the `file_read` tool.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FileReadRequest {
    /// The file path to read.
    pub path: String,
    /// Line number to start from (1-indexed). Defaults to 1.
    pub offset: Option<usize>,
    /// Maximum number of lines to return.
    pub limit: Option<usize>,
}

/// Formatted file contents returned by `file_read`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(transparent)]
#[schemars(description = "Formatted file contents with line numbers and truncation notices.")]
pub struct FileReadResponse(pub String);

impl<D: FileReadDriver> TypedTool for FileReadTool<D> {
    type Request = FileReadRequest;
    type Response = FileReadResponse;

    fn name(&self) -> &'static str {
        "file_read"
    }

    fn description(&self) -> &'static str {
        "Read file contents with optional line offset and limit."
    }

    fn execute_typed<'a>(
        &'a self,
        request: Self::Request,
    ) -> BoxFuture<'a, Result<Self::Response, RuntimeError>> {
        Box::pin(async move {
            let output = self
                .driver
                .read_file(&request.path, request.offset, request.limit)
                .await?;
            Ok(FileReadResponse(output))
        })
    }
}

/// Native filesystem-backed file reader.
pub struct NativeFileReadDriver;

impl FileReadDriver for NativeFileReadDriver {
    fn read_file(
        &self,
        path: &str,
        offset: Option<usize>,
        limit: Option<usize>,
    ) -> BoxFuture<'_, Result<String, RuntimeError>> {
        let path = path.to_owned();
        Box::pin(async move {
            let content = tokio::fs::read_to_string(&path).await.map_err(|error| {
                RuntimeError::Tool(format!("file_read failed for {path}: {error}"))
            })?;
            Ok(format_file_read_output(&content, offset, limit))
        })
    }
}

fn format_file_read_output(content: &str, offset: Option<usize>, limit: Option<usize>) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let start = offset.map(|o| o.saturating_sub(1)).unwrap_or(0);
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT);
    let end = (start + limit).min(lines.len());

    if start >= lines.len() {
        return "Requested offset is beyond the end of the file.".into();
    }

    let mut formatted = lines[start..end]
        .iter()
        .enumerate()
        .map(|(idx, line)| {
            let line = truncate_line(line, MAX_LINE_CHARS);
            format!("{:>6}|{}", start + idx + 1, line)
        })
        .collect::<Vec<_>>()
        .join("\n");

    if end < lines.len() {
        formatted.push_str(&format!(
            "\n\n...[showing lines {}-{} of {}. Use offset={} to continue]...",
            start + 1,
            end,
            lines.len(),
            end + 1
        ));
    }

    formatted
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn reads_file_with_line_numbers() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("notes.txt");
        tokio::fs::write(&path, "first\nsecond\n")
            .await
            .expect("write file");

        let output = NativeFileReadDriver
            .read_file(path.to_str().expect("path"), None, None)
            .await
            .expect("read file");
        assert!(output.contains("1|first"));
        assert!(output.contains("2|second"));
    }
}
