use agent_tool::{ToolError, TypedTool};
use futures::future::BoxFuture;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[cfg(feature = "component")]
pub(crate) mod component;
#[cfg(feature = "native")]
pub mod native;

/// Applies a V4A patch envelope to workspace-relative paths.
pub struct ApplyPatchTool<D: ApplyPatchDriver> {
    driver: D,
}

impl<D: ApplyPatchDriver> ApplyPatchTool<D> {
    /// Creates an apply-patch tool backed by the provided driver.
    pub fn new(driver: D) -> Self {
        Self { driver }
    }
}

/// Semantic driver for applying V4A patch envelopes.
pub trait ApplyPatchDriver: Send + Sync + 'static {
    /// Applies the provided patch text.
    fn apply_patch(&self, patch: &str) -> BoxFuture<'_, Result<String, ToolError>>;
}

/// Request payload for the `apply_patch` tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplyPatchRequest {
    /// V4A patch text including the begin/end patch markers.
    pub patch: String,
}

/// Summary returned after applying a patch.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
#[schemars(description = "Summary of the patch operations that were applied.")]
pub struct ApplyPatchResponse(pub String);

impl<D: ApplyPatchDriver> TypedTool for ApplyPatchTool<D> {
    type Request = ApplyPatchRequest;
    type Response = ApplyPatchResponse;

    fn name(&self) -> &'static str {
        "apply_patch"
    }

    fn description(&self) -> &'static str {
        "Apply a V4A patch envelope to relative workspace paths. Format: *** Begin Patch ... *** End Patch with *** Add File:, *** Delete File:, or *** Update File: sections, optional *** Move to:, and @@ hunks using space/-/+ line prefixes."
    }

    fn execute_typed<'a>(
        &'a self,
        request: Self::Request,
    ) -> BoxFuture<'a, Result<Self::Response, ToolError>> {
        Box::pin(async move {
            let output = self.driver.apply_patch(&request.patch).await?;
            Ok(ApplyPatchResponse(output))
        })
    }
}
