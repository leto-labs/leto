use agent_client_protocol::{self as acp, Agent as _};
use agent_core::{AgentCoreNative, CoreEvent};
use agent_runtime::{BlockDelta, Message, RuntimeEvent, ToolCall, ToolExecutionResult};
use agent_store::{InMemoryStore, Session, StoredMessage};
use async_stream::stream;
use provider::{
    Block, BlockKind, ContentBlock, Event, EventStream, FinishReason, MessageRole, MockProvider,
    ModelInfo, Provider, ProviderCapabilities, ProviderInfo, Request, Usage,
};
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::split;
use tokio_util::compat::{TokioAsyncReadCompatExt as _, TokioAsyncWriteCompatExt as _};

use crate::adapter::AgentCoreAcpBackend;
use crate::event_mapper::{EventMapper, MappedEvent};
use crate::file_bridge::{AcpFileBridge, tool_executor, wrap_provider};
use crate::history_replay::replay_updates;
use crate::stdio::spawn_notification_forwarder;

#[test]
fn output_text_delta_maps_to_agent_message_chunk() {
    let mut mapper = EventMapper::new();

    let mapped = mapper.map(CoreEvent::Turn {
        session_id: Session::new(ulid::Ulid::new()).id,
        event: RuntimeEvent::OutputBlockDelta {
            id: "block-1".into(),
            delta: BlockDelta::Text {
                text: "hello".into(),
            },
        },
    });

    let MappedEvent::Updates(updates) = mapped else {
        panic!("expected updates");
    };
    assert!(matches!(
        updates.as_slice(),
        [acp::SessionUpdate::AgentMessageChunk(acp::ContentChunk {
            content: acp::ContentBlock::Text(text),
            ..
        })] if text.text == "hello"
    ));
}

#[test]
fn tool_events_map_to_pending_and_completed_updates() {
    let mut mapper = EventMapper::new();
    let session_id = Session::new(ulid::Ulid::new()).id;
    let call = ToolCall {
        id: "call-1".into(),
        name: "shell".into(),
        input: serde_json::json!({"cmd": "pwd"}),
    };

    let pending = mapper.map(CoreEvent::Turn {
        session_id,
        event: RuntimeEvent::ToolCallPending { call: call.clone() },
    });
    let started = mapper.map(CoreEvent::Turn {
        session_id,
        event: RuntimeEvent::ToolCallStarted { call: call.clone() },
    });
    let completed = mapper.map(CoreEvent::Turn {
        session_id,
        event: RuntimeEvent::ToolCallFinished {
            call,
            result: ToolExecutionResult::success(serde_json::json!({"stdout": "/tmp"})),
        },
    });

    let MappedEvent::Updates(pending_updates) = pending else {
        panic!("expected pending updates");
    };
    assert!(matches!(
        pending_updates.as_slice(),
        [acp::SessionUpdate::ToolCall(tool_call)]
            if tool_call.tool_call_id.0.as_ref() == "call-1"
                && matches!(tool_call.status, acp::ToolCallStatus::Pending)
    ));

    let MappedEvent::Updates(started_updates) = started else {
        panic!("expected started updates");
    };
    assert!(matches!(
        started_updates.as_slice(),
        [acp::SessionUpdate::ToolCallUpdate(update)]
            if update.tool_call_id.0.as_ref() == "call-1"
                && matches!(update.fields.status, Some(acp::ToolCallStatus::InProgress))
    ));

    let MappedEvent::Updates(completed_updates) = completed else {
        panic!("expected completed updates");
    };
    assert!(matches!(
        completed_updates.as_slice(),
        [acp::SessionUpdate::ToolCallUpdate(update)]
            if update.tool_call_id.0.as_ref() == "call-1"
                && matches!(update.fields.status, Some(acp::ToolCallStatus::Completed))
    ));
}

#[test]
fn assistant_message_tool_calls_are_suppressed_after_runtime_tool_events() {
    let mut mapper = EventMapper::new();
    let session_id = Session::new(ulid::Ulid::new()).id;
    let call = ToolCall {
        id: "call-1".into(),
        name: "file_write".into(),
        input: serde_json::json!({"path": "hello.txt", "content": "Hello, world!"}),
    };

    let _ = mapper.map(CoreEvent::Turn {
        session_id,
        event: RuntimeEvent::ToolCallPending { call: call.clone() },
    });

    let mapped = mapper.map(CoreEvent::Turn {
        session_id,
        event: RuntimeEvent::MessageCommitted {
            message: Message::new(
                MessageRole::Assistant,
                vec![ContentBlock::ToolCall {
                    id: call.id,
                    call_id: None,
                    name: call.name,
                    input: call.input,
                }],
            ),
        },
    });

    let MappedEvent::Updates(updates) = mapped else {
        panic!("expected updates");
    };
    assert!(updates.is_empty());
}

