use std::path::Path;

use futures::future::BoxFuture;

use brain_types::BrainError;

use super::FileWriteDriver;

pub struct FileWriteDriverNative;

impl FileWriteDriver for FileWriteDriverNative {
    fn write_file(
        &self,
        path: &str,
        content: &str,
    ) -> BoxFuture<'_, Result<String, BrainError>> {
        let path = path.to_owned();
        let content = content.to_owned();
        Box::pin(async move {
            if let Some(parent) = Path::new(&path).parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| BrainError::ToolFailed {
                        tool: "file_write".into(),
                        reason: format!("failed to create directories: {e}"),
                    })?;
            }

            tokio::fs::write(&path, &content)
                .await
                .map_err(|e| BrainError::ToolFailed {
                    tool: "file_write".into(),
                    reason: format!("{e}"),
                })?;

            let line_count = content.lines().count();
            Ok(format!("Wrote {line_count} lines to {path}"))
        })
    }
}
