use futures::future::BoxFuture;
use tokio::sync::oneshot;

use brain_types::BrainError;

use super::{FileReadDriver, format_file_read_output};
use crate::acp::{AcpClientRequest, active_acp_client_context, resolve_acp_client_path};

pub struct FileReadDriverAcp;

impl FileReadDriver for FileReadDriverAcp {
    fn read_file(
        &self,
        path: &str,
        offset: Option<usize>,
        limit: Option<usize>,
    ) -> BoxFuture<'_, Result<String, BrainError>> {
        let path = path.to_owned();
        let context = match active_acp_client_context("file_read") {
            Ok(context) => context,
            Err(error) => return Box::pin(async move { Err(error) }),
        };
        let resolved = resolve_acp_client_path(&context.cwd, &path);
        let (tx, rx) = oneshot::channel();
        if let Err(error) = context.handle.send(AcpClientRequest::ReadTextFile {
            session_id: context.session_id,
            path: resolved.clone(),
            response_tx: tx,
        }) {
            return Box::pin(async move {
                Err(BrainError::ToolFailed {
                    tool: "file_read".into(),
                    reason: format!("ACP read_text_file bridge is unavailable: {error}"),
                })
            });
        }

        Box::pin(async move {
            let read_result = rx.await.map_err(|_| BrainError::ToolFailed {
                tool: "file_read".into(),
                reason: "ACP read_text_file task was cancelled".into(),
            })?;
            let content = read_result.map_err(|reason| BrainError::ToolFailed {
                tool: "file_read".into(),
                reason: format!("ACP read_text_file failed: {reason}"),
            })?;
            Ok(format_file_read_output(&content, offset, limit))
        })
    }
}
