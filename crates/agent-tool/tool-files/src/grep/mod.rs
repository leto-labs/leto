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
#[cfg(feature = "native")]
pub(crate) const MAX_LINE_CHARS: usize = 500;

/// Searches file contents by regex pattern.
pub struct GrepTool<D: GrepDriver> {
    driver: D,
}

impl<D: GrepDriver> GrepTool<D> {
    /// Creates a grep tool backed by the provided driver.
    pub fn new(driver: D) -> Self {
        Self { driver }
    }
}

/// Semantic driver for regex search over files.
pub trait GrepDriver: Send + Sync + 'static {
    /// Searches files matching the provided constraints.
    fn grep(
        &self,
        pattern: &str,
        base_path: Option<&str>,
        include: Option<&str>,
    ) -> BoxFuture<'_, Result<String, ToolError>>;
}

/// Request payload for the `grep` tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GrepRequest {
    /// Regex pattern to search for.
    pub pattern: String,
    /// Directory to search in. Defaults to the current directory.
    pub path: Option<String>,
    /// Optional glob filter for file names, for example `*.rs`.
    pub include: Option<String>,
}

/// Text listing of matching lines with file paths and line numbers.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
#[schemars(description = "Text listing of grep matches with file paths and line numbers.")]
pub struct GrepResponse(pub String);

impl<D: GrepDriver> TypedTool for GrepTool<D> {
    type Request = GrepRequest;
    type Response = GrepResponse;

    fn name(&self) -> &'static str {
        "grep"
    }

    fn description(&self) -> &'static str {
        "Search file contents by regex pattern. Returns matching lines with file paths and line numbers."
    }

    fn execute_typed<'a>(
        &'a self,
        request: Self::Request,
    ) -> BoxFuture<'a, Result<Self::Response, ToolError>> {
        Box::pin(async move {
            let output = self
                .driver
                .grep(
                    &request.pattern,
                    request.path.as_deref(),
                    request.include.as_deref(),
                )
                .await?;
            Ok(GrepResponse(output))
        })
    }
}
