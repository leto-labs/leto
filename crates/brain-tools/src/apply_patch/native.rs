use futures::future::BoxFuture;
use std::path::{Component, Path, PathBuf};

use brain_types::BrainError;

use super::ApplyPatchDriver;

const BEGIN_PATCH: &str = "*** Begin Patch";
const END_PATCH: &str = "*** End Patch";
const END_OF_FILE: &str = "*** End of File";
const ADD_FILE: &str = "*** Add File:";
const DELETE_FILE: &str = "*** Delete File:";
const UPDATE_FILE: &str = "*** Update File:";
const MOVE_TO: &str = "*** Move to:";

pub struct ApplyPatchDriverNative;

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

impl ApplyPatchDriver for ApplyPatchDriverNative {
    fn apply_patch(&self, patch: &str) -> BoxFuture<'_, Result<String, BrainError>> {
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
                        let content = tokio::fs::read_to_string(&path).await.map_err(|error| {
                            tool_failed(format!("failed to read {}: {error}", path.display()))
                        })?;
                        if !content.is_empty() {
                            tokio::fs::remove_file(&path).await.map_err(|error| {
                                tool_failed(format!("failed to remove {}: {error}", path.display()))
                            })?;
                        } else {
                            tokio::fs::remove_file(&path).await.map_err(|error| {
                                tool_failed(format!("failed to remove {}: {error}", path.display()))
                            })?;
                        }
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

fn parse_v4a_patch(patch: &str) -> Result<Vec<PatchOp>, BrainError> {
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
                let add_line = lines[index];
                let added = add_line.strip_prefix('+').ok_or_else(|| {
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
                            "invalid hunk line in {}: expected ' ', '-', or '+' prefix",
                            path.display()
                        )));
                    }

                    index += 1;
                }

                if !saw_change && !end_of_file {
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
                    "invalid update section for {}: expected at least one hunk",
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

        return Err(tool_failed(format!(
            "invalid patch: unexpected line inside patch: {line}"
        )));
    }

    if operations.is_empty() {
        return Err(tool_failed("patch rejected: empty patch"));
    }

    Ok(operations)
}

fn apply_update_hunk(
    lines: &mut Vec<String>,
    hunk: &UpdateHunk,
    path: &Path,
    cursor: &mut usize,
) -> Result<(), BrainError> {
    if hunk.old_lines.is_empty() {
        if hunk.end_of_file {
            let insert_at = eof_insertion_index(lines);
            lines.splice(insert_at..insert_at, hunk.new_lines.clone());
            *cursor = insert_at + hunk.new_lines.len();
            return Ok(());
        }

        return Err(tool_failed(format!(
            "failed to apply patch to {}: hunk must include context or removed lines",
            path.display()
        )));
    }

    let start = find_unique_match(lines, &hunk.old_lines, *cursor, path, &hunk.headers)?;
    let end = start + hunk.old_lines.len();

    if hunk.end_of_file && end != eof_insertion_index(lines) {
        return Err(tool_failed(format!(
            "failed to apply patch to {}: hunk marked with *** End of File does not match the end of the file",
            path.display()
        )));
    }

    lines.splice(start..end, hunk.new_lines.clone());
    *cursor = start + hunk.new_lines.len();
    Ok(())
}

fn find_unique_match(
    lines: &[String],
    needle: &[String],
    cursor: usize,
    path: &Path,
    headers: &[String],
) -> Result<usize, BrainError> {
    let matches_after_cursor = find_matches(lines, needle, cursor);
    if matches_after_cursor.len() == 1 {
        return Ok(matches_after_cursor[0]);
    }
    if matches_after_cursor.len() > 1 {
        return Err(tool_failed(format!(
            "failed to apply patch to {}: hunk matched multiple locations{}",
            path.display(),
            format_header_suffix(headers)
        )));
    }

    let all_matches = find_matches(lines, needle, 0);
    match all_matches.len() {
        0 => Err(tool_failed(format!(
            "failed to apply patch to {}: missing context{}",
            path.display(),
            format_header_suffix(headers)
        ))),
        1 => Ok(all_matches[0]),
        _ => Err(tool_failed(format!(
            "failed to apply patch to {}: hunk matched multiple locations{}",
            path.display(),
            format_header_suffix(headers)
        ))),
    }
}

fn find_matches(lines: &[String], needle: &[String], start: usize) -> Vec<usize> {
    if needle.is_empty() || lines.len() < needle.len() {
        return Vec::new();
    }

    let mut matches = Vec::new();
    for index in start..=lines.len() - needle.len() {
        if lines[index..index + needle.len()] == *needle {
            matches.push(index);
        }
    }
    matches
}

fn eof_insertion_index(lines: &[String]) -> usize {
    if matches!(lines.last(), Some(last) if last.is_empty()) {
        lines.len().saturating_sub(1)
    } else {
        lines.len()
    }
}

