use agent_tool::{ToolError, TypedTool};
use futures::future::BoxFuture;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[cfg(feature = "component")]
pub(crate) mod component;
#[cfg(feature = "native")]
pub mod native;

#[cfg(feature = "native")]
const DEFAULT_LIMIT: usize = 200;
#[cfg(feature = "native")]
const MAX_LIMIT: usize = 400;
#[cfg(feature = "native")]
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
    ) -> BoxFuture<'_, Result<String, ToolError>>;
}

/// Request payload for the `file_read` tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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
    ) -> BoxFuture<'a, Result<Self::Response, ToolError>> {
        Box::pin(async move {
            let output = self
                .driver
                .read_file(&request.path, request.offset, request.limit)
                .await?;
            Ok(FileReadResponse(output))
        })
    }
}

#[cfg(feature = "native")]
pub(crate) fn format_file_read_output(
    content: &str,
    offset: Option<usize>,
    limit: Option<usize>,
) -> String {
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
            let line = crate::truncation::truncate_line(line, MAX_LINE_CHARS);
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
