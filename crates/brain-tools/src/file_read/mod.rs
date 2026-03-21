#[cfg(feature = "acp")]
pub mod acp;
#[cfg(feature = "native")]
pub mod native;

use futures::future::BoxFuture;

use brain_types::{BrainError, Tool, ToolDef};

const DEFAULT_LIMIT: usize = 200;
const MAX_LIMIT: usize = 400;
const MAX_LINE_CHARS: usize = 500;

pub trait FileReadDriver: Send + Sync {
    fn read_file(
        &self,
        path: &str,
        offset: Option<usize>,
        limit: Option<usize>,
    ) -> BoxFuture<'_, Result<String, BrainError>>;
}

pub struct FileReadTool<T: FileReadDriver> {
    driver: T,
}

impl<T: FileReadDriver> FileReadTool<T> {
    pub fn new(driver: T) -> Self {
        Self { driver }
    }
}

impl<T: FileReadDriver> Tool for FileReadTool<T> {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "file_read".into(),
            description: "Read file contents with optional line offset and limit.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The file path to read"
                    },
                    "offset": {
                        "type": "integer",
                        "description": "Line number to start from (1-indexed). Defaults to 1."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of lines to return"
                    }
                },
                "required": ["path"]
            }),
        }
    }

    fn execute(&self, args: serde_json::Value) -> BoxFuture<'_, Result<String, BrainError>> {
        Box::pin(async move {
            let path = args.get("path").and_then(|v| v.as_str()).ok_or_else(|| {
                BrainError::ToolFailed {
                    tool: "file_read".into(),
                    reason: "missing required parameter 'path'".into(),
                }
            })?;
            let offset = args
                .get("offset")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize);
            let limit = args
                .get("limit")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize);

            tracing::info!(path, ?offset, ?limit, "file_read invoked");
            self.driver.read_file(path, offset, limit).await
        })
    }
}

pub(super) fn format_file_read_output(
    content: &str,
    offset: Option<usize>,
    limit: Option<usize>,
) -> String {
    use crate::truncation::truncate_line;

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
        .map(|(i, line)| {
            let line = truncate_line(line, MAX_LINE_CHARS);
            format!("{:>6}|{}", start + i + 1, line)
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
