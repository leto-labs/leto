//! ACP-backed file bridge for the real backend.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use agent_client_protocol::{self as acp, Client as _};
use agent_runtime::{RuntimeError, ToolCall, ToolExecutionResult, ToolExecutor};
use agent_tools::{
    ApplyPatchTool, AutoGrepDriver, EchoTool, FileEditTool, GlobSearchTool, GrepTool,
    ListDirectoryTool, NativeApplyPatchDriver, NativeFileEditDriver, NativeGlobSearchDriver,
    NativeListDirectoryDriver, NativeShellDriver, RegistryToolExecutor, ShellTool, TypedTool,
};
use async_stream::stream;
use futures::{StreamExt, future::BoxFuture};
use provider::ToolDefinition;
use provider::{BlockDelta, BlockKind, Event, EventStream, Provider, ProviderInfo, Request};
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::{RwLock, mpsc, oneshot};

const ACP_SESSION_ID_FIELD: &str = "__agent_acp_session_id";
const DEFAULT_READ_LIMIT: usize = 200;
const MAX_READ_LIMIT: usize = 400;
const MAX_LINE_CHARS: usize = 500;

#[derive(Clone, Default)]
pub(crate) struct AcpFileBridge {
    state: Arc<RwLock<BridgeState>>,
    command_tx: Arc<Mutex<Option<mpsc::UnboundedSender<FileBridgeCommand>>>>,
}

#[derive(Default)]
struct BridgeState {
    fs_capabilities: FileSystemCapabilities,
    session_cwds: HashMap<String, PathBuf>,
}

#[derive(Clone, Copy, Default)]
struct FileSystemCapabilities {
    read_text_file: bool,
    write_text_file: bool,
}

enum FileBridgeCommand {
    Read {
        session_id: acp::SessionId,
        path: PathBuf,
        reply: oneshot::Sender<Result<String, acp::Error>>,
    },
    Write {
        session_id: acp::SessionId,
        path: PathBuf,
        content: String,
        reply: oneshot::Sender<Result<(), acp::Error>>,
    },
}

impl AcpFileBridge {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) async fn set_client_capabilities(&self, capabilities: &acp::ClientCapabilities) {
        let mut state = self.state.write().await;
        state.fs_capabilities = FileSystemCapabilities {
            read_text_file: capabilities.fs.read_text_file,
            write_text_file: capabilities.fs.write_text_file,
        };
    }

    pub(crate) async fn remember_session_cwd(&self, session_id: &acp::SessionId, cwd: &Path) {
        self.state
            .write()
            .await
            .session_cwds
            .insert(session_id.0.to_string(), cwd.to_path_buf());
    }

    pub(crate) fn spawn_local_client_runner(&self, connection: Rc<acp::AgentSideConnection>) {
        let (command_tx, mut command_rx) = mpsc::unbounded_channel();
        *self
            .command_tx
            .lock()
            .expect("bridge command mutex poisoned") = Some(command_tx);

        tokio::task::spawn_local(async move {
            while let Some(command) = command_rx.recv().await {
                match command {
                    FileBridgeCommand::Read {
                        session_id,
                        path,
                        reply,
                    } => {
                        let result = connection
                            .read_text_file(acp::ReadTextFileRequest::new(session_id, path))
                            .await
                            .map(|response| response.content);
                        let _ = reply.send(result);
                    }
                    FileBridgeCommand::Write {
                        session_id,
                        path,
                        content,
                        reply,
                    } => {
                        let result = connection
                            .write_text_file(acp::WriteTextFileRequest::new(
                                session_id, path, content,
                            ))
                            .await
                            .map(|_| ());
                        let _ = reply.send(result);
                    }
                }
            }
        });
    }

    async fn resolve_relative_path(
        &self,
        session_id: &str,
        path: &str,
        access: BridgeAccess,
    ) -> Option<PathBuf> {
        let candidate = Path::new(path);
        if candidate.is_absolute() {
            return None;
        }

        let state = self.state.read().await;
        let allowed = match access {
            BridgeAccess::Read => state.fs_capabilities.read_text_file,
            BridgeAccess::Write => state.fs_capabilities.write_text_file,
        };
        if !allowed {
            return None;
        }

        state
            .session_cwds
            .get(session_id)
            .map(|cwd| cwd.join(candidate))
    }

    async fn read_text_file(
        &self,
        session_id: &str,
        path: &str,
        offset: Option<usize>,
        limit: Option<usize>,
    ) -> Result<Option<String>, RuntimeError> {
        let Some(resolved_path) = self
            .resolve_relative_path(session_id, path, BridgeAccess::Read)
            .await
        else {
            return Ok(None);
        };

        let Some(command_tx) = self
            .command_tx
            .lock()
            .expect("bridge command mutex poisoned")
            .clone()
        else {
            return Err(RuntimeError::Tool(
                "ACP file bridge is unavailable for file_read".to_owned(),
            ));
        };

        let (reply_tx, reply_rx) = oneshot::channel();
        command_tx
            .send(FileBridgeCommand::Read {
                session_id: acp::SessionId::new(session_id.to_owned()),
                path: resolved_path.clone(),
                reply: reply_tx,
            })
            .map_err(|_| {
                RuntimeError::Tool("ACP file bridge failed to queue file_read request".to_owned())
            })?;

        let content = reply_rx
            .await
            .map_err(|_| RuntimeError::Tool("ACP file bridge dropped file_read reply".to_owned()))?
            .map_err(|error| RuntimeError::Tool(format!("ACP file_read bridge failed: {error}")))?;

        Ok(Some(format_file_read_output(&content, offset, limit)))
    }

    async fn write_text_file(
        &self,
        session_id: &str,
        path: &str,
        content: &str,
    ) -> Result<Option<String>, RuntimeError> {
        let Some(resolved_path) = self
            .resolve_relative_path(session_id, path, BridgeAccess::Write)
            .await
        else {
            return Ok(None);
        };

        let Some(command_tx) = self
            .command_tx
            .lock()
            .expect("bridge command mutex poisoned")
            .clone()
        else {
            return Err(RuntimeError::Tool(
                "ACP file bridge is unavailable for file_write".to_owned(),
            ));
        };

        let (reply_tx, reply_rx) = oneshot::channel();
        command_tx
            .send(FileBridgeCommand::Write {
                session_id: acp::SessionId::new(session_id.to_owned()),
                path: resolved_path,
                content: content.to_owned(),
                reply: reply_tx,
            })
            .map_err(|_| {
                RuntimeError::Tool("ACP file bridge failed to queue file_write request".to_owned())
            })?;

        reply_rx
            .await
            .map_err(|_| RuntimeError::Tool("ACP file bridge dropped file_write reply".to_owned()))?
            .map_err(|error| {
                RuntimeError::Tool(format!("ACP file_write bridge failed: {error}"))
            })?;

        Ok(Some(format!(
            "Wrote {} lines to {}",
            content.lines().count(),
            path
        )))
    }
}

