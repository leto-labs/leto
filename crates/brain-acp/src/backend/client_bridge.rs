use std::rc::Rc;

use agent_client_protocol::{self as acp, Client as _};
use brain_tools::AcpClientRequest;
use tokio::sync::mpsc;

// This module does not define tools. It owns the live ACP connection and
// forwards tool-originated client requests from brain-tools into real ACP
// client RPCs.
pub(super) fn spawn_client_request_forwarder(
    mut request_rx: mpsc::UnboundedReceiver<AcpClientRequest>,
    connection: Rc<acp::AgentSideConnection>,
) {
    tokio::task::spawn_local(async move {
        while let Some(request) = request_rx.recv().await {
            match request {
                AcpClientRequest::WriteTextFile {
                    session_id,
                    path,
                    content,
                    response_tx,
                } => {
                    let result = connection
                        .write_text_file(acp::WriteTextFileRequest::new(
                            session_id, &path, &content,
                        ))
                        .await
                        .map(|_| ())
                        .map_err(|error| error.to_string());
                    let _ = response_tx.send(result);
                }
                AcpClientRequest::ReadTextFile {
                    session_id,
                    path,
                    response_tx,
                } => {
                    let result = connection
                        .read_text_file(acp::ReadTextFileRequest::new(session_id, &path))
                        .await
                        .map(|response| response.content)
                        .map_err(|error| error.to_string());
                    let _ = response_tx.send(result);
                }
            }
        }
    });
}
