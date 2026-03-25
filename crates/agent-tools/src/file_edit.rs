use agent_runtime::RuntimeError;
use futures::future::BoxFuture;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::registry::TypedTool;

/// Edits a file by replacing a unique exact string match.
pub struct FileEditTool<D: FileEditDriver> {
    driver: D,
}

impl<D: FileEditDriver> FileEditTool<D> {
    /// Creates a file-edit tool backed by the provided driver.
    pub fn new(driver: D) -> Self {
        Self { driver }
    }
}

/// Semantic driver for exact-string file edits.
pub trait FileEditDriver: Send + Sync + 'static {
    /// Replaces an exact string match in the target file.
    fn edit_file(
        &self,
        path: &str,
        old_string: &str,
        new_string: &str,
    ) -> BoxFuture<'_, Result<String, RuntimeError>>;
}

/// Request payload for the `file_edit` tool.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FileEditRequest {
    /// The file path to edit.
    pub path: String,
    /// The exact text to find and replace. It must appear exactly once.
    pub old_string: String,
    /// The replacement text.
    pub new_string: String,
}

/// Summary returned after editing a file.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(transparent)]
#[schemars(description = "Summary of the completed file edit operation.")]
pub struct FileEditResponse(pub String);

impl<D: FileEditDriver> TypedTool for FileEditTool<D> {
    type Request = FileEditRequest;
    type Response = FileEditResponse;

    fn name(&self) -> &'static str {
        "file_edit"
    }

    fn description(&self) -> &'static str {
        "Edit a file by replacing an exact string match with new content. The old_string must appear exactly once in the file."
    }

    fn execute_typed<'a>(
        &'a self,
        request: Self::Request,
    ) -> BoxFuture<'a, Result<Self::Response, RuntimeError>> {
        Box::pin(async move {
            let output = self
                .driver
                .edit_file(&request.path, &request.old_string, &request.new_string)
                .await?;
            Ok(FileEditResponse(output))
        })
    }
}

/// Native filesystem-backed exact-string file editor.
pub struct NativeFileEditDriver;

impl FileEditDriver for NativeFileEditDriver {
    fn edit_file(
        &self,
        path: &str,
        old_string: &str,
        new_string: &str,
    ) -> BoxFuture<'_, Result<String, RuntimeError>> {
        let path = path.to_owned();
        let old_string = old_string.to_owned();
        let new_string = new_string.to_owned();
        Box::pin(async move {
            let content = tokio::fs::read_to_string(&path).await.map_err(|error| {
                RuntimeError::Tool(format!("file_edit failed for {path}: {error}"))
            })?;

            let count = content.matches(&old_string).count();
            if count == 0 {
                return Err(RuntimeError::Tool("old_string not found in file".into()));
            }
            if count > 1 {
                return Err(RuntimeError::Tool(format!(
                    "old_string found {count} times — must be unique. Add more context to make it unique."
                )));
            }

            let updated = content.replacen(&old_string, &new_string, 1);
            tokio::fs::write(&path, updated).await.map_err(|error| {
                RuntimeError::Tool(format!("file_edit failed to write {path}: {error}"))
            })?;

            Ok(format!("Edited {path}"))
        })
    }
}