#[test]
fn replay_updates_converts_stored_message_blocks_to_chunks() {
    let project_id = ulid::Ulid::new();
    let session = Session::new(project_id);
    let messages = vec![
        StoredMessage::new(
            session.id,
            0,
            Message::new(
                MessageRole::User,
                vec![
                    ContentBlock::text("hello"),
                    ContentBlock::image_url("https://example.com/cat.png"),
                ],
            ),
        ),
        StoredMessage::new(
            session.id,
            1,
            Message::new(
                MessageRole::Assistant,
                vec![ContentBlock::Reasoning {
                    text: "thinking".into(),
                }],
            ),
        ),
    ];

    let updates = replay_updates(&session, &messages);

    assert!(matches!(
        updates.get(1),
        Some(acp::SessionUpdate::UserMessageChunk(acp::ContentChunk {
            content: acp::ContentBlock::Text(text),
            ..
        })) if text.text == "hello"
    ));
    assert!(matches!(
        updates.get(2),
        Some(acp::SessionUpdate::UserMessageChunk(acp::ContentChunk {
            content: acp::ContentBlock::Text(text),
            ..
        })) if text.text == "[image:https://example.com/cat.png]"
    ));
    assert!(matches!(
        updates.get(3),
        Some(acp::SessionUpdate::AgentMessageChunk(acp::ContentChunk {
            content: acp::ContentBlock::Text(text),
            ..
        })) if text.text == "thinking"
    ));
}

#[derive(Clone, Default)]
struct BridgeTestClient {
    file_contents: Arc<Mutex<HashMap<PathBuf, String>>>,
    read_requests: Arc<Mutex<Vec<PathBuf>>>,
    write_requests: Arc<Mutex<Vec<(PathBuf, String)>>>,
    notifications: Arc<Mutex<Vec<acp::SessionNotification>>>,
}

impl BridgeTestClient {
    fn add_file(&self, path: impl Into<PathBuf>, content: impl Into<String>) {
        self.file_contents
            .lock()
            .unwrap()
            .insert(path.into(), content.into());
    }
}

#[async_trait::async_trait(?Send)]
impl acp::Client for BridgeTestClient {
    async fn request_permission(
        &self,
        _args: acp::RequestPermissionRequest,
    ) -> Result<acp::RequestPermissionResponse, acp::Error> {
        Ok(acp::RequestPermissionResponse::new(
            acp::RequestPermissionOutcome::Cancelled,
        ))
    }

    async fn session_notification(&self, args: acp::SessionNotification) -> Result<(), acp::Error> {
        self.notifications.lock().unwrap().push(args);
        Ok(())
    }

    async fn write_text_file(
        &self,
        args: acp::WriteTextFileRequest,
    ) -> Result<acp::WriteTextFileResponse, acp::Error> {
        self.file_contents
            .lock()
            .unwrap()
            .insert(args.path.clone(), args.content.clone());
        self.write_requests
            .lock()
            .unwrap()
            .push((args.path, args.content));
        Ok(acp::WriteTextFileResponse::new())
    }

    async fn read_text_file(
        &self,
        args: acp::ReadTextFileRequest,
    ) -> Result<acp::ReadTextFileResponse, acp::Error> {
        self.read_requests.lock().unwrap().push(args.path.clone());
        let content = self
            .file_contents
            .lock()
            .unwrap()
            .get(&args.path)
            .cloned()
            .unwrap_or_default();
        Ok(acp::ReadTextFileResponse::new(content))
    }
}

struct FileWriteToolProvider;

impl Provider for FileWriteToolProvider {
    fn stream<'a>(
        &'a self,
        request: &'a Request,
    ) -> futures::future::BoxFuture<'a, Result<EventStream<'a>, provider::Error>> {
        Box::pin(async move {
            let saw_tool_result = request.messages.iter().any(message_has_tool_result);
            let stream = stream! {
                yield Ok(Event::ResponseStart {
                    response_id: Some("file-write-provider".into()),
                    model: Some("file-write-model".into()),
                });

                if saw_tool_result {
                    yield Ok(Event::BlockStart {
                        block: Block {
                            id: "assistant-text".into(),
                            output_index: 0,
                            kind: BlockKind::Text,
                            item_id: Some("assistant-item".into()),
                        },
                    });
                    yield Ok(Event::BlockDelta {
                        id: "assistant-text".into(),
                        delta: provider::BlockDelta::Text {
                            text: "write complete".into(),
                        },
                    });
                    yield Ok(Event::BlockStop {
                        id: "assistant-text".into(),
                    });
                } else {
                    yield Ok(Event::BlockStart {
                        block: Block {
                            id: "assistant-tool".into(),
                            output_index: 0,
                            kind: BlockKind::ToolCall {
                                name: Some("file_write".into()),
                                call_id: Some("tool-call-1".into()),
                            },
                            item_id: Some("tool-item".into()),
                        },
                    });
                    yield Ok(Event::BlockDelta {
                        id: "assistant-tool".into(),
                        delta: provider::BlockDelta::Json {
                            partial_json: serde_json::json!({
                                "path": "nested/output.txt",
                                "content": "hello from client workspace"
                            }).to_string(),
                        },
                    });
                    yield Ok(Event::BlockStop {
                        id: "assistant-tool".into(),
                    });
                }

                yield Ok(Event::Usage {
                    usage: Usage::with_totals(Some(1), Some(1)),
                });
                yield Ok(Event::Completed {
                    response_id: Some("file-write-provider".into()),
                    finish_reason: Some(FinishReason::Stop),
                });
            };

            Ok(Box::pin(stream) as EventStream<'a>)
        })
    }

    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "file-write-provider".into(),
            default_model_id: Some("file-write-model".into()),
            capabilities: ProviderCapabilities {
                system_messages: true,
                developer_messages: true,
                input_text: true,
                input_image_urls: false,
                tool_calls: true,
                tool_results: true,
                reasoning_blocks: false,
                refusal_blocks: false,
                tool_call_argument_deltas: true,
                parallel_tool_calls: false,
                stream_granularity: provider::StreamGranularity::Block,
            },
            models: Vec::new(),
        }
    }
}

