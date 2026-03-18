#[cfg(feature = "native")]
pub mod native;

use futures::future::BoxFuture;

use brain_types::{BrainError, Tool, ToolDef};

pub trait FileEditDriver: Send + Sync {
    fn edit_file(
        &self,
        path: &str,
        old_string: &str,
        new_string: &str,
    ) -> BoxFuture<'_, Result<String, BrainError>>;
}

pub struct FileEditTool<T: FileEditDriver> {
    driver: T,
}

impl<T: FileEditDriver> FileEditTool<T> {
    pub fn new(driver: T) -> Self {
        Self { driver }
    }
}

impl<T: FileEditDriver> Tool for FileEditTool<T> {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "file_edit".into(),
            description: "Edit a file by replacing an exact string match with new content. The old_string must appear exactly once in the file.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The file path to edit"
                    },
                    "old_string": {
                        "type": "string",
                        "description": "The exact text to find and replace (must be unique in the file)"
                    },
                    "new_string": {
                        "type": "string",
                        "description": "The replacement text"
                    }
                },
                "required": ["path", "old_string", "new_string"]
            }),
        }
    }

    fn execute(&self, args: serde_json::Value) -> BoxFuture<'_, Result<String, BrainError>> {
        Box::pin(async move {
            let path = args.get("path").and_then(|v| v.as_str()).ok_or_else(|| {
                BrainError::ToolFailed {
                    tool: "file_edit".into(),
                    reason: "missing required parameter 'path'".into(),
                }
            })?;
            let old_string = args
                .get("old_string")
                .and_then(|v| v.as_str())
                .ok_or_else(|| BrainError::ToolFailed {
                    tool: "file_edit".into(),
                    reason: "missing required parameter 'old_string'".into(),
                })?;
            let new_string = args
                .get("new_string")
                .and_then(|v| v.as_str())
                .ok_or_else(|| BrainError::ToolFailed {
                    tool: "file_edit".into(),
                    reason: "missing required parameter 'new_string'".into(),
                })?;

            tracing::info!(path, "file_edit invoked");
            self.driver.edit_file(path, old_string, new_string).await
        })
    }
}
