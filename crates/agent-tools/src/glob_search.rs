use std::path::PathBuf;

use agent_runtime::RuntimeError;
use futures::future::BoxFuture;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::registry::TypedTool;

const MAX_MATCHES: usize = 200;

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
    ) -> BoxFuture<'_, Result<String, RuntimeError>>;
}

/// Request payload for the `glob_search` tool.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GlobSearchRequest {
    /// Glob pattern to match, for example `**/*.rs`.
    pub pattern: String,
    /// Base directory to search from. Defaults to the current directory.
    pub path: Option<String>,
}

/// Text listing of matching files.
#[derive(Debug, Clone, Serialize, JsonSchema)]
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
    ) -> BoxFuture<'a, Result<Self::Response, RuntimeError>> {
        Box::pin(async move {
            let output = self
                .driver
                .glob_search(&request.pattern, request.path.as_deref())
                .await?;
            Ok(GlobSearchResponse(output))
        })
    }
}

/// Native filesystem-backed glob search driver.
pub struct NativeGlobSearchDriver;

impl GlobSearchDriver for NativeGlobSearchDriver {
    fn glob_search(
        &self,
        pattern: &str,
        base_path: Option<&str>,
    ) -> BoxFuture<'_, Result<String, RuntimeError>> {
        let pattern = pattern.to_owned();
        let base_path = base_path.map(ToOwned::to_owned);
        Box::pin(async move {
            let full_pattern = match &base_path {
                Some(base) => PathBuf::from(base)
                    .join(&pattern)
                    .to_string_lossy()
                    .into_owned(),
                None => pattern,
            };

            let mut paths: Vec<String> = glob::glob(&full_pattern)
                .map_err(|error| RuntimeError::Tool(format!("invalid glob pattern: {error}")))?
                .filter_map(Result::ok)
                .map(|path| path.to_string_lossy().into_owned())
                .collect();
            paths.sort();

            if paths.is_empty() {
                Ok("No files found matching the pattern.".into())
            } else {
                let total = paths.len();
                let truncated = total > MAX_MATCHES;
                let paths = paths.into_iter().take(MAX_MATCHES).collect::<Vec<_>>();
                let mut output = format!("{total} files found:\n{}", paths.join("\n"));
                if truncated {
                    output.push_str(&format!(
                        "\n... results truncated at {MAX_MATCHES} files; use a narrower pattern or path ..."
                    ));
                }
                Ok(output)
            }
        })
    }
}
