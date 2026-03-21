use futures::future::BoxFuture;

use brain_types::BrainError;

use super::FileReadDriver;
use crate::truncation::truncate_line;

const DEFAULT_LIMIT: usize = 200;
const MAX_LIMIT: usize = 400;
const MAX_LINE_CHARS: usize = 500;

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

            let lines: Vec<&str> = content.lines().collect();
            let start = offset.map(|o| o.saturating_sub(1)).unwrap_or(0);
            let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT);
            let end = (start + limit).min(lines.len());

            if start >= lines.len() {
                return Ok("Requested offset is beyond the end of the file.".into());
            }

            let mut formatted = lines[start..end]
                .iter()
                .enumerate()
                .map(|(i, line)| {
                    let line = truncate_line(line, MAX_LINE_CHARS);
                    format!("{:>6}|{}", start + i + 1, line)
                })
                .collect::<Vec<_>>()
                .join("\n");

            if end < lines.len() {
                formatted.push_str(&format!(
                    "\n\n...[showing lines {}-{} of {}. Use offset={} to continue]...",
                    start + 1,
                    end,
                    lines.len(),
                    end + 1
                ));
            }

            Ok(formatted)
        })
    }
}
