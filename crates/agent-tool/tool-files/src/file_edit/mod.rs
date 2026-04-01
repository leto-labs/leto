use agent_tool::{ToolError, TypedTool};
use futures::future::BoxFuture;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[cfg(feature = "component")]
pub(crate) mod component;
#[cfg(feature = "native")]
pub mod native;

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
    ) -> BoxFuture<'_, Result<String, ToolError>>;
}

/// Request payload for the `file_edit` tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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
    ) -> BoxFuture<'a, Result<Self::Response, ToolError>> {
        Box::pin(async move {
            let output = self
                .driver
                .edit_file(&request.path, &request.old_string, &request.new_string)
                .await?;
            Ok(FileEditResponse(output))
        })
    }
}
