use std::{cell::RefCell, rc::Rc, sync::Arc};

use agent_client_protocol::{self as acp, Agent as _};
use tokio::io::split;
use tokio::sync::mpsc;
use tokio_util::compat::{TokioAsyncReadCompatExt as _, TokioAsyncWriteCompatExt as _};

use brain_core::{
    BrainRuntime, BrainRuntimeNative, InMemoryStore, MockProvider, SimpleLoop, Store,
};

use super::agent::BackendAgent;
use super::app::BackendApp;
use super::stdio::spawn_notification_forwarder;

#[derive(Clone, Default)]
pub(super) struct RecordingClient {
    notifications: Rc<RefCell<Vec<acp::SessionNotification>>>,
}

impl RecordingClient {
    pub(super) fn take_notifications(&self) -> Vec<acp::SessionNotification> {
        std::mem::take(&mut *self.notifications.borrow_mut())
    }
}

#[async_trait::async_trait(?Send)]
impl acp::Client for RecordingClient {
    async fn request_permission(
        &self,
        _args: acp::RequestPermissionRequest,
    ) -> Result<acp::RequestPermissionResponse, acp::Error> {
        Err(acp::Error::method_not_found())
    }

    async fn write_text_file(
        &self,
        _args: acp::WriteTextFileRequest,
    ) -> Result<acp::WriteTextFileResponse, acp::Error> {
        Err(acp::Error::method_not_found())
    }

    async fn read_text_file(
        &self,
        _args: acp::ReadTextFileRequest,
    ) -> Result<acp::ReadTextFileResponse, acp::Error> {
        Err(acp::Error::method_not_found())
    }

    async fn create_terminal(
        &self,
        _args: acp::CreateTerminalRequest,
    ) -> Result<acp::CreateTerminalResponse, acp::Error> {
        Err(acp::Error::method_not_found())
    }

    async fn terminal_output(
        &self,
        _args: acp::TerminalOutputRequest,
    ) -> Result<acp::TerminalOutputResponse, acp::Error> {
        Err(acp::Error::method_not_found())
    }

    async fn release_terminal(
        &self,
        _args: acp::ReleaseTerminalRequest,
    ) -> Result<acp::ReleaseTerminalResponse, acp::Error> {
        Err(acp::Error::method_not_found())
    }

    async fn wait_for_terminal_exit(
        &self,
        _args: acp::WaitForTerminalExitRequest,
    ) -> Result<acp::WaitForTerminalExitResponse, acp::Error> {
        Err(acp::Error::method_not_found())
    }

    async fn kill_terminal(
        &self,
        _args: acp::KillTerminalRequest,
    ) -> Result<acp::KillTerminalResponse, acp::Error> {
        Err(acp::Error::method_not_found())
    }

    async fn session_notification(&self, args: acp::SessionNotification) -> Result<(), acp::Error> {
        self.notifications.borrow_mut().push(args);
        Ok(())
    }

    async fn ext_method(&self, _args: acp::ExtRequest) -> Result<acp::ExtResponse, acp::Error> {
        Err(acp::Error::method_not_found())
    }

    async fn ext_notification(&self, _args: acp::ExtNotification) -> Result<(), acp::Error> {
        Err(acp::Error::method_not_found())
    }
}

pub(super) fn build_test_app(provider_delay_ms: u64) -> Arc<BackendApp> {
    build_test_app_with_loops(provider_delay_ms, &["simple"])
}

pub(super) fn build_test_app_with_loops(
    provider_delay_ms: u64,
    loop_names: &[&str],
) -> Arc<BackendApp> {
    let store: Arc<dyn Store> = Arc::new(InMemoryStore::new());
    let runtime = Arc::new(BrainRuntimeNative::new(store, "mock", "simple"));
    runtime
        .set_provider(
            "mock",
            Arc::new(MockProvider::new().with_delay(provider_delay_ms)),
        )
        .expect("mock provider should register");
    for loop_name in loop_names {
        runtime
            .set_loop((*loop_name).to_owned(), Arc::new(SimpleLoop))
            .expect("loop should register");
    }
    let runtime: Arc<dyn BrainRuntime> = runtime;
    Arc::new(BackendApp::new(runtime))
}

pub(super) fn start_test_connection(
    client: RecordingClient,
    app: Arc<BackendApp>,
) -> Rc<acp::ClientSideConnection> {
    let (client_stream, agent_stream) = tokio::io::duplex(8192);
    let (client_read, client_write) = split(client_stream);
    let (agent_read, agent_write) = split(agent_stream);

    let (session_update_tx, session_update_rx) = mpsc::unbounded_channel();
    let agent = BackendAgent::new(app, session_update_tx);

    let (client_conn, client_io) = acp::ClientSideConnection::new(
        client,
        client_write.compat_write(),
        client_read.compat(),
        |future| {
            tokio::task::spawn_local(future);
        },
    );
    let (agent_conn, agent_io) = acp::AgentSideConnection::new(
        agent,
        agent_write.compat_write(),
        agent_read.compat(),
        |future| {
            tokio::task::spawn_local(future);
        },
    );

    let client_conn = Rc::new(client_conn);
    let agent_conn = Rc::new(agent_conn);

    spawn_notification_forwarder(session_update_rx, agent_conn);

    tokio::task::spawn_local(async move {
        let _ = tokio::join!(client_io, agent_io);
    });

    client_conn
}

pub(super) async fn initialized_connection(
    client: RecordingClient,
    app: Arc<BackendApp>,
) -> (RecordingClient, Rc<acp::ClientSideConnection>) {
    let connection = start_test_connection(client.clone(), app);
    connection
        .initialize(acp::InitializeRequest::new(acp::ProtocolVersion::V1))
        .await
        .expect("initialize should succeed");
    (client, connection)
}

pub(super) fn streamed_agent_text(notifications: &[acp::SessionNotification]) -> String {
    notifications
        .iter()
        .filter_map(|notification| match &notification.update {
            acp::SessionUpdate::AgentMessageChunk(chunk) => match &chunk.content {
                acp::ContentBlock::Text(content) => Some(content.text.clone()),
                _ => None,
            },
            _ => None,
        })
        .collect::<String>()
}