#[derive(Clone, Copy)]
enum BridgeAccess {
    Read,
    Write,
}

pub(crate) fn wrap_provider(provider: Arc<dyn Provider>) -> Arc<dyn Provider> {
    Arc::new(AcpBridgeProvider { inner: provider })
}

pub(crate) fn tool_executor(bridge: AcpFileBridge) -> Arc<dyn ToolExecutor> {
    Arc::new(AcpBridgeToolExecutor::new(bridge))
}

pub(crate) fn sanitize_tool_input(raw_input: Value) -> Value {
    let mut raw_input = raw_input;
    if let Value::Object(object) = &mut raw_input {
        object.remove(ACP_SESSION_ID_FIELD);
    }
    raw_input
}

struct AcpBridgeProvider {
    inner: Arc<dyn Provider>,
}

impl Provider for AcpBridgeProvider {
    fn stream<'a>(
        &'a self,
        request: &'a Request,
    ) -> BoxFuture<'a, Result<EventStream<'a>, provider::Error>> {
        Box::pin(async move {
            let Some(session_id) = request
                .options
                .metadata
                .get("session_id")
                .and_then(Value::as_str)
                .map(str::to_owned)
            else {
                return self.inner.stream(request).await;
            };

            let inner = self.inner.stream(request).await?;
            let stream = stream! {
                let mut pending = HashMap::<String, PendingToolCall>::new();
                futures::pin_mut!(inner);

                while let Some(event) = inner.as_mut().next().await {
                    match event {
                        Ok(Event::BlockStart { block }) => {
                            if let BlockKind::ToolCall { name: Some(name), .. } = &block.kind
                                && bridges_via_acp(name)
                            {
                                pending.insert(block.id.clone(), PendingToolCall::new(name.clone()));
                            }
                            yield Ok(Event::BlockStart { block });
                        }
                        Ok(Event::BlockDelta { id, delta: BlockDelta::Json { partial_json } }) => {
                            if let Some(call) = pending.get_mut(&id) {
                                call.parts.push(partial_json);
                            } else {
                                yield Ok(Event::BlockDelta {
                                    id,
                                    delta: BlockDelta::Json { partial_json },
                                });
                            }
                        }
                        Ok(Event::BlockStop { id }) => {
                            if let Some(call) = pending.remove(&id) {
                                yield Ok(Event::BlockDelta {
                                    id: id.clone(),
                                    delta: BlockDelta::Json {
                                        partial_json: call.inject_session_id(&session_id),
                                    },
                                });
                            }
                            yield Ok(Event::BlockStop { id });
                        }
                        other => yield other,
                    }
                }
            };

            Ok(Box::pin(stream) as EventStream<'a>)
        })
    }

    fn info(&self) -> ProviderInfo {
        self.inner.info()
    }
}

struct PendingToolCall {
    name: String,
    parts: Vec<String>,
}

impl PendingToolCall {
    fn new(name: String) -> Self {
        Self {
            name,
            parts: Vec::new(),
        }
    }