struct FileReadToolProvider;

impl Provider for FileReadToolProvider {
    fn stream<'a>(
        &'a self,
        request: &'a Request,
    ) -> futures::future::BoxFuture<'a, Result<EventStream<'a>, provider::Error>> {
        Box::pin(async move {
            let saw_tool_result = request.messages.iter().any(message_has_tool_result);
            let stream = stream! {
                yield Ok(Event::ResponseStart {
                    response_id: Some("file-read-provider".into()),
                    model: Some("file-read-model".into()),
                });

                if saw_tool_result {
                    yield Ok(Event::BlockStart {
                        block: Block {
                            id: "assistant-text".into(),
                            output_index: 0,
                            kind: BlockKind::Text,
                            item_id: Some("assistant-item".into()),
                        },
                    });
                    yield Ok(Event::BlockDelta {
                        id: "assistant-text".into(),
                        delta: provider::BlockDelta::Text {
                            text: "read complete".into(),
                        },
                    });
                    yield Ok(Event::BlockStop {
                        id: "assistant-text".into(),
                    });
                } else {
                    yield Ok(Event::BlockStart {
                        block: Block {
                            id: "assistant-tool".into(),
                            output_index: 0,
                            kind: BlockKind::ToolCall {
                                name: Some("file_read".into()),
                                call_id: Some("tool-call-1".into()),
                            },
                            item_id: Some("tool-item".into()),
                        },
                    });
                    yield Ok(Event::BlockDelta {
                        id: "assistant-tool".into(),
                        delta: provider::BlockDelta::Json {
                            partial_json: serde_json::json!({
                                "path": "nested/input.txt"
                            }).to_string(),
                        },
                    });
                    yield Ok(Event::BlockStop {
                        id: "assistant-tool".into(),
                    });
                }

                yield Ok(Event::Usage {
                    usage: Usage::with_totals(Some(1), Some(1)),
                });
                yield Ok(Event::Completed {
                    response_id: Some("file-read-provider".into()),
                    finish_reason: Some(FinishReason::Stop),
                });
            };

            Ok(Box::pin(stream) as EventStream<'a>)
        })
    }

    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "file-read-provider".into(),
            default_model_id: Some("file-read-model".into()),
            capabilities: ProviderCapabilities {
                system_messages: true,
                developer_messages: true,
                input_text: true,
                input_image_urls: false,
                tool_calls: true,
                tool_results: true,
                reasoning_blocks: false,
                refusal_blocks: false,
                tool_call_argument_deltas: true,
                parallel_tool_calls: false,
                stream_granularity: provider::StreamGranularity::Block,
            },
            models: Vec::new(),
        }
    }
}

fn message_has_tool_result(message: &provider::Message) -> bool {
    message
        .content
        .iter()
        .any(|block| matches!(block, provider::ContentBlock::ToolResult { .. }))
}

struct DualModelProvider {
    inner: MockProvider,
}

impl DualModelProvider {
    fn new() -> Self {
        Self {
            inner: MockProvider::new(),
        }
    }
}

