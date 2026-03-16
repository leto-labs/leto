#[cfg(feature = "native")]
pub mod native;

use futures::future::BoxFuture;

use brain_types::{BrainError, Tool, ToolDef};

pub trait GrepDriver: Send + Sync {
    fn grep(
        &self,
        pattern: &str,
        base_path: Option<&str>,
        include: Option<&str>,
    ) -> BoxFuture<'_, Result<String, BrainError>>;
}

pub struct GrepTool<T: GrepDriver> {
    driver: T,
}

impl<T: GrepDriver> GrepTool<T> {
    pub fn new(driver: T) -> Self {
        Self { driver }
    }
}

impl<T: GrepDriver> Tool for GrepTool<T> {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "grep".into(),
            description: "Search file contents by regex pattern. Returns matching lines with file paths and line numbers.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Regex pattern to search for"
                    },
                    "path": {
                        "type": "string",
                        "description": "Directory to search in (defaults to current directory)"
                    },
                    "include": {
                        "type": "string",
                        "description": "Glob filter for file names (e.g. '*.rs')"
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
                    tool: "grep".into(),
                    reason: "missing required parameter 'pattern'".into(),
                })?;
            let base_path = args.get("path").and_then(|v| v.as_str());
            let include = args.get("include").and_then(|v| v.as_str());

            tracing::info!(pattern, ?base_path, ?include, "grep invoked");
            self.driver.grep(pattern, base_path, include).await
        })
    }
}
