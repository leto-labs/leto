use std::path::{Component, Path, PathBuf};

use agent_runtime::RuntimeError;
use futures::future::BoxFuture;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::registry::TypedTool;

const BEGIN_PATCH: &str = "*** Begin Patch";
const END_PATCH: &str = "*** End Patch";
const END_OF_FILE: &str = "*** End of File";
const ADD_FILE: &str = "*** Add File:";
const DELETE_FILE: &str = "*** Delete File:";
const UPDATE_FILE: &str = "*** Update File:";
const MOVE_TO: &str = "*** Move to:";

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
    fn apply_patch(&self, patch: &str) -> BoxFuture<'_, Result<String, RuntimeError>>;
}

/// Request payload for the `apply_patch` tool.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplyPatchRequest {
    /// V4A patch text including the begin/end patch markers.
    pub patch: String,
}

/// Summary returned after applying a patch.
#[derive(Debug, Clone, Serialize, JsonSchema)]
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
    ) -> BoxFuture<'a, Result<Self::Response, RuntimeError>> {
        Box::pin(async move {
            let output = self.driver.apply_patch(&request.patch).await?;
            Ok(ApplyPatchResponse(output))
        })
    }
}

/// Native filesystem-backed V4A patch driver.
pub struct NativeApplyPatchDriver;

#[derive(Debug)]
enum PatchOp {
    Add {
        path: PathBuf,
        contents: Vec<String>,
    },
    Delete {
        path: PathBuf,
    },
    Update {
        path: PathBuf,
        move_to: Option<PathBuf>,
        hunks: Vec<UpdateHunk>,
    },
}

#[derive(Debug)]
struct UpdateHunk {
    headers: Vec<String>,
    old_lines: Vec<String>,
    new_lines: Vec<String>,
    end_of_file: bool,
}

impl ApplyPatchDriver for NativeApplyPatchDriver {
    fn apply_patch(&self, patch: &str) -> BoxFuture<'_, Result<String, RuntimeError>> {
        let patch = patch.to_owned();
        Box::pin(async move {
            let operations = parse_v4a_patch(&patch)?;
            let mut results = Vec::with_capacity(operations.len());

            for operation in operations {
                match operation {
                    PatchOp::Add { path, contents } => {
                        if tokio::fs::try_exists(&path).await.map_err(|error| {
                            tool_failed(format!(
                                "failed to check whether {} exists: {error}",
                                path.display()
                            ))
                        })? {
                            return Err(tool_failed(format!(
                                "cannot add {} because it already exists",
                                path.display()
                            )));
                        }

                        if let Some(parent) = path.parent()
                            && !parent.as_os_str().is_empty()
                        {
                            tokio::fs::create_dir_all(parent).await.map_err(|error| {
                                tool_failed(format!(
                                    "failed to create parent directory for {}: {error}",
                                    path.display()
                                ))
                            })?;
                        }

                        tokio::fs::write(&path, join_lines(&contents))
                            .await
                            .map_err(|error| {
                                tool_failed(format!("failed to write {}: {error}", path.display()))
                            })?;
                        results.push(format!("created {}", path.display()));
                    }
                    PatchOp::Delete { path } => {
                        tokio::fs::remove_file(&path).await.map_err(|error| {
                            tool_failed(format!("failed to remove {}: {error}", path.display()))
                        })?;
                        results.push(format!("deleted {}", path.display()));
                    }
                    PatchOp::Update {
                        path,
                        move_to,
                        hunks,
                    } => {
                        let original = tokio::fs::read_to_string(&path).await.map_err(|error| {
                            tool_failed(format!("failed to read {}: {error}", path.display()))
                        })?;
                        let mut lines = split_lines(&original);
                        let mut cursor = 0usize;

                        for hunk in &hunks {
                            apply_update_hunk(&mut lines, hunk, &path, &mut cursor)?;
                        }

                        let updated = join_lines(&lines);
                        if let Some(new_path) = move_to {
                            if let Some(parent) = new_path.parent()
                                && !parent.as_os_str().is_empty()
                            {
                                tokio::fs::create_dir_all(parent).await.map_err(|error| {
                                    tool_failed(format!(
                                        "failed to create parent directory for {}: {error}",
                                        new_path.display()
                                    ))
                                })?;
                            }
                            tokio::fs::write(&new_path, updated)
                                .await
                                .map_err(|error| {
                                    tool_failed(format!(
                                        "failed to write {}: {error}",
                                        new_path.display()
                                    ))
                                })?;
                            tokio::fs::remove_file(&path).await.map_err(|error| {
                                tool_failed(format!("failed to remove {}: {error}", path.display()))
                            })?;
                            results.push(format!(
                                "moved {} -> {}",
                                path.display(),
                                new_path.display()
                            ));
                        } else {
                            tokio::fs::write(&path, updated).await.map_err(|error| {
                                tool_failed(format!("failed to write {}: {error}", path.display()))
                            })?;
                            results.push(format!("updated {}", path.display()));
                        }
                    }
                }
            }

            Ok(results.join("\n"))
        })
    }
}

