use std::rc::Rc;

use agent_client_protocol::{self as acp, Client as _};
use futures::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc;
use tokio_util::compat::{TokioAsyncReadCompatExt as _, TokioAsyncWriteCompatExt as _};

use super::agent::{MockAgent, NotificationEnvelope};

pub async fn run_stdio() -> Result<(), acp::Error> {
    run_connection(
        tokio::io::stdin().compat(),
        tokio::io::stdout().compat_write(),
    )
    .await
}

async fn run_connection(
    incoming: impl AsyncRead + Unpin + 'static,
    outgoing: impl AsyncWrite + Unpin + 'static,
) -> Result<(), acp::Error> {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async move {
            let (session_update_tx, session_update_rx) = mpsc::unbounded_channel();
            let agent = MockAgent::new(session_update_tx);
            let agent_handle = agent.clone();

            let (connection, handle_io) =
                acp::AgentSideConnection::new(agent, outgoing, incoming, |future| {
                    tokio::task::spawn_local(future);
                });
            let connection = Rc::new(connection);
            agent_handle.set_client_connection(connection.clone());
            spawn_notification_forwarder(session_update_rx, connection);

            handle_io.await
        })
        .await
}

pub(super) fn spawn_notification_forwarder(
    mut session_update_rx: mpsc::UnboundedReceiver<NotificationEnvelope>,
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