impl Provider for DualModelProvider {
    fn stream<'a>(
        &'a self,
        request: &'a Request,
    ) -> futures::future::BoxFuture<'a, Result<EventStream<'a>, provider::Error>> {
        self.inner.stream(request)
    }

    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "dual-model".into(),
            default_model_id: Some("mock-echo".into()),
            capabilities: ProviderCapabilities {
                system_messages: true,
                developer_messages: true,
                input_text: true,
                input_image_urls: false,
                tool_calls: true,
                tool_results: true,
                reasoning_blocks: false,
                refusal_blocks: false,
                tool_call_argument_deltas: true,
                parallel_tool_calls: false,
                stream_granularity: provider::StreamGranularity::Block,
            },
            models: vec![
                ModelInfo {
                    id: Cow::Borrowed("mock-echo"),
                    name: Cow::Borrowed("Mock Echo"),
                    family: Some(Cow::Borrowed("mock")),
                    reasoning_efforts: Cow::Borrowed(&[]),
                    tool_call: true,
                    attachment: false,
                    structured_output: Some(false),
                    temperature: Some(true),
                    knowledge: None,
                    release_date: None,
                    last_updated: None,
                    open_weights: None,
                    input_modalities: Cow::Borrowed(&["text"]),
                    output_modalities: Cow::Borrowed(&["text"]),
                    cost: None,
                    limit: None,
                    status: None,
                    capabilities: None,
                },
                ModelInfo {
                    id: Cow::Borrowed("mock-deep"),
                    name: Cow::Borrowed("Mock Deep"),
                    family: Some(Cow::Borrowed("mock")),
                    reasoning_efforts: Cow::Borrowed(&[]),
                    tool_call: true,
                    attachment: false,
                    structured_output: Some(false),
                    temperature: Some(true),
                    knowledge: None,
                    release_date: None,
                    last_updated: None,
                    open_weights: None,
                    input_modalities: Cow::Borrowed(&["text"]),
                    output_modalities: Cow::Borrowed(&["text"]),
                    cost: None,
                    limit: None,
                    status: None,
                    capabilities: None,
                },
            ],
        }
    }
}

async fn build_test_core(provider: Arc<dyn Provider>, bridge: AcpFileBridge) -> AgentCoreNative {
    let store = Arc::new(InMemoryStore::new());
    AgentCoreNative::builder(store)
        .without_credential_discovery()
        .with_provider("tool", wrap_provider(provider))
        .with_tools(tool_executor(bridge))
        .default_provider("tool")
        .build()
        .await
        .expect("build test core")
}

async fn create_connection_pair(
    client: &BridgeTestClient,
    provider: Arc<dyn Provider>,
) -> (acp::ClientSideConnection, PathBuf) {
    let workspace_root = std::env::temp_dir().join(format!("agent-acp-{}", ulid::Ulid::new()));
    let bridge = AcpFileBridge::new();
    let core = build_test_core(provider, bridge.clone()).await;
    let (session_update_tx, session_update_rx) = tokio::sync::mpsc::unbounded_channel();
    let backend = AgentCoreAcpBackend::with_file_bridge(core, bridge.clone(), session_update_tx);

    let (client_stream, agent_stream) = tokio::io::duplex(16 * 1024);
    let (client_read, client_write) = split(client_stream);
    let (agent_read, agent_write) = split(agent_stream);

    let (agent_conn, agent_io_task) = acp::ClientSideConnection::new(
        client.clone(),
        client_write.compat_write(),
        client_read.compat(),
        |future| {
            tokio::task::spawn_local(future);
        },
    );

    let (connection, agent_side_io_task) = acp::AgentSideConnection::new(
        backend,
        agent_write.compat_write(),
        agent_read.compat(),
        |future| {
            tokio::task::spawn_local(future);
        },
    );
    let connection = std::rc::Rc::new(connection);

    spawn_notification_forwarder(session_update_rx, connection.clone());
    bridge.spawn_local_client_runner(connection);
    tokio::task::spawn_local(agent_io_task);
    tokio::task::spawn_local(agent_side_io_task);

    (agent_conn, workspace_root)
}

#[tokio::test]
async fn real_backend_file_write_bridges_into_client_workspace() {
    tokio::task::LocalSet::new()
        .run_until(async {
            let client = BridgeTestClient::default();
            let (agent_conn, workspace_root) =
                create_connection_pair(&client, Arc::new(FileWriteToolProvider)).await;

            agent_conn
                .initialize(
                    acp::InitializeRequest::new(acp::ProtocolVersion::LATEST).client_capabilities(
                        acp::ClientCapabilities::new().fs(acp::FileSystemCapabilities::new()
                            .read_text_file(true)
                            .write_text_file(true)),
                    ),
                )
                .await
                .expect("initialize");

            let session = agent_conn
                .new_session(acp::NewSessionRequest::new(workspace_root.clone()))
                .await
                .expect("new session");

            let response = agent_conn
                .prompt(acp::PromptRequest::new(
                    session.session_id,
                    vec![acp::ContentBlock::Text(acp::TextContent::new(
                        "write a file",
                    ))],
                ))
                .await
                .expect("prompt");

            assert!(matches!(response.stop_reason, acp::StopReason::EndTurn));

            let writes = client.write_requests.lock().unwrap();
            assert_eq!(writes.len(), 1);
            assert_eq!(writes[0].0, workspace_root.join("nested/output.txt"));
            assert_eq!(writes[0].1, "hello from client workspace");
            assert_eq!(
                client
                    .file_contents
                    .lock()
                    .unwrap()
                    .get(&workspace_root.join("nested/output.txt"))
                    .cloned(),
                Some("hello from client workspace".to_owned())
            );
        })
        .await;
}

