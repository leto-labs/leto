//! ACP stdio server.

use std::rc::Rc;

use agent_client_protocol::{self as acp, Client as _};
use futures::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc;
use tokio_util::compat::{TokioAsyncReadCompatExt as _, TokioAsyncWriteCompatExt as _};

use crate::AgentCoreAcpBackend;

/// Runs the ACP stdio server with the given core.
pub async fn run_stdio<C>(core: C) -> Result<(), acp::Error>
where
    C: agent_core::AgentCore + Send + Sync + 'static,
{
    run_connection(
        core,
        tokio::io::stdin().compat(),
        tokio::io::stdout().compat_write(),
    )
    .await
}

async fn run_connection<C>(
    core: C,
    incoming: impl AsyncRead + Unpin + 'static,
    outgoing: impl AsyncWrite + Unpin + 'static,
) -> Result<(), acp::Error>
where
    C: agent_core::AgentCore + Send + Sync + 'static,
{
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async move {
            let (session_update_tx, session_update_rx) = mpsc::unbounded_channel();
            let backend = AgentCoreAcpBackend::with_notification_sender(core, session_update_tx);

            let (connection, handle_io) =
                acp::AgentSideConnection::new(backend, outgoing, incoming, |future| {
                    tokio::task::spawn_local(future);
                });
            let connection = Rc::new(connection);
            spawn_notification_forwarder(session_update_rx, connection);
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
