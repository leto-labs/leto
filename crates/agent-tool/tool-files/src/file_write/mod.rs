use agent_tool::{ToolError, TypedTool};
use futures::future::BoxFuture;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[cfg(feature = "component")]
pub(crate) mod component;
#[cfg(feature = "native")]
pub mod native;

/// Creates or overwrites a file with the given content.
pub struct FileWriteTool<D: FileWriteDriver> {
    driver: D,
}

impl<D: FileWriteDriver> FileWriteTool<D> {
    /// Creates a file-write tool backed by the provided driver.
    pub fn new(driver: D) -> Self {
        Self { driver }
    }
}

/// Semantic driver for writing file contents.
pub trait FileWriteDriver: Send + Sync + 'static {
    /// Writes the given content to the target path.
    fn write_file(&self, path: &str, content: &str) -> BoxFuture<'_, Result<String, ToolError>>;
}

/// Request payload for the `file_write` tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FileWriteRequest {
    /// The file path to write.
    pub path: String,
    /// The content to write to the file.
    pub content: String,
}

/// Summary returned after writing a file.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
#[schemars(description = "Summary of the completed file write operation.")]
pub struct FileWriteResponse(pub String);

impl<D: FileWriteDriver> TypedTool for FileWriteTool<D> {
    type Request = FileWriteRequest;
    type Response = FileWriteResponse;

    fn name(&self) -> &'static str {
        "file_write"
    }

    fn description(&self) -> &'static str {
        "Create or overwrite a file with the given content."
    }

    fn execute_typed<'a>(
        &'a self,
        request: Self::Request,
    ) -> BoxFuture<'a, Result<Self::Response, ToolError>> {
        Box::pin(async move {
            let output = self
                .driver
                .write_file(&request.path, &request.content)
                .await?;
            Ok(FileWriteResponse(output))
        })
    }
}
