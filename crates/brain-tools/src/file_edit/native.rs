use futures::future::BoxFuture;

use brain_types::BrainError;

use super::FileEditDriver;

pub struct FileEditDriverNative;

impl FileEditDriver for FileEditDriverNative {
    fn edit_file(
        &self,
        path: &str,
        old_string: &str,
        new_string: &str,
    ) -> BoxFuture<'_, Result<String, BrainError>> {
        let path = path.to_owned();
        let old_string = old_string.to_owned();
        let new_string = new_string.to_owned();
        Box::pin(async move {
            let content = tokio::fs::read_to_string(&path)
                .await
                .map_err(|e| BrainError::ToolFailed {
                    tool: "file_edit".into(),
                    reason: format!("{e}"),
                })?;

            let count = content.matches(&old_string).count();
            if count == 0 {
                return Err(BrainError::ToolFailed {
                    tool: "file_edit".into(),
                    reason: "old_string not found in file".into(),
                });
            }
            if count > 1 {
                return Err(BrainError::ToolFailed {
                    tool: "file_edit".into(),
                    reason: format!(
                        "old_string found {count} times — must be unique. Add more context to make it unique."
                    ),
                });
            }

            let updated = content.replacen(&old_string, &new_string, 1);
            tokio::fs::write(&path, &updated)
                .await
                .map_err(|e| BrainError::ToolFailed {
                    tool: "file_edit".into(),
                    reason: format!("{e}"),
                })?;

            Ok(format!("Edited {path}"))
        })
    }
}
