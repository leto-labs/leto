use futures::future::BoxFuture;
use tokio::sync::oneshot;

use brain_types::BrainError;

use super::FileWriteDriver;
use crate::acp::{AcpClientRequest, active_acp_client_context, resolve_acp_client_path};

pub struct FileWriteDriverAcp;

impl FileWriteDriver for FileWriteDriverAcp {
    fn write_file(&self, path: &str, content: &str) -> BoxFuture<'_, Result<String, BrainError>> {
        let path = path.to_owned();
        let content = content.to_owned();
        let context = match active_acp_client_context("file_write") {
            Ok(context) => context,
            Err(error) => return Box::pin(async move { Err(error) }),
        };
        let resolved = resolve_acp_client_path(&context.cwd, &path);
        let (tx, rx) = oneshot::channel();
        if let Err(error) = context.handle.send(AcpClientRequest::WriteTextFile {
            session_id: context.session_id,
            path: resolved.clone(),
            content: content.clone(),
            response_tx: tx,
        }) {
            return Box::pin(async move {
                Err(BrainError::ToolFailed {
                    tool: "file_write".into(),
                    reason: format!("ACP write_text_file bridge is unavailable: {error}"),
                })
            });
        }

        Box::pin(async move {
            let write_result = rx.await.map_err(|_| BrainError::ToolFailed {
                tool: "file_write".into(),
                reason: "ACP write_text_file task was cancelled".into(),
            })?;
            write_result.map_err(|reason| BrainError::ToolFailed {
                tool: "file_write".into(),
                reason: format!("ACP write_text_file failed: {reason}"),
            })?;

            let line_count = content.lines().count();
            Ok(format!(
                "Wrote {line_count} lines to {}",
                resolved.display()
            ))
        })
    }
}
