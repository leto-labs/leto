//! ACP stdio server.

use std::rc::Rc;

use agent_client_protocol::{self as acp, Client as _};
use agent_core::AgentCoreNative;
use futures::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc;
use tokio_util::compat::{TokioAsyncReadCompatExt as _, TokioAsyncWriteCompatExt as _};

use crate::AgentCoreAcpBackend;
use crate::file_bridge::AcpFileBridge;

/// Runs the ACP stdio server with the given core.
pub async fn run_stdio(core: AgentCoreNative) -> Result<(), acp::Error> {
    run_connection(
        core,
        tokio::io::stdin().compat(),
        tokio::io::stdout().compat_write(),
    )
    .await
}

async fn run_connection(
    core: AgentCoreNative,
    incoming: impl AsyncRead + Unpin + 'static,
    outgoing: impl AsyncWrite + Unpin + 'static,
) -> Result<(), acp::Error> {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async move {
            let (session_update_tx, session_update_rx) = mpsc::unbounded_channel();
            let file_bridge = AcpFileBridge::new();
            let backend =
                AgentCoreAcpBackend::with_file_bridge(core, file_bridge.clone(), session_update_tx);

            let (connection, handle_io) =
                acp::AgentSideConnection::new(backend, outgoing, incoming, |future| {
                    tokio::task::spawn_local(future);
                });
            let connection = Rc::new(connection);
            spawn_notification_forwarder(session_update_rx, connection.clone());
            file_bridge.spawn_local_client_runner(connection.clone());
            handle_io.await
        })
        .await
}

pub(crate) fn spawn_notification_forwarder(
    mut session_update_rx: mpsc::UnboundedReceiver<crate::adapter::NotificationEnvelope>,
    connection: Rc<acp::AgentSideConnection>,
) {
    tokio::task::spawn_local(async move {
        while let Some((notification, ack)) = session_update_rx.recv().await {
            if connection.session_notification(notification).await.is_err() {
                break;
            }
            let _ = ack.send(());
        }
    });
}
