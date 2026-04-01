use agent_tool::{ToolError, TypedTool};
use futures::future::BoxFuture;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[cfg(feature = "component")]
pub(crate) mod component;
#[cfg(feature = "native")]
pub mod native;

#[cfg(feature = "native")]
pub(crate) const MAX_MATCHES: usize = 200;

/// Finds files matching a glob pattern.
pub struct GlobSearchTool<D: GlobSearchDriver> {
    driver: D,
}

impl<D: GlobSearchDriver> GlobSearchTool<D> {
    /// Creates a glob-search tool backed by the provided driver.
    pub fn new(driver: D) -> Self {
        Self { driver }
    }
}

/// Semantic driver for glob-based file search.
pub trait GlobSearchDriver: Send + Sync + 'static {
    /// Searches for files matching the provided pattern.
    fn glob_search(
        &self,
        pattern: &str,
        base_path: Option<&str>,
    ) -> BoxFuture<'_, Result<String, ToolError>>;
}

/// Request payload for the `glob_search` tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GlobSearchRequest {
    /// Glob pattern to match, for example `**/*.rs`.
    pub pattern: String,
    /// Base directory to search from. Defaults to the current directory.
    pub path: Option<String>,
}

/// Text listing of matching files.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
#[schemars(description = "Text listing of files matching the requested glob pattern.")]
pub struct GlobSearchResponse(pub String);

impl<D: GlobSearchDriver> TypedTool for GlobSearchTool<D> {
    type Request = GlobSearchRequest;
    type Response = GlobSearchResponse;

    fn name(&self) -> &'static str {
        "glob_search"
    }

    fn description(&self) -> &'static str {
        "Find files matching a glob pattern."
    }

    fn execute_typed<'a>(
        &'a self,
        request: Self::Request,
    ) -> BoxFuture<'a, Result<Self::Response, ToolError>> {
        Box::pin(async move {
            let output = self
                .driver
                .glob_search(&request.pattern, request.path.as_deref())
                .await?;
            Ok(GlobSearchResponse(output))
        })
    }
}
