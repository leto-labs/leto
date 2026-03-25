use std::path::{Path, PathBuf};

use agent_runtime::RuntimeError;
use futures::future::BoxFuture;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::registry::TypedTool;

const DEFAULT_DEPTH: u32 = 3;
const MAX_ENTRIES: usize = 200;
const IGNORED_NAMES: &[&str] = &[".git", "node_modules", "target"];

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
    ) -> BoxFuture<'_, Result<String, RuntimeError>>;
}

/// Request payload for the `list_directory` tool.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListDirectoryRequest {
    /// Directory path to list.
    pub path: String,
    /// Maximum recursion depth. Defaults to 3.
    pub depth: Option<u32>,
}

/// Text tree returned by `list_directory`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
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
    ) -> BoxFuture<'a, Result<Self::Response, RuntimeError>> {
        Box::pin(async move {
            let output = self
                .driver
                .list_directory(&request.path, request.depth)
                .await?;
            Ok(ListDirectoryResponse(output))
        })
    }
}

/// Native filesystem-backed directory lister.
pub struct NativeListDirectoryDriver;

impl ListDirectoryDriver for NativeListDirectoryDriver {
    fn list_directory(
        &self,
        path: &str,
        depth: Option<u32>,
    ) -> BoxFuture<'_, Result<String, RuntimeError>> {
        let path = path.to_owned();
        let max_depth = depth.unwrap_or(DEFAULT_DEPTH);
        Box::pin(async move {
            let root = PathBuf::from(&path);
            let metadata = tokio::fs::metadata(&root).await.map_err(|error| {
                RuntimeError::Tool(format!(
                    "list_directory failed to read {}: {error}",
                    root.display()
                ))
            })?;

            let mut lines = Vec::new();
            let mut visited = 0usize;
            let mut truncated = false;
            if metadata.is_dir() {
                lines.push(format!("{}/", display_name(&root)));
                render_directory(
                    &root,
                    0,
                    max_depth,
                    &mut visited,
                    &mut truncated,
                    &mut lines,
                )?;
            } else {
                lines.push(display_name(&root));
            }

            if truncated {
                lines.push(format!(
                    "...[directory listing truncated at {MAX_ENTRIES} entries; use a narrower path or smaller depth]..."
                ));
            }

            Ok(lines.join("\n"))
        })
    }
}

fn render_directory(
    path: &Path,
    depth: u32,
    max_depth: u32,
    visited: &mut usize,
    truncated: &mut bool,
    lines: &mut Vec<String>,
) -> Result<(), RuntimeError> {
    if depth >= max_depth || *truncated {
        return Ok(());
    }

    let mut entries = std::fs::read_dir(path)
        .map_err(|error| {
            RuntimeError::Tool(format!(
                "list_directory failed to read {}: {error}",
                path.display()
            ))
        })?
        .filter_map(Result::ok)
        .filter(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            !IGNORED_NAMES.iter().any(|ignored| *ignored == name)
        })
        .collect::<Vec<_>>();

    entries.sort_by(|left, right| {
        let left_dir = left.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
        let right_dir = right.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
        right_dir
            .cmp(&left_dir)
            .then_with(|| left.file_name().cmp(&right.file_name()))
    });

    for entry in entries {
        if *visited >= MAX_ENTRIES {
            *truncated = true;
            break;
        }

        let entry_path = entry.path();
        let file_type = entry.file_type().map_err(|error| {
            RuntimeError::Tool(format!(
                "list_directory failed to inspect {}: {error}",
                entry_path.display()
            ))
        })?;
        let indent = "  ".repeat((depth + 1) as usize);
        let name = entry.file_name().to_string_lossy().to_string();
        *visited += 1;

        if file_type.is_dir() {
            lines.push(format!("{indent}{name}/"));
            render_directory(&entry_path, depth + 1, max_depth, visited, truncated, lines)?;
        } else {
            lines.push(format!("{indent}{name}"));
        }
    }

    Ok(())
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| path.display().to_string())
}
