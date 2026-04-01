use agent_tool::{ToolError, TypedTool};
use futures::future::BoxFuture;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[cfg(feature = "component")]
pub(crate) mod component;
#[cfg(feature = "native")]
pub mod native;

#[cfg(feature = "native")]
pub(crate) const DEFAULT_DEPTH: u32 = 3;
#[cfg(feature = "native")]
pub(crate) const MAX_ENTRIES: usize = 200;
#[cfg(feature = "native")]
pub(crate) const IGNORED_NAMES: &[&str] = &[".git", "node_modules", "target"];

/// Lists files and directories in a tree-like view.
pub struct ListDirectoryTool<D: ListDirectoryDriver> {
    driver: D,
}

impl<D: ListDirectoryDriver> ListDirectoryTool<D> {
    /// Creates a list-directory tool backed by the provided driver.
    pub fn new(driver: D) -> Self {
        Self { driver }
    }
}

/// Semantic driver for directory listings.
pub trait ListDirectoryDriver: Send + Sync + 'static {
    /// Returns a tree-like directory listing.
    fn list_directory(
        &self,
        path: &str,
        depth: Option<u32>,
    ) -> BoxFuture<'_, Result<String, ToolError>>;
}

/// Request payload for the `list_directory` tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListDirectoryRequest {
    /// Directory path to list.
    pub path: String,
    /// Maximum recursion depth. Defaults to 3.
    pub depth: Option<u32>,
}

/// Text tree returned by `list_directory`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
#[schemars(description = "Tree-like listing of files and directories.")]
pub struct ListDirectoryResponse(pub String);

impl<D: ListDirectoryDriver> TypedTool for ListDirectoryTool<D> {
    type Request = ListDirectoryRequest;
    type Response = ListDirectoryResponse;

    fn name(&self) -> &'static str {
        "list_directory"
    }

    fn description(&self) -> &'static str {
        "List files and directories in a tree-like view. Useful for understanding repository structure."
    }

    fn execute_typed<'a>(
        &'a self,
        request: Self::Request,
    ) -> BoxFuture<'a, Result<Self::Response, ToolError>> {
        Box::pin(async move {
            let output = self
                .driver
                .list_directory(&request.path, request.depth)
                .await?;
            Ok(ListDirectoryResponse(output))
        })
    }
}
