#[cfg(feature = "native")]
pub mod native;

use futures::future::BoxFuture;

use brain_types::{BrainError, Tool, ToolDef};

pub trait FileWriteDriver: Send + Sync {
    fn write_file(
        &self,
        path: &str,
        content: &str,
    ) -> BoxFuture<'_, Result<String, BrainError>>;
}

pub struct FileWriteTool<T: FileWriteDriver> {
    driver: T,
}

impl<T: FileWriteDriver> FileWriteTool<T> {
    pub fn new(driver: T) -> Self {
        Self { driver }
    }
}

impl<T: FileWriteDriver> Tool for FileWriteTool<T> {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "file_write".into(),
            description: "Create or overwrite a file with the given content.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The file path to write"
                    },
                    "content": {
                        "type": "string",
                        "description": "The content to write to the file"
                    }
                },
                "required": ["path", "content"]
            }),
        }
    }

    fn execute(&self, args: serde_json::Value) -> BoxFuture<'_, Result<String, BrainError>> {
        Box::pin(async move {
            let path = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| BrainError::ToolFailed {
                    tool: "file_write".into(),
                    reason: "missing required parameter 'path'".into(),
                })?;
            let content = args
                .get("content")
                .and_then(|v| v.as_str())
                .ok_or_else(|| BrainError::ToolFailed {
                    tool: "file_write".into(),
                    reason: "missing required parameter 'content'".into(),
                })?;

            tracing::info!(path, "file_write invoked");
            self.driver.write_file(path, content).await
        })
    }
}
