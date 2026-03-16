use futures::future::BoxFuture;

use brain_types::BrainError;

use super::FileReadDriver;

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
            let content = tokio::fs::read_to_string(&path)
                .await
                .map_err(|e| BrainError::ToolFailed {
                    tool: "file_read".into(),
                    reason: format!("{e}"),
                })?;

            let lines: Vec<&str> = content.lines().collect();
            let start = offset.map(|o| o.saturating_sub(1)).unwrap_or(0);
            let end = limit
                .map(|l| (start + l).min(lines.len()))
                .unwrap_or(lines.len());

            if start >= lines.len() {
                return Ok(String::new());
            }

            let formatted = lines[start..end]
                .iter()
                .enumerate()
                .map(|(i, line)| format!("{:>6}|{}", start + i + 1, line))
                .collect::<Vec<_>>()
                .join("\n");

            Ok(formatted)
        })
    }
}
