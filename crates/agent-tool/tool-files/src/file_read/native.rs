use agent_tool::ToolError;
use futures::future::BoxFuture;

use super::{FileReadDriver, format_file_read_output};

/// Native filesystem-backed file reader.
pub struct NativeFileReadDriver;

impl FileReadDriver for NativeFileReadDriver {
    fn read_file(
        &self,
        path: &str,
        offset: Option<usize>,
        limit: Option<usize>,
    ) -> BoxFuture<'_, Result<String, ToolError>> {
        let path = path.to_owned();
        Box::pin(async move {
            let content = tokio::fs::read_to_string(&path)
                .await
                .map_err(|error| ToolError::new(format!("file_read failed for {path}: {error}")))?;
            Ok(format_file_read_output(&content, offset, limit))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::NativeFileReadDriver;
    use crate::file_read::FileReadDriver;
    use tempfile::tempdir;

    #[tokio::test]
    async fn reads_file_with_line_numbers() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("notes.txt");
        tokio::fs::write(&path, "first\nsecond\n")
            .await
            .expect("write file");

        let output = NativeFileReadDriver
            .read_file(path.to_str().expect("path"), None, None)
            .await
            .expect("read file");
        assert!(output.contains("1|first"));
        assert!(output.contains("2|second"));
    }
}