#[tokio::test]
async fn real_backend_file_read_bridges_from_client_workspace() {
    tokio::task::LocalSet::new()
        .run_until(async {
            let client = BridgeTestClient::default();
            let (agent_conn, workspace_root) =
                create_connection_pair(&client, Arc::new(FileReadToolProvider)).await;
            client.add_file(
                workspace_root.join("nested/input.txt"),
                "first line\nsecond line\n",
            );

            agent_conn
                .initialize(
                    acp::InitializeRequest::new(acp::ProtocolVersion::LATEST).client_capabilities(
                        acp::ClientCapabilities::new().fs(acp::FileSystemCapabilities::new()
                            .read_text_file(true)
                            .write_text_file(true)),
                    ),
                )
                .await
                .expect("initialize");

            let session = agent_conn
                .new_session(acp::NewSessionRequest::new(workspace_root.clone()))
                .await
                .expect("new session");

            let response = agent_conn
                .prompt(acp::PromptRequest::new(
                    session.session_id,
                    vec![acp::ContentBlock::Text(acp::TextContent::new(
                        "read a file",
                    ))],
                ))
                .await
                .expect("prompt");

            assert!(matches!(response.stop_reason, acp::StopReason::EndTurn));

            let reads = client.read_requests.lock().unwrap();
            assert_eq!(reads.as_slice(), &[workspace_root.join("nested/input.txt")]);

            let notifications = client.notifications.lock().unwrap();
            assert!(notifications.iter().any(|notification| {
                format!("{:?}", notification.update).contains("first line")
            }));
        })
        .await;
}

#[tokio::test]
async fn real_backend_new_session_exposes_modes_and_available_commands() {
    tokio::task::LocalSet::new()
        .run_until(async {
            let client = BridgeTestClient::default();
            let (agent_conn, workspace_root) =
                create_connection_pair(&client, Arc::new(MockProvider::new())).await;

            agent_conn
                .initialize(acp::InitializeRequest::new(acp::ProtocolVersion::LATEST))
                .await
                .expect("initialize");

            let session = agent_conn
                .new_session(acp::NewSessionRequest::new(workspace_root))
                .await
                .expect("new session");

            let modes = session.modes.expect("expected real backend modes");
            assert_eq!(modes.current_mode_id.0.as_ref(), "simple");
            assert!(
                modes
                    .available_modes
                    .iter()
                    .any(|mode| mode.id.0.as_ref() == "robust"),
                "expected robust loop to be surfaced as an ACP mode"
            );

            let notifications = client.notifications.lock().unwrap();
            assert!(notifications.iter().any(|notification| matches!(
                notification.update,
                acp::SessionUpdate::AvailableCommandsUpdate(_)
            )));
        })
        .await;
}

#[tokio::test]
async fn real_backend_set_session_mode_updates_loop_state_and_load_response() {
    tokio::task::LocalSet::new()
        .run_until(async {
            let client = BridgeTestClient::default();
            let (agent_conn, workspace_root) =
                create_connection_pair(&client, Arc::new(MockProvider::new())).await;

            agent_conn
                .initialize(acp::InitializeRequest::new(acp::ProtocolVersion::LATEST))
                .await
                .expect("initialize");

            let session = agent_conn
                .new_session(acp::NewSessionRequest::new(workspace_root.clone()))
                .await
                .expect("new session");

            agent_conn
                .set_session_mode(acp::SetSessionModeRequest::new(
                    session.session_id.clone(),
                    "robust",
                ))
                .await
                .expect("set_session_mode");

            let loaded = agent_conn
                .load_session(acp::LoadSessionRequest::new(
                    session.session_id.clone(),
                    workspace_root,
                ))
                .await
                .expect("load_session");

            assert_eq!(
                loaded
                    .modes
                    .expect("expected load_session modes")
                    .current_mode_id
                    .0
                    .as_ref(),
                "robust"
            );

            let notifications = client.notifications.lock().unwrap();
            assert!(notifications.iter().any(|notification| matches!(
                &notification.update,
                acp::SessionUpdate::CurrentModeUpdate(update)
                    if update.current_mode_id.0.as_ref() == "robust"
            )));
        })
        .await;
}

