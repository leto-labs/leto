#[cfg(feature = "native")]
pub mod native;

use futures::future::BoxFuture;

use brain_types::{BrainError, Tool, ToolDef};

pub trait GlobDriver: Send + Sync {
    fn glob_search(
        &self,
        pattern: &str,
        base_path: Option<&str>,
    ) -> BoxFuture<'_, Result<String, BrainError>>;
}

pub struct GlobTool<T: GlobDriver> {
    driver: T,
}

impl<T: GlobDriver> GlobTool<T> {
    pub fn new(driver: T) -> Self {
        Self { driver }
    }
}

impl<T: GlobDriver> Tool for GlobTool<T> {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "glob_search".into(),
            description: "Find files matching a glob pattern.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Glob pattern to match (e.g. '**/*.rs')"
                    },
                    "path": {
                        "type": "string",
                        "description": "Base directory to search from (defaults to current directory)"
                    }
                },
                "required": ["pattern"]
            }),
        }
    }

    fn execute(&self, args: serde_json::Value) -> BoxFuture<'_, Result<String, BrainError>> {
        Box::pin(async move {
            let pattern = args
                .get("pattern")
                .and_then(|v| v.as_str())
                .ok_or_else(|| BrainError::ToolFailed {
                    tool: "glob_search".into(),
                    reason: "missing required parameter 'pattern'".into(),
                })?;
            let base_path = args.get("path").and_then(|v| v.as_str());

            tracing::info!(pattern, ?base_path, "glob_search invoked");
            self.driver.glob_search(pattern, base_path).await
        })
    }
}
