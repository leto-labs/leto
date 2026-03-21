use futures::future::BoxFuture;

use brain_types::BrainError;

use super::{FileReadDriver, format_file_read_output};

pub struct FileReadDriverNative;

impl FileReadDriver for FileReadDriverNative {
    fn read_file(
        &self,
        path: &str,
        offset: Option<usize>,
        limit: Option<usize>,
    ) -> BoxFuture<'_, Result<String, BrainError>> {
        let path = path.to_owned();
        Box::pin(async move {
            let content =
                tokio::fs::read_to_string(&path)
                    .await
                    .map_err(|e| BrainError::ToolFailed {
                        tool: "file_read".into(),
                        reason: format!("{e}"),
                    })?;
            Ok(format_file_read_output(&content, offset, limit))
        })
    }
}