#[tokio::test]
async fn real_backend_does_not_expose_mode_as_config_option() {
    tokio::task::LocalSet::new()
        .run_until(async {
            let client = BridgeTestClient::default();
            let (agent_conn, workspace_root) =
                create_connection_pair(&client, Arc::new(MockProvider::new())).await;

            agent_conn
                .initialize(acp::InitializeRequest::new(acp::ProtocolVersion::LATEST))
                .await
                .expect("initialize");

            let session = agent_conn
                .new_session(acp::NewSessionRequest::new(workspace_root))
                .await
                .expect("new session");

            let config_options = session
                .config_options
                .expect("expected config options on new session");
            assert!(
                config_options
                    .iter()
                    .all(|option| option.id.0.as_ref() != "mode" && option.id.0.as_ref() != "loop"),
                "expected ACP mode selection to be exposed only through session modes"
            );
        })
        .await;
}

#[cfg(feature = "unstable_session_model")]
#[tokio::test]
async fn real_backend_new_session_exposes_models_and_set_session_model_updates_reload() {
    tokio::task::LocalSet::new()
        .run_until(async {
            let client = BridgeTestClient::default();
            let (agent_conn, workspace_root) =
                create_connection_pair(&client, Arc::new(DualModelProvider::new())).await;

            agent_conn
                .initialize(acp::InitializeRequest::new(acp::ProtocolVersion::LATEST))
                .await
                .expect("initialize");

            let session = agent_conn
                .new_session(acp::NewSessionRequest::new(workspace_root.clone()))
                .await
                .expect("new session");

            let models = session.models.expect("expected ACP models on new session");
            assert!(
                models
                    .available_models
                    .iter()
                    .any(|model| model.model_id.0.as_ref() == "mock-echo"),
                "expected default model to be surfaced through ACP model state"
            );
            assert!(
                models
                    .available_models
                    .iter()
                    .any(|model| model.model_id.0.as_ref() == "mock-deep"),
                "expected alternate model to be exposed through ACP model state"
            );
            let target_model = if models.current_model_id.0.as_ref() == "mock-deep" {
                "mock-echo"
            } else {
                "mock-deep"
            };

            let notification_count_before_update = client.notifications.lock().unwrap().len();
            agent_conn
                .set_session_model(acp::SetSessionModelRequest::new(
                    session.session_id.clone(),
                    target_model,
                ))
                .await
                .expect("set_session_model");

            let loaded = agent_conn
                .load_session(acp::LoadSessionRequest::new(
                    session.session_id.clone(),
                    workspace_root,
                ))
                .await
                .expect("load_session");

            assert_eq!(
                loaded
                    .models
                    .expect("expected load_session models")
                    .current_model_id
                    .0
                    .as_ref(),
                target_model
            );

            let notifications = client.notifications.lock().unwrap();
            assert!(
                notifications[notification_count_before_update..]
                    .iter()
                    .any(|notification| matches!(
                        notification.update,
                        acp::SessionUpdate::ConfigOptionUpdate(_)
                    ))
            );
        })
        .await;
}

#[cfg(feature = "unstable_session_resume")]
#[tokio::test]
async fn real_backend_resume_session_returns_state_without_replaying_history() {
    tokio::task::LocalSet::new()
        .run_until(async {
            let client = BridgeTestClient::default();
            let (agent_conn, workspace_root) =
                create_connection_pair(&client, Arc::new(MockProvider::new())).await;

            agent_conn
                .initialize(acp::InitializeRequest::new(acp::ProtocolVersion::LATEST))
                .await
                .expect("initialize");

            let session = agent_conn
                .new_session(acp::NewSessionRequest::new(workspace_root.clone()))
                .await
                .expect("new session");

            agent_conn
                .prompt(acp::PromptRequest::new(
                    session.session_id.clone(),
                    vec![acp::ContentBlock::Text(acp::TextContent::new(
                        "resume should not replay this",
                    ))],
                ))
                .await
                .expect("prompt");

            let notification_count_before_resume = client.notifications.lock().unwrap().len();
            let resumed = agent_conn
                .resume_session(acp::ResumeSessionRequest::new(
                    session.session_id.clone(),
                    workspace_root,
                ))
                .await
                .expect("resume_session");

            assert_eq!(
                resumed
                    .modes
                    .expect("expected resumed modes")
                    .current_mode_id
                    .0
                    .as_ref(),
                "simple"
            );

            let notifications = client.notifications.lock().unwrap();
            let resumed_notifications = &notifications[notification_count_before_resume..];
            assert!(resumed_notifications.iter().any(|notification| matches!(
                notification.update,
                acp::SessionUpdate::AvailableCommandsUpdate(_)
            )));
            assert!(resumed_notifications.iter().all(|notification| {
                !matches!(
                    notification.update,
                    acp::SessionUpdate::UserMessageChunk(_)
                        | acp::SessionUpdate::AgentMessageChunk(_)
                        | acp::SessionUpdate::AgentThoughtChunk(_)
                )
            }));
        })
        .await;
}