fn parse_v4a_patch(patch: &str) -> Result<Vec<PatchOp>, RuntimeError> {
    let normalized = patch.replace("\r\n", "\n").replace('\r', "\n");
    let lines: Vec<&str> = normalized.lines().collect();

    let begin = lines
        .iter()
        .position(|line| line.trim() == BEGIN_PATCH)
        .ok_or_else(|| tool_failed("invalid patch: missing *** Begin Patch marker"))?;
    let end = lines
        .iter()
        .rposition(|line| line.trim() == END_PATCH)
        .ok_or_else(|| tool_failed("invalid patch: missing *** End Patch marker"))?;

    if begin >= end {
        return Err(tool_failed(
            "invalid patch: *** Begin Patch must appear before *** End Patch",
        ));
    }

    let mut operations = Vec::new();
    let mut index = begin + 1;

    while index < end {
        let line = lines[index];
        if line.trim().is_empty() {
            index += 1;
            continue;
        }

        if let Some(path) = line.strip_prefix(ADD_FILE) {
            let path = resolve_patch_path(path.trim())?;
            index += 1;
            let mut contents = Vec::new();
            while index < end && !is_operation_header(lines[index]) {
                let added = lines[index].strip_prefix('+').ok_or_else(|| {
                    tool_failed(format!(
                        "invalid add file section for {}: every line must start with '+'",
                        path.display()
                    ))
                })?;
                contents.push(added.to_owned());
                index += 1;
            }
            operations.push(PatchOp::Add { path, contents });
            continue;
        }

        if let Some(path) = line.strip_prefix(DELETE_FILE) {
            let path = resolve_patch_path(path.trim())?;
            index += 1;
            operations.push(PatchOp::Delete { path });
            continue;
        }

        if let Some(path) = line.strip_prefix(UPDATE_FILE) {
            let path = resolve_patch_path(path.trim())?;
            index += 1;

            let mut move_to = None;
            if index < end && lines[index].starts_with(MOVE_TO) {
                move_to = Some(resolve_patch_path(lines[index][MOVE_TO.len()..].trim())?);
                index += 1;
            }

            let mut hunks = Vec::new();
            while index < end && !is_operation_header(lines[index]) {
                if lines[index].trim().is_empty() {
                    index += 1;
                    continue;
                }
                if !lines[index].starts_with("@@") {
                    return Err(tool_failed(format!(
                        "invalid update section for {}: expected '@@' or a file operation header",
                        path.display()
                    )));
                }

                let mut headers = Vec::new();
                while index < end && lines[index].starts_with("@@") {
                    headers.push(lines[index][2..].trim().to_owned());
                    index += 1;
                }

                let mut old_lines = Vec::new();
                let mut new_lines = Vec::new();
                let mut saw_change = false;
                let mut end_of_file = false;

                while index < end
                    && !lines[index].starts_with("@@")
                    && !is_operation_header(lines[index])
                {
                    let hunk_line = lines[index];
                    if hunk_line == END_OF_FILE {
                        end_of_file = true;
                        index += 1;
                        break;
                    }

                    if let Some(line) = hunk_line.strip_prefix(' ') {
                        old_lines.push(line.to_owned());
                        new_lines.push(line.to_owned());
                        saw_change = true;
                    } else if let Some(line) = hunk_line.strip_prefix('-') {
                        old_lines.push(line.to_owned());
                        saw_change = true;
                    } else if let Some(line) = hunk_line.strip_prefix('+') {
                        new_lines.push(line.to_owned());
                        saw_change = true;
                    } else {
                        return Err(tool_failed(format!(
                            "invalid update line for {}: expected one of ' ', '-', '+'",
                            path.display()
                        )));
                    }
                    index += 1;
                }

                if !saw_change {
                    return Err(tool_failed(format!(
                        "invalid update section for {}: empty hunk",
                        path.display()
                    )));
                }

                hunks.push(UpdateHunk {
                    headers,
                    old_lines,
                    new_lines,
                    end_of_file,
                });
            }

            if hunks.is_empty() {
                return Err(tool_failed(format!(
                    "invalid update section for {}: missing hunks",
                    path.display()
                )));
            }

            operations.push(PatchOp::Update {
                path,
                move_to,
                hunks,
            });
            continue;
        }

        return Err(tool_failed(format!("invalid patch line: {line}")));
    }

    Ok(operations)
}