fn split_lines(input: &str) -> Vec<String> {
    if input.is_empty() {
        Vec::new()
    } else {
        input.split('\n').map(ToOwned::to_owned).collect()
    }
}

fn join_lines(lines: &[String]) -> String {
    lines.join("\n")
}

fn is_operation_header(line: &str) -> bool {
    line.starts_with(ADD_FILE) || line.starts_with(DELETE_FILE) || line.starts_with(UPDATE_FILE)
}

fn resolve_patch_path(path: &str) -> Result<PathBuf, BrainError> {
    if path.is_empty() {
        return Err(tool_failed("invalid patch: missing file path"));
    }

    let path = Path::new(path);
    if path.is_absolute() {
        return Err(tool_failed("invalid patch: absolute paths are not allowed"));
    }

    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(tool_failed(
            "invalid patch: paths must be relative and stay within the workspace",
        ));
    }

    Ok(path.to_path_buf())
}

fn format_header_suffix(headers: &[String]) -> String {
    let headers: Vec<&str> = headers
        .iter()
        .map(|header| header.trim())
        .filter(|header| !header.is_empty())
        .collect();

    if headers.is_empty() {
        String::new()
    } else {
        format!(" near {}", headers.join(" / "))
    }
}

fn tool_failed(reason: impl Into<String>) -> BrainError {
    BrainError::ToolFailed {
        tool: "apply_patch".into(),
        reason: reason.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tempfile::tempdir;

    static CWD_LOCK: Mutex<()> = Mutex::new(());

    #[tokio::test]
    async fn applies_v4a_patch_to_existing_file() {
        let _guard = CWD_LOCK.lock().unwrap();
        let dir = tempdir().unwrap();
        let original_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        tokio::fs::write("notes.txt", "hello\nworld\n")
            .await
            .unwrap();

        let patch = "\
*** Begin Patch
*** Update File: notes.txt
@@ class Greeting
 hello
-world
+brain
*** End Patch";

        let result = ApplyPatchDriverNative.apply_patch(patch).await.unwrap();

        assert_eq!(
            tokio::fs::read_to_string("notes.txt").await.unwrap(),
            "hello\nbrain\n"
        );
        assert_eq!(result, "updated notes.txt");
        std::env::set_current_dir(original_cwd).unwrap();
    }

    #[tokio::test]
    async fn supports_add_delete_and_move_operations() {
        let _guard = CWD_LOCK.lock().unwrap();
        let dir = tempdir().unwrap();
        let original_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        tokio::fs::create_dir_all("old").await.unwrap();
        tokio::fs::write("old/name.txt", "old content\n")
            .await
            .unwrap();
        tokio::fs::write("remove.txt", "remove me\n").await.unwrap();

        let patch = "\
*** Begin Patch
*** Add File: nested/new.txt
+created
*** Delete File: remove.txt
*** Update File: old/name.txt
*** Move to: renamed/dir/name.txt
@@
-old content
+new content
*** End Patch";

        let result = ApplyPatchDriverNative.apply_patch(patch).await.unwrap();

        assert_eq!(
            tokio::fs::read_to_string("nested/new.txt").await.unwrap(),
            "created"
        );
        assert_eq!(
            tokio::fs::read_to_string("renamed/dir/name.txt")
                .await
                .unwrap(),
            "new content\n"
        );
        assert!(!Path::new("old/name.txt").exists());
        assert!(!Path::new("remove.txt").exists());
        assert!(result.contains("created nested/new.txt"));
        assert!(result.contains("deleted remove.txt"));
        assert!(result.contains("moved old/name.txt -> renamed/dir/name.txt"));
        std::env::set_current_dir(original_cwd).unwrap();
    }

    #[tokio::test]
    async fn supports_multiple_anchor_headers_and_end_of_file() {
        let _guard = CWD_LOCK.lock().unwrap();
        let dir = tempdir().unwrap();
        let original_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        tokio::fs::write("tail.txt", "line1\nlast\n").await.unwrap();

        let patch = "\
*** Begin Patch
*** Update File: tail.txt
@@ class Tail
@@ fn finish()
-last
+end
*** End of File
*** End Patch";

        ApplyPatchDriverNative.apply_patch(patch).await.unwrap();

        assert_eq!(
            tokio::fs::read_to_string("tail.txt").await.unwrap(),
            "line1\nend\n"
        );
        std::env::set_current_dir(original_cwd).unwrap();
    }

    #[tokio::test]
    async fn rejects_invalid_markers_and_absolute_paths() {
        let err = ApplyPatchDriverNative
            .apply_patch("*** Update File: notes.txt\n@@\n-old\n+new\n")
            .await
            .unwrap_err();
        assert!(format!("{err}").contains("missing *** Begin Patch marker"));

        let err = ApplyPatchDriverNative
            .apply_patch(
                "\
*** Begin Patch
*** Add File: /tmp/nope.txt
+hello
*** End Patch",
            )
            .await
            .unwrap_err();
        assert!(format!("{err}").contains("absolute paths are not allowed"));
    }
}