#[cfg(all(feature = "unstable_session_fork", feature = "unstable_session_model"))]
#[tokio::test]
async fn real_backend_fork_session_copies_transcript_and_overrides() {
    tokio::task::LocalSet::new()
        .run_until(async {
            let client = BridgeTestClient::default();
            let (agent_conn, workspace_root) =
                create_connection_pair(&client, Arc::new(DualModelProvider::new())).await;

            agent_conn
                .initialize(acp::InitializeRequest::new(acp::ProtocolVersion::LATEST))
                .await
                .expect("initialize");

            let session = agent_conn
                .new_session(acp::NewSessionRequest::new(workspace_root.clone()))
                .await
                .expect("new session");

            agent_conn
                .set_session_mode(acp::SetSessionModeRequest::new(
                    session.session_id.clone(),
                    "robust",
                ))
                .await
                .expect("set_session_mode");
            agent_conn
                .set_session_model(acp::SetSessionModelRequest::new(
                    session.session_id.clone(),
                    "mock-deep",
                ))
                .await
                .expect("set_session_model");
            agent_conn
                .set_session_config_option(acp::SetSessionConfigOptionRequest::new(
                    session.session_id.clone(),
                    "thought_level",
                    "high",
                ))
                .await
                .expect("set_session_config_option");
            agent_conn
                .prompt(acp::PromptRequest::new(
                    session.session_id.clone(),
                    vec![acp::ContentBlock::Text(acp::TextContent::new(
                        "fork this transcript",
                    ))],
                ))
                .await
                .expect("prompt");

            let forked = agent_conn
                .fork_session(acp::ForkSessionRequest::new(
                    session.session_id.clone(),
                    workspace_root.clone(),
                ))
                .await
                .expect("fork_session");

            assert_eq!(
                forked
                    .modes
                    .expect("expected forked modes")
                    .current_mode_id
                    .0
                    .as_ref(),
                "robust"
            );
            assert_eq!(
                forked
                    .models
                    .expect("expected forked models")
                    .current_model_id
                    .0
                    .as_ref(),
                "mock-deep"
            );
            let thought_level = forked
                .config_options
                .expect("expected forked config options")
                .into_iter()
                .find(|option| option.id.0.as_ref() == "thought_level")
                .expect("thought_level config option");
            assert!(matches!(
                thought_level.kind,
                acp::SessionConfigKind::Select(ref select)
                    if select.current_value.0.as_ref() == "high"
            ));

            let notification_count_before_load = client.notifications.lock().unwrap().len();
            agent_conn
                .load_session(acp::LoadSessionRequest::new(
                    forked.session_id.clone(),
                    workspace_root,
                ))
                .await
                .expect("load forked session");

            let notifications = client.notifications.lock().unwrap();
            assert!(notifications[notification_count_before_load..].iter().any(
                |notification| matches!(
                    &notification.update,
                    acp::SessionUpdate::UserMessageChunk(acp::ContentChunk {
                        content: acp::ContentBlock::Text(text),
                        ..
                    }) if text.text.contains("fork this transcript")
                )
            ));
        })
        .await;
}

#[cfg(feature = "unstable_session_close")]
#[tokio::test]
async fn real_backend_close_session_cancels_active_turn_and_preserves_history() {
    tokio::task::LocalSet::new()
        .run_until(async {
            let client = BridgeTestClient::default();
            let (agent_conn, workspace_root) =
                create_connection_pair(&client, Arc::new(MockProvider::new().with_delay(25))).await;
            let agent_conn = Rc::new(agent_conn);

            agent_conn
                .initialize(acp::InitializeRequest::new(acp::ProtocolVersion::LATEST))
                .await
                .expect("initialize");

            let session = agent_conn
                .new_session(acp::NewSessionRequest::new(workspace_root.clone()))
                .await
                .expect("new session");

            agent_conn
                .prompt(acp::PromptRequest::new(
                    session.session_id.clone(),
                    vec![acp::ContentBlock::Text(acp::TextContent::new(
                        "persisted history",
                    ))],
                ))
                .await
                .expect("prompt");

            let prompt_conn = agent_conn.clone();
            let session_id = session.session_id.clone();
            let active_prompt = tokio::task::spawn_local(async move {
                prompt_conn
                    .prompt(acp::PromptRequest::new(
                        session_id,
                        vec![acp::ContentBlock::Text(acp::TextContent::new(
                            "slow close target",
                        ))],
                    ))
                    .await
            });

            tokio::time::sleep(Duration::from_millis(10)).await;
            agent_conn
                .close_session(acp::CloseSessionRequest::new(session.session_id.clone()))
                .await
                .expect("close_session");

            let prompt_response = active_prompt.await.expect("join prompt").expect("prompt");
            assert!(matches!(
                prompt_response.stop_reason,
                acp::StopReason::Cancelled
            ));

            let notification_count_before_load = client.notifications.lock().unwrap().len();
            agent_conn
                .load_session(acp::LoadSessionRequest::new(
                    session.session_id.clone(),
                    workspace_root,
                ))
                .await
                .expect("load_session");

            let notifications = client.notifications.lock().unwrap();
            assert!(notifications[notification_count_before_load..].iter().any(
                |notification| matches!(
                    &notification.update,
                    acp::SessionUpdate::UserMessageChunk(acp::ContentChunk {
                        content: acp::ContentBlock::Text(text),
                        ..
                    }) if text.text.contains("persisted history")
                )
            ));
        })
        .await;
}