    fn inject_session_id(self, session_id: &str) -> String {
        let source = self.parts.concat();
        let Ok(Value::Object(mut object)) = serde_json::from_str::<Value>(&source) else {
            return source;
        };

        if bridges_via_acp(&self.name) {
            object.insert(
                ACP_SESSION_ID_FIELD.to_owned(),
                Value::String(session_id.to_owned()),
            );
        }

        Value::Object(object).to_string()
    }
}

struct AcpBridgeToolExecutor {
    native: RegistryToolExecutor,
    bridge: AcpFileBridge,
}

impl AcpBridgeToolExecutor {
    fn new(bridge: AcpFileBridge) -> Self {
        let mut native = RegistryToolExecutor::new();
        native.register(Arc::new(EchoTool));
        native.register(Arc::new(FileEditTool::new(NativeFileEditDriver)));
        native.register(Arc::new(ApplyPatchTool::new(NativeApplyPatchDriver)));
        native.register(Arc::new(ShellTool::new(NativeShellDriver)));
        native.register(Arc::new(ListDirectoryTool::new(NativeListDirectoryDriver)));
        native.register(Arc::new(GlobSearchTool::new(NativeGlobSearchDriver)));
        native.register(Arc::new(GrepTool::new(AutoGrepDriver)));
        Self { native, bridge }
    }
}

impl ToolExecutor for AcpBridgeToolExecutor {
    fn definitions(&self) -> Vec<ToolDefinition> {
        let mut definitions = self.native.definitions();
        definitions
            .push(agent_tools::FileReadTool::new(agent_tools::NativeFileReadDriver).definition());
        definitions
            .push(agent_tools::FileWriteTool::new(agent_tools::NativeFileWriteDriver).definition());
        definitions.sort_by(|left, right| left.name.cmp(&right.name));
        definitions
    }

    fn execute<'a>(
        &'a self,
        call: ToolCall,
    ) -> BoxFuture<'a, Result<ToolExecutionResult, RuntimeError>> {
        Box::pin(async move {
            match call.name.as_str() {
                "file_read" => {
                    let request: BridgeFileReadRequest = serde_json::from_value(call.input.clone())
                        .map_err(|error| {
                            RuntimeError::Tool(format!("invalid input for file_read: {error}"))
                        })?;
                    if let Some(session_id) = request.session_id.as_deref()
                        && let Some(output) = self
                            .bridge
                            .read_text_file(
                                session_id,
                                &request.path,
                                request.offset,
                                request.limit,
                            )
                            .await?
                    {
                        return Ok(ToolExecutionResult::success(serde_json::json!(output)));
                    }
                    self.native.execute(sanitize_call(call)).await
                }
                "file_write" => {
                    let request: BridgeFileWriteRequest =
                        serde_json::from_value(call.input.clone()).map_err(|error| {
                            RuntimeError::Tool(format!("invalid input for file_write: {error}"))
                        })?;
                    if let Some(session_id) = request.session_id.as_deref()
                        && let Some(output) = self
                            .bridge
                            .write_text_file(session_id, &request.path, &request.content)
                            .await?
                    {
                        return Ok(ToolExecutionResult::success(serde_json::json!(output)));
                    }
                    self.native.execute(sanitize_call(call)).await
                }
                _ => self.native.execute(call).await,
            }
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BridgeFileReadRequest {
    path: String,
    offset: Option<usize>,
    limit: Option<usize>,
    #[serde(default, rename = "__agent_acp_session_id")]
    session_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BridgeFileWriteRequest {
    path: String,
    content: String,
    #[serde(default, rename = "__agent_acp_session_id")]
    session_id: Option<String>,
}

fn sanitize_call(call: ToolCall) -> ToolCall {
    ToolCall {
        input: sanitize_tool_input(call.input),
        ..call
    }
}

fn bridges_via_acp(tool_name: &str) -> bool {
    matches!(tool_name, "file_read" | "file_write")
}

fn format_file_read_output(content: &str, offset: Option<usize>, limit: Option<usize>) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let start = offset.map(|value| value.saturating_sub(1)).unwrap_or(0);
    let limit = limit.unwrap_or(DEFAULT_READ_LIMIT).min(MAX_READ_LIMIT);
    let end = (start + limit).min(lines.len());

    if start >= lines.len() {
        return "Requested offset is beyond the end of the file.".to_owned();
    }

    let mut formatted = lines[start..end]
        .iter()
        .enumerate()
        .map(|(index, line)| {
            let line = truncate_line(line, MAX_LINE_CHARS);
            format!("{:>6}|{}", start + index + 1, line)
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

    formatted
}

fn truncate_line(line: &str, max_chars: usize) -> String {
    let mut truncated = String::new();
    for (index, ch) in line.chars().enumerate() {
        if index >= max_chars {
            truncated.push_str("...");
            return truncated;
        }
        truncated.push(ch);
    }
    truncated
}
