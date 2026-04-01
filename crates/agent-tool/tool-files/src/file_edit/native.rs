use agent_tool::ToolError;
use futures::future::BoxFuture;

use super::FileEditDriver;

/// Native filesystem-backed exact-string file editor.
pub struct NativeFileEditDriver;

impl FileEditDriver for NativeFileEditDriver {
    fn edit_file(
        &self,
        path: &str,
        old_string: &str,
        new_string: &str,
    ) -> BoxFuture<'_, Result<String, ToolError>> {
        let path = path.to_owned();
        let old_string = old_string.to_owned();
        let new_string = new_string.to_owned();
        Box::pin(async move {
            let content = tokio::fs::read_to_string(&path)
                .await
                .map_err(|error| ToolError::new(format!("file_edit failed for {path}: {error}")))?;

            let count = content.matches(&old_string).count();
            if count == 0 {
                return Err(ToolError::new("old_string not found in file"));
            }
            if count > 1 {
                return Err(ToolError::new(format!(
                    "old_string found {count} times — must be unique. Add more context to make it unique."
                )));
            }

            let updated = content.replacen(&old_string, &new_string, 1);
            tokio::fs::write(&path, updated).await.map_err(|error| {
                ToolError::new(format!("file_edit failed to write {path}: {error}"))
            })?;

            Ok(format!("Edited {path}"))
        })
    }
}
