use std::path::Path;

use agent_tool::ToolError;
use futures::future::BoxFuture;

use super::FileWriteDriver;

/// Native filesystem-backed file writer.
pub struct NativeFileWriteDriver;

impl FileWriteDriver for NativeFileWriteDriver {
    fn write_file(&self, path: &str, content: &str) -> BoxFuture<'_, Result<String, ToolError>> {
        let path = path.to_owned();
        let content = content.to_owned();
        Box::pin(async move {
            if let Some(parent) = Path::new(&path).parent()
                && !parent.as_os_str().is_empty()
            {
                tokio::fs::create_dir_all(parent).await.map_err(|error| {
                    ToolError::new(format!(
                        "file_write failed to create parent directories for {path}: {error}"
                    ))
                })?;
            }

            tokio::fs::write(&path, &content).await.map_err(|error| {
                ToolError::new(format!("file_write failed for {path}: {error}"))
            })?;

            Ok(format!(
                "Wrote {} lines to {}",
                content.lines().count(),
                path
            ))
        })
    }
}
