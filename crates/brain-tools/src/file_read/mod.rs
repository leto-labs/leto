#[cfg(feature = "native")]
pub mod native;

use futures::future::BoxFuture;

use brain_types::{BrainError, Tool, ToolDef};

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
            let path = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| BrainError::ToolFailed {
                    tool: "file_read".into(),
                    reason: "missing required parameter 'path'".into(),
                })?;
            let offset = args.get("offset").and_then(|v| v.as_u64()).map(|v| v as usize);
            let limit = args.get("limit").and_then(|v| v.as_u64()).map(|v| v as usize);

            tracing::info!(path, ?offset, ?limit, "file_read invoked");
            self.driver.read_file(path, offset, limit).await
        })
    }
}