fn is_operation_header(line: &str) -> bool {
    line.starts_with(ADD_FILE)
        || line.starts_with(DELETE_FILE)
        || line.starts_with(UPDATE_FILE)
        || line.trim() == END_PATCH
}

fn resolve_patch_path(raw: &str) -> Result<PathBuf, RuntimeError> {
    let path = Path::new(raw);
    if path.is_absolute() {
        return Err(tool_failed(format!(
            "absolute paths are not allowed: {raw}"
        )));
    }
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(tool_failed(format!(
            "path cannot escape the workspace: {raw}"
        )));
    }
    Ok(path.to_path_buf())
}

fn apply_update_hunk(
    lines: &mut Vec<String>,
    hunk: &UpdateHunk,
    path: &Path,
    cursor: &mut usize,
) -> Result<(), RuntimeError> {
    let headers = hunk
        .headers
        .iter()
        .map(String::as_str)
        .filter(|header| !header.is_empty())
        .collect::<Vec<_>>();

    let start = if headers.is_empty() {
        *cursor
    } else {
        find_hunk_start(lines, &headers, *cursor).ok_or_else(|| {
            tool_failed(format!(
                "failed to locate hunk context in {} for headers {:?}",
                path.display(),
                hunk.headers
            ))
        })?
    };

    if start + hunk.old_lines.len() > lines.len() {
        return Err(tool_failed(format!(
            "hunk extends beyond end of file {}",
            path.display()
        )));
    }

    let existing = &lines[start..start + hunk.old_lines.len()];
    if existing != hunk.old_lines {
        return Err(tool_failed(format!(
            "hunk did not match existing content in {}",
            path.display()
        )));
    }

    lines.splice(start..start + hunk.old_lines.len(), hunk.new_lines.clone());
    *cursor = if hunk.end_of_file {
        lines.len()
    } else {
        start + hunk.new_lines.len()
    };
    Ok(())
}

fn find_hunk_start(lines: &[String], headers: &[&str], cursor: usize) -> Option<usize> {
    if headers.is_empty() {
        return Some(cursor.min(lines.len()));
    }

    for start in cursor..=lines.len().saturating_sub(headers.len()) {
        if lines[start..start + headers.len()]
            .iter()
            .map(String::as_str)
            .eq(headers.iter().copied())
        {
            return Some(start);
        }
    }

    for start in 0..cursor.min(lines.len()) {
        if start + headers.len() <= lines.len()
            && lines[start..start + headers.len()]
                .iter()
                .map(String::as_str)
                .eq(headers.iter().copied())
        {
            return Some(start);
        }
    }

    None
}

fn split_lines(input: &str) -> Vec<String> {
    if input.is_empty() {
        return Vec::new();
    }
    input.lines().map(|line| line.to_owned()).collect()
}

fn join_lines(lines: &[String]) -> String {
    lines.join("\n")
}

fn tool_failed(message: impl Into<String>) -> RuntimeError {
    RuntimeError::Tool(message.into())
}
