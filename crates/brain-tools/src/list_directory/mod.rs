#[cfg(feature = "native")]
pub mod native;

use futures::future::BoxFuture;

use brain_types::{BrainError, Tool, ToolDef};

pub trait ListDirectoryDriver: Send + Sync {
    fn list_directory(
        &self,
        path: &str,
        depth: Option<u32>,
    ) -> BoxFuture<'_, Result<String, BrainError>>;
}

pub struct ListDirectoryTool<T: ListDirectoryDriver> {
    driver: T,
}

impl<T: ListDirectoryDriver> ListDirectoryTool<T> {
    pub fn new(driver: T) -> Self {
        Self { driver }
    }
}

impl<T: ListDirectoryDriver> Tool for ListDirectoryTool<T> {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "list_directory".into(),
            description: "List files and directories in a tree-like view. Useful for understanding repository structure.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Directory path to list"
                    },
                    "depth": {
                        "type": "integer",
                        "description": "Maximum recursion depth. Defaults to 3."
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
                .and_then(|value| value.as_str())
                .ok_or_else(|| BrainError::ToolFailed {
                    tool: "list_directory".into(),
                    reason: "missing required parameter 'path'".into(),
                })?;
            let depth = args
                .get("depth")
                .and_then(|value| value.as_u64())
                .map(|v| v as u32);

            tracing::info!(path, ?depth, "list_directory invoked");
            self.driver.list_directory(path, depth).await
        })
    }
}
