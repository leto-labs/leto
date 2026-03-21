use std::path::{Path, PathBuf};

use futures::future::BoxFuture;

use brain_types::BrainError;

use super::ListDirectoryDriver;

const DEFAULT_DEPTH: u32 = 3;
const IGNORED_NAMES: &[&str] = &[".git", "node_modules", "target"];

pub struct ListDirectoryDriverNative;

impl ListDirectoryDriver for ListDirectoryDriverNative {
    fn list_directory(
        &self,
        path: &str,
        depth: Option<u32>,
    ) -> BoxFuture<'_, Result<String, BrainError>> {
        let path = path.to_owned();
        let max_depth = depth.unwrap_or(DEFAULT_DEPTH);
        Box::pin(async move {
            let root = PathBuf::from(&path);
            let metadata =
                tokio::fs::metadata(&root)
                    .await
                    .map_err(|error| BrainError::ToolFailed {
                        tool: "list_directory".into(),
                        reason: format!("failed to read {}: {error}", root.display()),
                    })?;

            let mut lines = Vec::new();
            if metadata.is_dir() {
                lines.push(format!("{}/", display_name(&root)));
                render_directory(&root, 0, max_depth, &mut lines)?;
            } else {
                lines.push(display_name(&root));
            }

            Ok(lines.join("\n"))
        })
    }
}

fn render_directory(
    path: &Path,
    depth: u32,
    max_depth: u32,
    lines: &mut Vec<String>,
) -> Result<(), BrainError> {
    if depth >= max_depth {
        return Ok(());
    }

    let mut entries = std::fs::read_dir(path)
        .map_err(|error| BrainError::ToolFailed {
            tool: "list_directory".into(),
            reason: format!("failed to read {}: {error}", path.display()),
        })?
        .filter_map(|entry| entry.ok())
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
        let entry_path = entry.path();
        let file_type = entry.file_type().map_err(|error| BrainError::ToolFailed {
            tool: "list_directory".into(),
            reason: format!("failed to inspect {}: {error}", entry_path.display()),
        })?;
        let indent = "  ".repeat((depth + 1) as usize);
        let name = entry.file_name().to_string_lossy().to_string();

        if file_type.is_dir() {
            lines.push(format!("{indent}{name}/"));
            render_directory(&entry_path, depth + 1, max_depth, lines)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn lists_directory_tree_and_ignores_common_build_dirs() {
        let dir = tempdir().unwrap();
        tokio::fs::create_dir_all(dir.path().join("src/nested"))
            .await
            .unwrap();
        tokio::fs::create_dir_all(dir.path().join("target/debug"))
            .await
            .unwrap();
        tokio::fs::write(dir.path().join("Cargo.toml"), "")
            .await
            .unwrap();
        tokio::fs::write(dir.path().join("src/lib.rs"), "")
            .await
            .unwrap();
        tokio::fs::write(dir.path().join("src/nested/mod.rs"), "")
            .await
            .unwrap();

        let output = ListDirectoryDriverNative
            .list_directory(dir.path().to_str().unwrap(), Some(4))
            .await
            .unwrap();

        assert!(output.contains("src/"));
        assert!(output.contains("lib.rs"));
        assert!(output.contains("nested/"));
        assert!(output.contains("Cargo.toml"));
        assert!(!output.contains("target/"));
    }
}
