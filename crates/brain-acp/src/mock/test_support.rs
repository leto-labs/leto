use std::{cell::RefCell, collections::HashMap, path::PathBuf, rc::Rc};

use agent_client_protocol::{self as acp, Agent as _};
use tokio::io::split;
use tokio::sync::mpsc;
use tokio_util::compat::{TokioAsyncReadCompatExt as _, TokioAsyncWriteCompatExt as _};

use super::agent::MockAgent;
use super::runtime::spawn_notification_forwarder;

#[derive(Clone)]
pub(super) struct RecordingClient {
    notifications: Rc<RefCell<Vec<acp::SessionNotification>>>,
    files: Rc<RefCell<HashMap<PathBuf, String>>>,
    reads: Rc<RefCell<Vec<PathBuf>>>,
    writes: Rc<RefCell<Vec<(PathBuf, String)>>>,
    permission_outcome: Rc<RefCell<acp::RequestPermissionOutcome>>,
    terminals: Rc<RefCell<HashMap<acp::TerminalId, MockTerminal>>>,
    next_terminal_index: Rc<RefCell<u64>>,
    created_terminals: Rc<RefCell<Vec<(String, Vec<String>)>>>,
    released_terminals: Rc<RefCell<Vec<acp::TerminalId>>>,
    killed_terminals: Rc<RefCell<Vec<acp::TerminalId>>>,
}

impl Default for RecordingClient {
    fn default() -> Self {
        Self {
            notifications: Rc::new(RefCell::new(Vec::new())),
            files: Rc::new(RefCell::new(HashMap::new())),
            reads: Rc::new(RefCell::new(Vec::new())),
            writes: Rc::new(RefCell::new(Vec::new())),
            permission_outcome: Rc::new(RefCell::new(acp::RequestPermissionOutcome::Selected(
                acp::SelectedPermissionOutcome::new("allow-once"),
            ))),
            terminals: Rc::new(RefCell::new(HashMap::new())),
            next_terminal_index: Rc::new(RefCell::new(1)),
            created_terminals: Rc::new(RefCell::new(Vec::new())),
            released_terminals: Rc::new(RefCell::new(Vec::new())),
            killed_terminals: Rc::new(RefCell::new(Vec::new())),
        }
    }
}

#[derive(Clone)]
struct MockTerminal {
    output: String,
    exit_status: acp::TerminalExitStatus,
}

impl RecordingClient {
    pub(super) fn take_notifications(&self) -> Vec<acp::SessionNotification> {
        std::mem::take(&mut *self.notifications.borrow_mut())
    }

    pub(super) fn with_file(self, path: impl Into<PathBuf>, content: impl Into<String>) -> Self {
        self.files.borrow_mut().insert(path.into(), content.into());
        self
    }

    pub(super) fn with_permission_outcome(self, outcome: acp::RequestPermissionOutcome) -> Self {
        *self.permission_outcome.borrow_mut() = outcome;
        self
    }

    pub(super) fn writes(&self) -> Vec<(PathBuf, String)> {
        self.writes.borrow().clone()
    }

    pub(super) fn reads(&self) -> Vec<PathBuf> {
        self.reads.borrow().clone()
    }

    pub(super) fn created_terminals(&self) -> Vec<(String, Vec<String>)> {
        self.created_terminals.borrow().clone()
    }

    pub(super) fn released_terminals(&self) -> Vec<acp::TerminalId> {
        self.released_terminals.borrow().clone()
    }

    pub(super) fn killed_terminals(&self) -> Vec<acp::TerminalId> {
        self.killed_terminals.borrow().clone()
    }
}

#[async_trait::async_trait(?Send)]
impl acp::Client for RecordingClient {
    async fn request_permission(
        &self,
        _args: acp::RequestPermissionRequest,
    ) -> Result<acp::RequestPermissionResponse, acp::Error> {
        Ok(acp::RequestPermissionResponse::new(
            self.permission_outcome.borrow().clone(),
        ))
    }

    async fn write_text_file(
        &self,
        args: acp::WriteTextFileRequest,
    ) -> Result<acp::WriteTextFileResponse, acp::Error> {
        self.files
            .borrow_mut()
            .insert(args.path.clone(), args.content.clone());
        self.writes.borrow_mut().push((args.path, args.content));
        Ok(acp::WriteTextFileResponse::new())
    }

    async fn read_text_file(
        &self,
        args: acp::ReadTextFileRequest,
    ) -> Result<acp::ReadTextFileResponse, acp::Error> {
        self.reads.borrow_mut().push(args.path.clone());
        let content = self
            .files
            .borrow()
            .get(&args.path)
            .cloned()
            .ok_or_else(acp::Error::invalid_params)?;
        Ok(acp::ReadTextFileResponse::new(content))
    }

