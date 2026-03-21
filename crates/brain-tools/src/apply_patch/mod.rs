#[cfg(feature = "native")]
pub mod native;

use futures::future::BoxFuture;

use brain_types::{BrainError, Tool, ToolDef};

pub trait ApplyPatchDriver: Send + Sync {
    fn apply_patch(&self, patch: &str) -> BoxFuture<'_, Result<String, BrainError>>;
}

pub struct ApplyPatchTool<T: ApplyPatchDriver> {
    driver: T,
}

impl<T: ApplyPatchDriver> ApplyPatchTool<T> {
    pub fn new(driver: T) -> Self {
        Self { driver }
    }
}

impl<T: ApplyPatchDriver> Tool for ApplyPatchTool<T> {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "apply_patch".into(),
            description: "Apply a V4A patch envelope to relative workspace paths. Format: *** Begin Patch ... *** End Patch with *** Add File:, *** Delete File:, or *** Update File: sections, optional *** Move to:, and @@ hunks using space/-/+ line prefixes.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "patch": {
                        "type": "string",
                        "description": "V4A patch text to apply, including *** Begin Patch and *** End Patch markers"
                    }
                },
                "required": ["patch"]
            }),
        }
    }

    fn execute(&self, args: serde_json::Value) -> BoxFuture<'_, Result<String, BrainError>> {
        Box::pin(async move {
            let patch = args
                .get("patch")
                .and_then(|value| value.as_str())
                .ok_or_else(|| BrainError::ToolFailed {
                    tool: "apply_patch".into(),
                    reason: "missing required parameter 'patch'".into(),
                })?;

            tracing::info!("apply_patch invoked");
            self.driver.apply_patch(patch).await
        })
    }
}