#[tokio::test]
async fn real_backend_list_sessions_without_cwd_returns_all_sessions() {
    tokio::task::LocalSet::new()
        .run_until(async {
            let client = BridgeTestClient::default();
            let (agent_conn, workspace_root) =
                create_connection_pair(&client, Arc::new(MockProvider::new())).await;
            let second_workspace =
                std::env::temp_dir().join(format!("agent-acp-second-{}", ulid::Ulid::new()));

            agent_conn
                .initialize(acp::InitializeRequest::new(acp::ProtocolVersion::LATEST))
                .await
                .expect("initialize");

            let first = agent_conn
                .new_session(acp::NewSessionRequest::new(workspace_root.clone()))
                .await
                .expect("first session");
            let second = agent_conn
                .new_session(acp::NewSessionRequest::new(second_workspace.clone()))
                .await
                .expect("second session");

            let sessions = agent_conn
                .list_sessions(acp::ListSessionsRequest::new())
                .await
                .expect("list_sessions");

            assert!(
                sessions
                    .sessions
                    .iter()
                    .any(|session| session.session_id == first.session_id
                        && session.cwd == workspace_root)
            );
            assert!(
                sessions
                    .sessions
                    .iter()
                    .any(|session| session.session_id == second.session_id
                        && session.cwd == second_workspace)
            );
        })
        .await;
}

#[tokio::test]
async fn real_backend_prompt_resource_links_preserves_baseline_content() {
    tokio::task::LocalSet::new()
        .run_until(async {
            let client = BridgeTestClient::default();
            let (agent_conn, workspace_root) =
                create_connection_pair(&client, Arc::new(MockProvider::new())).await;

            agent_conn
                .initialize(acp::InitializeRequest::new(acp::ProtocolVersion::LATEST))
                .await
                .expect("initialize");

            let session = agent_conn
                .new_session(acp::NewSessionRequest::new(workspace_root))
                .await
                .expect("new session");

            let notification_count_before_prompt = client.notifications.lock().unwrap().len();
            agent_conn
                .prompt(acp::PromptRequest::new(
                    session.session_id.clone(),
                    vec![
                        acp::ContentBlock::Text(acp::TextContent::new("inspect")),
                        acp::ContentBlock::ResourceLink(acp::ResourceLink::new(
                            "repo-readme",
                            "file:///tmp/README.md",
                        )),
                    ],
                ))
                .await
                .expect("prompt");

            let notifications = client.notifications.lock().unwrap();
            assert!(
                notifications[notification_count_before_prompt..]
                    .iter()
                    .any(|notification| matches!(
                        &notification.update,
                        acp::SessionUpdate::AgentMessageChunk(acp::ContentChunk {
                            content: acp::ContentBlock::Text(text),
                            ..
                        }) if text.text.contains("[resource:file:///tmp/README.md]")
                    ))
            );
            assert!(
                notifications[notification_count_before_prompt..]
                    .iter()
                    .all(|notification| !matches!(
                        &notification.update,
                        acp::SessionUpdate::AgentMessageChunk(acp::ContentChunk {
                            content: acp::ContentBlock::Text(text),
                            ..
                        }) if text.text.contains("[unsupported-content]")
                    ))
            );
        })
        .await;
}

#[tokio::test]
async fn real_backend_first_prompt_emits_session_title_update() {
    tokio::task::LocalSet::new()
        .run_until(async {
            let client = BridgeTestClient::default();
            let (agent_conn, workspace_root) =
                create_connection_pair(&client, Arc::new(MockProvider::new())).await;

            agent_conn
                .initialize(acp::InitializeRequest::new(acp::ProtocolVersion::LATEST))
                .await
                .expect("initialize");

            let session = agent_conn
                .new_session(acp::NewSessionRequest::new(workspace_root))
                .await
                .expect("new session");

            let notification_count_before_prompt = client.notifications.lock().unwrap().len();
            agent_conn
                .prompt(acp::PromptRequest::new(
                    session.session_id.clone(),
                    vec![acp::ContentBlock::Text(acp::TextContent::new(
                        "title me please",
                    ))],
                ))
                .await
                .expect("prompt");

            let notifications = client.notifications.lock().unwrap();
            assert!(notifications[notification_count_before_prompt..]
                .iter()
                .any(|notification| matches!(
                    &notification.update,
                    acp::SessionUpdate::SessionInfoUpdate(update)
                        if matches!(update.title.as_opt_ref(), Some(Some(title)) if title == "title me please")
                )));
        })
        .await;
}