    async fn create_terminal(
        &self,
        args: acp::CreateTerminalRequest,
    ) -> Result<acp::CreateTerminalResponse, acp::Error> {
        self.created_terminals
            .borrow_mut()
            .push((args.command.clone(), args.args.clone()));

        let mut next_terminal_index = self.next_terminal_index.borrow_mut();
        let terminal_id = acp::TerminalId::new(format!("mock-terminal-{}", *next_terminal_index));
        *next_terminal_index += 1;

        let joined_args = if args.args.is_empty() {
            String::new()
        } else {
            format!(" {}", args.args.join(" "))
        };
        let output = format!("mock terminal output: {}{}\n", args.command, joined_args);
        let terminal = MockTerminal {
            output,
            exit_status: acp::TerminalExitStatus::new().exit_code(0),
        };
        self.terminals
            .borrow_mut()
            .insert(terminal_id.clone(), terminal);

        Ok(acp::CreateTerminalResponse::new(terminal_id))
    }

    async fn terminal_output(
        &self,
        args: acp::TerminalOutputRequest,
    ) -> Result<acp::TerminalOutputResponse, acp::Error> {
        let terminal = self
            .terminals
            .borrow()
            .get(&args.terminal_id)
            .cloned()
            .ok_or_else(acp::Error::invalid_params)?;
        Ok(acp::TerminalOutputResponse::new(terminal.output, false)
            .exit_status(terminal.exit_status))
    }

    async fn release_terminal(
        &self,
        args: acp::ReleaseTerminalRequest,
    ) -> Result<acp::ReleaseTerminalResponse, acp::Error> {
        self.released_terminals
            .borrow_mut()
            .push(args.terminal_id.clone());
        self.terminals.borrow_mut().remove(&args.terminal_id);
        Ok(acp::ReleaseTerminalResponse::new())
    }

    async fn wait_for_terminal_exit(
        &self,
        args: acp::WaitForTerminalExitRequest,
    ) -> Result<acp::WaitForTerminalExitResponse, acp::Error> {
        let terminal = self
            .terminals
            .borrow()
            .get(&args.terminal_id)
            .cloned()
            .ok_or_else(acp::Error::invalid_params)?;
        Ok(acp::WaitForTerminalExitResponse::new(terminal.exit_status))
    }

    async fn kill_terminal(
        &self,
        args: acp::KillTerminalRequest,
    ) -> Result<acp::KillTerminalResponse, acp::Error> {
        let mut terminals = self.terminals.borrow_mut();
        let terminal = terminals
            .get_mut(&args.terminal_id)
            .ok_or_else(acp::Error::invalid_params)?;
        terminal.exit_status = acp::TerminalExitStatus::new().signal("SIGKILL");
        terminal.output.push_str("killed by mock client\n");
        self.killed_terminals.borrow_mut().push(args.terminal_id);
        Ok(acp::KillTerminalResponse::new())
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

pub(super) fn start_test_connection(client: RecordingClient) -> Rc<acp::ClientSideConnection> {
    let (client_stream, agent_stream) = tokio::io::duplex(8192);
    let (client_read, client_write) = split(client_stream);
    let (agent_read, agent_write) = split(agent_stream);

    let (session_update_tx, session_update_rx) = mpsc::unbounded_channel();
    let agent = MockAgent::new(session_update_tx);
    let agent_handle = agent.clone();

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
    let agent_conn = Rc::new(agent_conn);
    agent_handle.set_client_connection(agent_conn.clone());
    spawn_notification_forwarder(session_update_rx, agent_conn);

    tokio::task::spawn_local(async move {
        let _ = client_io.await;
    });
    tokio::task::spawn_local(async move {
        let _ = agent_io.await;
    });

    Rc::new(client_conn)
}

pub(super) async fn initialized_connection(
    client: RecordingClient,
) -> (RecordingClient, Rc<acp::ClientSideConnection>) {
    let connection = start_test_connection(client.clone());
    connection
        .initialize(
            acp::InitializeRequest::new(acp::ProtocolVersion::V1)
                .client_info(acp::Implementation::new("test-client", "0.1.0").title("Test Client")),
        )
        .await
        .expect("initialize should succeed");
    (client, connection)
}

pub(super) fn streamed_agent_text(notifications: &[acp::SessionNotification]) -> String {
    notifications
        .iter()
        .filter_map(|notification| match &notification.update {
            acp::SessionUpdate::AgentMessageChunk(acp::ContentChunk {
                content: acp::ContentBlock::Text(text),
                ..
            }) => Some(text.text.clone()),
            _ => None,
        })
        .collect::<String>()
}

pub(super) fn streamed_agent_thoughts(notifications: &[acp::SessionNotification]) -> String {
    notifications
        .iter()
        .filter_map(|notification| match &notification.update {
            acp::SessionUpdate::AgentThoughtChunk(acp::ContentChunk {
                content: acp::ContentBlock::Text(text),
                ..
            }) => Some(text.text.clone()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn completed_tool_output(notifications: &[acp::SessionNotification]) -> String {
    notifications
        .iter()
        .filter_map(|notification| match &notification.update {
            acp::SessionUpdate::ToolCallUpdate(update)
                if update.fields.status == Some(acp::ToolCallStatus::Completed) =>
            {
                let text = update
                    .fields
                    .content
                    .as_ref()
                    .map(|content| {
                        content
                            .iter()
                            .filter_map(|item| match item {
                                acp::ToolCallContent::Content(acp::Content {
                                    content: acp::ContentBlock::Text(text),
                                    ..
                                }) => Some(text.text.clone()),
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .unwrap_or_default();
                (!text.is_empty()).then_some(text)
            }
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}
