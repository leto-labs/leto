use std::borrow::Cow;
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use agent_core::{AgentCore, AgentCoreNative, CoreEvent};
use agent_core_remote::CredentialHealthRecord;
use agent_runtime::RuntimeEvent;
use agent_server::{AgentServer, SessionRuntimeView, build_router};
use agent_store::{CredentialEntry, Project, ProjectConfig, Session, Store, StoredMessage};
use futures::StreamExt;
use provider::{
    Block, BlockKind, Event, EventStream, FinishReason, MessageRole, MockProvider, ModelInfo,
    Provider, ProviderCapabilities, ProviderInfo, Request, StreamGranularity, Usage,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

fn make_server() -> Arc<AgentServer> {
    make_server_with_provider(Arc::new(MockProvider::new()))
}

fn make_server_with_core(core: Arc<dyn AgentCore>) -> Arc<AgentServer> {
    Arc::new(AgentServer::new(core))
}

fn make_server_with_provider(provider: Arc<dyn Provider>) -> Arc<AgentServer> {
    make_server_with_providers(vec![("mock", provider)], "mock")
}

fn make_server_with_providers(
    providers: Vec<(&str, Arc<dyn Provider>)>,
    default_provider: &str,
) -> Arc<AgentServer> {
    let core: Arc<dyn AgentCore> = Arc::new(futures::executor::block_on(async {
        let mut builder = AgentCoreNative::builder(Arc::new(agent_store::InMemoryStore::new()))
            .default_provider(default_provider);
        for (name, provider) in providers {
            builder = builder.with_provider(name, provider);
        }
        builder.build().await.unwrap()
    }));
    Arc::new(AgentServer::new(core))
}

async fn start_server() -> String {
    let server = make_server();
    let router = build_router(server);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    format!("http://{addr}")
}

async fn start_server_with_core(core: Arc<dyn AgentCore>) -> String {
    let server = make_server_with_core(core);
    let router = build_router(server);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    format!("http://{addr}")
}

async fn start_server_with_provider(provider: Arc<dyn Provider>) -> String {
    let server = make_server_with_provider(provider);
    let router = build_router(server);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    format!("http://{addr}")
}

async fn start_server_with_providers(
    providers: Vec<(&str, Arc<dyn Provider>)>,
    default_provider: &str,
) -> String {
    let server = make_server_with_providers(providers, default_provider);
    let router = build_router(server);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    format!("http://{addr}")
}

async fn create_project(client: &reqwest::Client, base: &str) -> Project {
    create_project_with_config(client, base, ProjectConfig::default()).await
}

async fn create_project_with_config(
    client: &reqwest::Client,
    base: &str,
    config: ProjectConfig,
) -> Project {
    client
        .post(format!("{base}/v1/projects"))
        .json(&serde_json::json!({
            "name": "agent-server-turn-test",
            "config": config,
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap()
}

async fn update_project_config(
    client: &reqwest::Client,
    base: &str,
    project: &Project,
    config: ProjectConfig,
) -> Project {
    client
        .patch(format!("{base}/v1/projects/{}", project.id))
        .json(&serde_json::json!({
            "config": config,
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap()
}

async fn create_session(client: &reqwest::Client, base: &str, project: &Project) -> Session {
    client
        .post(format!("{base}/v1/projects/{}/sessions", project.id))
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap()
}

async fn post_turn(
    client: &reqwest::Client,
    base: &str,
    session: &Session,
    text: &str,
) -> reqwest::Response {
    client
        .post(format!("{base}/v1/sessions/{}/turns", session.id))
        .json(&serde_json::json!({
            "input": [
                {
                    "role": "user",
                    "content": [{"type": "text", "text": text}]
                }
            ]
        }))
        .send()
        .await
        .unwrap()
}

async fn post_stream_turn(
    client: &reqwest::Client,
    base: &str,
    session: &Session,
    text: &str,
) -> reqwest::Response {
    client
        .post(format!("{base}/v1/sessions/{}/stream-turns", session.id))
        .json(&serde_json::json!({
            "input": [
                {
                    "role": "user",
                    "content": [{"type": "text", "text": text}]
                }
            ]
        }))
        .send()
        .await
        .unwrap()
}

async fn post_cancel_turn(
    client: &reqwest::Client,
    base: &str,
    session: &Session,
) -> reqwest::Response {
    client
        .post(format!("{base}/v1/sessions/{}/cancel", session.id))
        .send()
        .await
        .unwrap()
}

async fn get_session_events(
    client: &reqwest::Client,
    base: &str,
    session: &Session,
) -> reqwest::Response {
    client
        .get(format!("{base}/v1/sessions/{}/events", session.id))
        .send()
        .await
        .unwrap()
}

fn parse_ndjson_events(body: &str) -> Vec<CoreEvent> {
    body.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn parse_sse_events(body: &str) -> Vec<CoreEvent> {
    let mut events = Vec::new();
    let mut data_lines = Vec::new();

    for line in body.lines() {
        if line.is_empty() {
            if !data_lines.is_empty() {
                events.push(serde_json::from_str(&data_lines.join("\n")).unwrap());
                data_lines.clear();
            }
            continue;
        }

        if line.starts_with(':') {
            continue;
        }

        if let Some(data) = line.strip_prefix("data:") {
            data_lines.push(data.trim_start().to_owned());
        }
    }

    if !data_lines.is_empty() {
        events.push(serde_json::from_str(&data_lines.join("\n")).unwrap());
    }

    events
}

static OLLAMA_MOCK_SERVER_LOCK: LazyLock<tokio::sync::Mutex<()>> =
    LazyLock::new(|| tokio::sync::Mutex::new(()));

fn completed_openai_sse_body() -> String {
    concat!(
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_test\",\"status\":\"completed\",\"output\":[],\"usage\":{\"input_tokens\":1,\"output_tokens\":1,\"total_tokens\":2}}}\n\n",
        "data: [DONE]\n\n"
    )
    .to_owned()
}

fn completed_openai_sse_body_with_usage_details() -> String {
    concat!(
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_usage\",\"status\":\"completed\",\"output\":[],\"usage\":{\"input_tokens\":11,\"output_tokens\":7,\"total_tokens\":18,\"input_tokens_details\":{\"cached_tokens\":3,\"cache_creation_tokens\":2},\"output_tokens_details\":{\"reasoning_tokens\":5}}}}\n\n",
        "data: [DONE]\n\n"
    )
    .to_owned()
}

fn created_only_openai_sse_body() -> String {
    "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_test\",\"status\":\"in_progress\",\"output\":[]}}\n\n".to_owned()
}

async fn spawn_ollama_mock_server(bodies: Vec<String>) -> oneshot::Receiver<Vec<String>> {
    let listener = TcpListener::bind("127.0.0.1:11434").await.unwrap();
    let (tx, rx) = oneshot::channel();

    tokio::spawn(async move {
        let mut requests = Vec::with_capacity(bodies.len());
        for body in bodies {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0_u8; 16 * 1024];
            let mut request = Vec::new();

            loop {
                let read = socket.read(&mut buf).await.unwrap();
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&buf[..read]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }

            requests.push(String::from_utf8_lossy(&request).into_owned());

            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(response.as_bytes()).await.unwrap();
            socket.shutdown().await.unwrap();
        }

        let _ = tx.send(requests);
    });

    rx
}

struct TaggedProvider {
    tag: &'static str,
}

impl TaggedProvider {
    fn new(tag: &'static str) -> Self {
        Self { tag }
    }
}

#[derive(Default)]
struct RecordingMemoryProvider {
    requests: Mutex<Vec<Vec<(MessageRole, String)>>>,
}

impl RecordingMemoryProvider {
    fn recorded_requests(&self) -> Vec<Vec<(MessageRole, String)>> {
        self.requests.lock().unwrap().clone()
    }
}

impl Provider for RecordingMemoryProvider {
    fn stream<'a>(
        &'a self,
        request: &'a Request,
    ) -> futures::future::BoxFuture<'a, Result<EventStream<'a>, provider::Error>> {
        Box::pin(async move {
            let transcript = request
                .messages
                .iter()
                .map(|message| (message.role, message.plain_text_lossy()))
                .collect();
            self.requests.lock().unwrap().push(transcript);

            let block_id = "memory-text-1".to_owned();
            let response_id = "memory-response-1".to_owned();
            let text = format!(
                "memory: {}",
                request.last_user_text_lossy().unwrap_or_default()
            );

            Ok(Box::pin(futures::stream::iter(vec![
                Ok(Event::ResponseStart {
                    response_id: Some(response_id.clone()),
                    model: Some("memory-echo".into()),
                }),
                Ok(Event::BlockStart {
                    block: Block {
                        id: block_id.clone(),
                        output_index: 0,
                        kind: BlockKind::Text,
                        item_id: Some("memory-item-1".into()),
                    },
                }),
                Ok(Event::text_delta(block_id.clone(), text)),
                Ok(Event::BlockStop { id: block_id }),
                Ok(Event::Usage {
                    usage: Usage::with_totals(Some(1), Some(1)),
                }),
                Ok(Event::Completed {
                    response_id: Some(response_id),
                    finish_reason: Some(FinishReason::Stop),
                }),
            ])) as EventStream<'a>)
        })
    }

    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "memory".into(),
            default_model_id: Some("memory-echo".into()),
            capabilities: ProviderCapabilities {
                system_messages: true,
                developer_messages: true,
                input_text: true,
                input_image_urls: false,
                tool_calls: false,
                tool_results: false,
                reasoning_blocks: false,
                refusal_blocks: false,
                tool_call_argument_deltas: false,
                parallel_tool_calls: false,
                stream_granularity: StreamGranularity::Block,
            },
            models: vec![ModelInfo {
                id: Cow::Borrowed("memory-echo"),
                name: Cow::Borrowed("Memory Echo"),
                family: Some(Cow::Borrowed("mock")),
                reasoning_efforts: Cow::Borrowed(&[]),
                tool_call: false,
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
            }],
        }
    }
}

impl Provider for TaggedProvider {
    fn stream<'a>(
        &'a self,
        request: &'a Request,
    ) -> futures::future::BoxFuture<'a, Result<EventStream<'a>, provider::Error>> {
        Box::pin(async move {
            let block_id = format!("{}-text-1", self.tag);
            let item_id = format!("{}-item-1", self.tag);
            let response_id = format!("{}-response-1", self.tag);
            let default_model_id = format!("{}-echo", self.tag);
            let text = format!(
                "{}: {}",
                self.tag,
                request.last_user_text_lossy().unwrap_or_default()
            );

            Ok(Box::pin(futures::stream::iter(vec![
                Ok(Event::ResponseStart {
                    response_id: Some(response_id.clone()),
                    model: Some(request.model.clone().unwrap_or(default_model_id)),
                }),
                Ok(Event::BlockStart {
                    block: Block {
                        id: block_id.clone(),
                        output_index: 0,
                        kind: BlockKind::Text,
                        item_id: Some(item_id),
                    },
                }),
                Ok(Event::text_delta(block_id.clone(), text)),
                Ok(Event::BlockStop { id: block_id }),
                Ok(Event::Usage {
                    usage: Usage::with_totals(Some(1), Some(1)),
                }),
                Ok(Event::Completed {
                    response_id: Some(response_id),
                    finish_reason: Some(FinishReason::Stop),
                }),
            ])) as EventStream<'a>)
        })
    }

    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: self.tag.into(),
            default_model_id: Some(format!("{}-echo", self.tag)),
            capabilities: ProviderCapabilities {
                system_messages: true,
                developer_messages: true,
                input_text: true,
                input_image_urls: false,
                tool_calls: false,
                tool_results: false,
                reasoning_blocks: false,
                refusal_blocks: false,
                tool_call_argument_deltas: false,
                parallel_tool_calls: false,
                stream_granularity: StreamGranularity::Block,
            },
            models: vec![ModelInfo {
                id: Cow::Owned(format!("{}-echo", self.tag)),
                name: Cow::Owned(format!("{} echo", self.tag)),
                family: Some(Cow::Borrowed("mock")),
                reasoning_efforts: Cow::Borrowed(&[]),
                tool_call: false,
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
            }],
        }
    }
}

struct RetryOnceProvider {
    attempts: AtomicUsize,
    fallback: MockProvider,
}

impl RetryOnceProvider {
    fn new() -> Self {
        Self {
            attempts: AtomicUsize::new(0),
            fallback: MockProvider::new(),
        }
    }
}

impl Provider for RetryOnceProvider {
    fn stream<'a>(
        &'a self,
        request: &'a Request,
    ) -> futures::future::BoxFuture<'a, Result<EventStream<'a>, provider::Error>> {
        Box::pin(async move {
            if self.attempts.fetch_add(1, Ordering::SeqCst) == 0 {
                return Err(provider::Error::Inference(
                    "429 Too Many Requests: request throttled, retry after 30 seconds".into(),
                ));
            }

            self.fallback.stream(request).await
        })
    }

    fn info(&self) -> ProviderInfo {
        self.fallback.info()
    }
}

struct TimeoutProvider;

impl Provider for TimeoutProvider {
    fn stream<'a>(
        &'a self,
        _request: &'a Request,
    ) -> futures::future::BoxFuture<'a, Result<EventStream<'a>, provider::Error>> {
        Box::pin(async {
            Err(provider::Error::Inference(
                "request timed out after 30 seconds".into(),
            ))
        })
    }

    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "mock".into(),
            default_model_id: Some("mock-echo".into()),
            capabilities: ProviderCapabilities {
                system_messages: true,
                developer_messages: true,
                input_text: true,
                input_image_urls: false,
                tool_calls: false,
                tool_results: false,
                reasoning_blocks: false,
                refusal_blocks: false,
                tool_call_argument_deltas: false,
                parallel_tool_calls: false,
                stream_granularity: StreamGranularity::Block,
            },
            models: vec![ModelInfo {
                id: Cow::Borrowed("mock-echo"),
                name: Cow::Borrowed("Mock Echo"),
                family: Some(Cow::Borrowed("mock")),
                reasoning_efforts: Cow::Borrowed(&[]),
                tool_call: false,
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
            }],
        }
    }
}

async fn read_sse_events_until<F>(response: reqwest::Response, predicate: F) -> Vec<CoreEvent>
where
    F: Fn(&[CoreEvent]) -> bool,
{
    let mut stream = response.bytes_stream();
    let mut pending = String::new();
    let mut events = Vec::new();

    loop {
        let chunk = tokio::time::timeout(Duration::from_secs(5), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        pending.push_str(std::str::from_utf8(&chunk).unwrap());

        while let Some(boundary) = pending.find("\n\n") {
            let frame = pending[..boundary].to_owned();
            pending.drain(..boundary + 2);

            let mut data_lines = Vec::new();
            for line in frame.lines() {
                if line.starts_with(':') {
                    continue;
                }
                if let Some(data) = line.strip_prefix("data:") {
                    data_lines.push(data.trim_start().to_owned());
                }
            }

            if !data_lines.is_empty() {
                events.push(serde_json::from_str(&data_lines.join("\n")).unwrap());
            }
        }

        if predicate(&events) {
            return events;
        }
    }
}

#[tokio::test]
async fn post_turns_completes_turn_and_persists_assistant_message() {
    let base = start_server().await;
    let client = reqwest::Client::new();
    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, &project).await;

    let response = post_turn(&client, &base, &session, "hello canonical server").await;

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .unwrap(),
        "application/x-ndjson"
    );

    let events = parse_ndjson_events(&response.text().await.unwrap());
    assert!(events.iter().any(|event| {
        matches!(
            event,
            CoreEvent::Turn {
                session_id,
                event: RuntimeEvent::TurnFinished {
                    session_id: finished_session_id,
                    finish_reason: Some(FinishReason::Stop),
                    ..
                },
            } if *session_id == session.id && *finished_session_id == session.id
        )
    }));

    let messages: Vec<StoredMessage> = client
        .get(format!("{base}/v1/sessions/{}/messages", session.id))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].message.role, MessageRole::User);
    assert_eq!(
        messages[0].message.plain_text_lossy(),
        "hello canonical server"
    );
    assert_eq!(messages[1].message.role, MessageRole::Assistant);
    assert!(
        messages[1]
            .message
            .plain_text_lossy()
            .contains("hello canonical server")
    );
}

#[tokio::test]
async fn post_turns_allows_follow_up_turn_after_completion() {
    let base = start_server().await;
    let client = reqwest::Client::new();
    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, &project).await;

    let first = post_turn(&client, &base, &session, "first turn").await;
    assert_eq!(first.status(), reqwest::StatusCode::OK);
    let first_events = parse_ndjson_events(&first.text().await.unwrap());
    assert!(matches!(
        first_events.last(),
        Some(CoreEvent::Turn {
            session_id,
            event: RuntimeEvent::TurnFinished {
                session_id: finished_session_id,
                finish_reason: Some(FinishReason::Stop),
                ..
            },
        }) if *session_id == session.id && *finished_session_id == session.id
    ));

    let second = post_turn(&client, &base, &session, "second turn").await;
    assert_eq!(second.status(), reqwest::StatusCode::OK);
    let second_events = parse_ndjson_events(&second.text().await.unwrap());
    assert!(matches!(
        second_events.last(),
        Some(CoreEvent::Turn {
            session_id,
            event: RuntimeEvent::TurnFinished {
                session_id: finished_session_id,
                finish_reason: Some(FinishReason::Stop),
                ..
            },
        }) if *session_id == session.id && *finished_session_id == session.id
    ));

    let messages: Vec<StoredMessage> = client
        .get(format!("{base}/v1/sessions/{}/messages", session.id))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(messages.len(), 4);
    assert_eq!(messages[0].message.role, MessageRole::User);
    assert_eq!(messages[0].message.plain_text_lossy(), "first turn");
    assert_eq!(messages[1].message.role, MessageRole::Assistant);
    assert_eq!(messages[2].message.role, MessageRole::User);
    assert_eq!(messages[2].message.plain_text_lossy(), "second turn");
    assert_eq!(messages[3].message.role, MessageRole::Assistant);
    assert!(
        messages[3]
            .message
            .plain_text_lossy()
            .contains("second turn")
    );
}

#[tokio::test]
async fn post_turns_send_prior_transcript_as_memory_on_follow_up_turns() {
    let provider = Arc::new(RecordingMemoryProvider::default());
    let base = start_server_with_provider(provider.clone()).await;
    let client = reqwest::Client::new();
    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, &project).await;

    let first = post_turn(&client, &base, &session, "first memory turn").await;
    assert_eq!(first.status(), reqwest::StatusCode::OK);
    let first_events = parse_ndjson_events(&first.text().await.unwrap());
    assert!(matches!(
        first_events.last(),
        Some(CoreEvent::Turn {
            session_id,
            event: RuntimeEvent::TurnFinished {
                session_id: finished_session_id,
                finish_reason: Some(FinishReason::Stop),
                ..
            },
        }) if *session_id == session.id && *finished_session_id == session.id
    ));

    let second = post_turn(&client, &base, &session, "second memory turn").await;
    assert_eq!(second.status(), reqwest::StatusCode::OK);
    let second_events = parse_ndjson_events(&second.text().await.unwrap());
    assert!(matches!(
        second_events.last(),
        Some(CoreEvent::Turn {
            session_id,
            event: RuntimeEvent::TurnFinished {
                session_id: finished_session_id,
                finish_reason: Some(FinishReason::Stop),
                ..
            },
        }) if *session_id == session.id && *finished_session_id == session.id
    ));

    let recorded_requests = provider.recorded_requests();
    assert_eq!(recorded_requests.len(), 2);
    assert_eq!(
        recorded_requests[0],
        vec![(MessageRole::User, "first memory turn".to_owned())]
    );
    assert_eq!(
        recorded_requests[1],
        vec![
            (MessageRole::User, "first memory turn".to_owned()),
            (
                MessageRole::Assistant,
                "memory: first memory turn".to_owned(),
            ),
            (MessageRole::User, "second memory turn".to_owned()),
        ]
    );
}

#[tokio::test]
async fn post_turns_fall_back_to_project_default_provider_when_session_provider_is_absent() {
    let base = start_server_with_providers(
        vec![
            ("primary", Arc::new(TaggedProvider::new("primary"))),
            ("fallback", Arc::new(TaggedProvider::new("fallback"))),
        ],
        "primary",
    )
    .await;
    let client = reqwest::Client::new();
    let mut project_config = ProjectConfig::default();
    project_config.default_provider = Some("fallback".into());
    let project = create_project_with_config(&client, &base, project_config).await;
    let session = create_session(&client, &base, &project).await;

    let response = post_turn(&client, &base, &session, "hello fallback").await;

    assert_eq!(response.status(), reqwest::StatusCode::OK);

    let events = parse_ndjson_events(&response.text().await.unwrap());
    assert!(matches!(
        events.last(),
        Some(CoreEvent::Turn {
            session_id,
            event: RuntimeEvent::TurnFinished {
                session_id: finished_session_id,
                finish_reason: Some(FinishReason::Stop),
                ..
            },
        }) if *session_id == session.id && *finished_session_id == session.id
    ));

    let messages: Vec<StoredMessage> = client
        .get(format!("{base}/v1/sessions/{}/messages", session.id))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].message.role, MessageRole::User);
    assert_eq!(messages[1].message.role, MessageRole::Assistant);
    assert_eq!(
        messages[1].message.plain_text_lossy(),
        "fallback: hello fallback"
    );
}

#[tokio::test]
async fn post_turns_pick_up_hot_reloaded_project_runtime_defaults_for_existing_session() {
    let base = start_server_with_providers(
        vec![
            ("primary", Arc::new(TaggedProvider::new("primary"))),
            ("fallback", Arc::new(TaggedProvider::new("fallback"))),
        ],
        "primary",
    )
    .await;
    let client = reqwest::Client::new();
    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, &project).await;

    let initial_runtime: SessionRuntimeView = client
        .get(format!("{base}/v1/sessions/{}/runtime", session.id))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(initial_runtime.config.max_retries, 0);
    assert_eq!(initial_runtime.current_model_id, None);

    let mut reloaded_config = ProjectConfig::default();
    reloaded_config.default_provider = Some("fallback".into());
    reloaded_config.default_model = Some("fallback-echo".into());
    reloaded_config.runtime.max_retries = 2;
    reloaded_config.runtime.retry_backoff_ms = 0;
    let updated_project = update_project_config(&client, &base, &project, reloaded_config).await;
    assert_eq!(
        updated_project.config.default_provider.as_deref(),
        Some("fallback")
    );
    assert_eq!(
        updated_project.config.default_model.as_deref(),
        Some("fallback-echo")
    );
    assert_eq!(updated_project.config.runtime.max_retries, 2);

    let reloaded_runtime: SessionRuntimeView = client
        .get(format!("{base}/v1/sessions/{}/runtime", session.id))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(reloaded_runtime.config.max_retries, 2);
    assert_eq!(
        reloaded_runtime.current_model_id.as_deref(),
        Some("fallback-echo")
    );

    let response = post_turn(&client, &base, &session, "hello after reload").await;
    assert_eq!(response.status(), reqwest::StatusCode::OK);

    let messages: Vec<StoredMessage> = client
        .get(format!("{base}/v1/sessions/{}/messages", session.id))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].message.role, MessageRole::User);
    assert_eq!(messages[1].message.role, MessageRole::Assistant);
    assert_eq!(
        messages[1].message.plain_text_lossy(),
        "fallback: hello after reload"
    );
}

#[tokio::test]
async fn post_stream_turns_streams_sse_until_turn_finished() {
    let base = start_server().await;
    let client = reqwest::Client::new();
    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, &project).await;

    let response = post_stream_turn(&client, &base, &session, "hello sse turn").await;

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert!(
        response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("text/event-stream")
    );

    let events = parse_sse_events(&response.text().await.unwrap());
    assert!(!events.is_empty());
    assert!(matches!(
        events.last(),
        Some(CoreEvent::Turn {
            session_id,
            event: RuntimeEvent::TurnFinished {
                session_id: finished_session_id,
                finish_reason: Some(FinishReason::Stop),
                ..
            },
        }) if *session_id == session.id && *finished_session_id == session.id
    ));

    let messages: Vec<StoredMessage> = client
        .get(format!("{base}/v1/sessions/{}/messages", session.id))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].message.role, MessageRole::User);
    assert_eq!(messages[0].message.plain_text_lossy(), "hello sse turn");
    assert_eq!(messages[1].message.role, MessageRole::Assistant);
    assert!(
        messages[1]
            .message
            .plain_text_lossy()
            .contains("hello sse turn")
    );
}

#[tokio::test]
async fn post_stream_turns_include_detailed_usage_stats_from_provider_completed_event() {
    let _lock = OLLAMA_MOCK_SERVER_LOCK.lock().await;
    let requests_rx =
        spawn_ollama_mock_server(vec![completed_openai_sse_body_with_usage_details()]).await;

    let store: Arc<dyn Store> = Arc::new(agent_store::InMemoryStore::new());
    store
        .credentials()
        .create(
            ("ollama".into(), "cred-1".into()),
            CredentialEntry::api_key("Primary", "sk-stream"),
        )
        .await
        .unwrap();

    let core: Arc<dyn AgentCore> =
        Arc::new(AgentCoreNative::build_default_local(store).await.unwrap());
    let base = start_server_with_core(core).await;
    let client = reqwest::Client::new();
    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, &project).await;

    let response = post_stream_turn(&client, &base, &session, "collect streaming stats").await;

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert!(
        response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("text/event-stream")
    );

    let events = parse_sse_events(&response.text().await.unwrap());
    assert!(events.iter().any(|event| {
        matches!(
            event,
            CoreEvent::Turn {
                session_id,
                event: RuntimeEvent::Usage { usage },
            } if *session_id == session.id
                && usage.input_tokens == Some(11)
                && usage.output_tokens == Some(7)
                && usage.total_tokens == Some(18)
                && usage.cache_read_tokens == Some(3)
                && usage.cache_write_tokens == Some(2)
                && usage.reasoning_tokens == Some(5)
        )
    }));
    assert!(matches!(
        events.last(),
        Some(CoreEvent::Turn {
            session_id,
            event: RuntimeEvent::TurnFinished {
                session_id: finished_session_id,
                finish_reason: Some(FinishReason::Stop),
                ..
            },
        }) if *session_id == session.id && *finished_session_id == session.id
    ));

    let requests = requests_rx.await.unwrap();
    assert_eq!(requests.len(), 1);
    assert!(
        requests[0]
            .to_lowercase()
            .contains("authorization: bearer sk-stream")
    );
}

#[tokio::test]
async fn post_turns_emits_retry_event_before_turn_finished() {
    let base = start_server_with_provider(Arc::new(RetryOnceProvider::new())).await;
    let client = reqwest::Client::new();
    let mut project_config = ProjectConfig::default();
    project_config.runtime.max_retries = 1;
    project_config.runtime.retry_backoff_ms = 0;
    let project = create_project_with_config(&client, &base, project_config).await;
    let session = create_session(&client, &base, &project).await;

    let response = post_turn(&client, &base, &session, "hello after retry").await;

    assert_eq!(response.status(), reqwest::StatusCode::OK);

    let events = parse_ndjson_events(&response.text().await.unwrap());
    assert!(events.iter().any(|event| {
        matches!(
            event,
            CoreEvent::Turn {
                session_id,
                event: RuntimeEvent::Retry { attempt, max, error },
            } if *session_id == session.id
                && *attempt == 1
                && *max == 2
                && error.contains("retry after 30 seconds")
        )
    }));
    assert!(matches!(
        events.last(),
        Some(CoreEvent::Turn {
            session_id,
            event: RuntimeEvent::TurnFinished {
                session_id: finished_session_id,
                finish_reason: Some(FinishReason::Stop),
                ..
            },
        }) if *session_id == session.id && *finished_session_id == session.id
    ));

    let messages: Vec<StoredMessage> = client
        .get(format!("{base}/v1/sessions/{}/messages", session.id))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].message.role, MessageRole::User);
    assert_eq!(messages[0].message.plain_text_lossy(), "hello after retry");
    assert_eq!(messages[1].message.role, MessageRole::Assistant);
    assert!(
        messages[1]
            .message
            .plain_text_lossy()
            .contains("hello after retry")
    );
}

#[tokio::test]
async fn post_turns_emit_timeout_error_and_finish_with_error() {
    let base = start_server_with_provider(Arc::new(TimeoutProvider)).await;
    let client = reqwest::Client::new();
    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, &project).await;

    let response = post_turn(&client, &base, &session, "trigger provider timeout").await;

    assert_eq!(response.status(), reqwest::StatusCode::OK);

    let events = parse_ndjson_events(&response.text().await.unwrap());
    assert!(events.iter().any(|event| {
        matches!(
            event,
            CoreEvent::Turn {
                session_id,
                event: RuntimeEvent::Error { message, .. },
            } if *session_id == session.id && message.contains("timed out")
        )
    }));
    assert!(events.iter().any(|event| {
        matches!(
            event,
            CoreEvent::Turn {
                session_id,
                event: RuntimeEvent::TurnFinished {
                    session_id: finished_session_id,
                    finish_reason: Some(FinishReason::Error),
                    ..
                },
            } if *session_id == session.id && *finished_session_id == session.id
        )
    }));
}

#[tokio::test]
async fn post_turns_skip_tripped_circuit_breaker_credential_for_new_session() {
    let _lock = OLLAMA_MOCK_SERVER_LOCK.lock().await;
    let requests_rx = spawn_ollama_mock_server(vec![
        created_only_openai_sse_body(),
        completed_openai_sse_body(),
    ])
    .await;

    let store: Arc<dyn Store> = Arc::new(agent_store::InMemoryStore::new());
    store
        .credentials()
        .create(
            ("ollama".into(), "cred-1".into()),
            CredentialEntry::api_key("Primary", "sk-one"),
        )
        .await
        .unwrap();
    store
        .credentials()
        .create(
            ("ollama".into(), "cred-2".into()),
            CredentialEntry::api_key("Secondary", "sk-two"),
        )
        .await
        .unwrap();

    let core: Arc<dyn AgentCore> =
        Arc::new(AgentCoreNative::build_default_local(store).await.unwrap());
    let base = start_server_with_core(core).await;
    let client = reqwest::Client::new();
    let mut project_config = ProjectConfig::default();
    project_config.runtime.max_retries = 0;
    project_config.runtime.retry_backoff_ms = 0;
    let project = create_project_with_config(&client, &base, project_config).await;

    let first_session = create_session(&client, &base, &project).await;
    let first_response =
        post_turn(&client, &base, &first_session, "trip the first credential").await;

    assert_eq!(first_response.status(), reqwest::StatusCode::OK);
    let first_events = parse_ndjson_events(&first_response.text().await.unwrap());
    assert!(first_events.iter().any(|event| {
        matches!(
            event,
            CoreEvent::Turn {
                session_id,
                event: RuntimeEvent::Error { message, .. },
            } if *session_id == first_session.id && message.contains("stream closed")
        )
    }));
    assert!(first_events.iter().any(|event| {
        matches!(
            event,
            CoreEvent::Turn {
                session_id,
                event: RuntimeEvent::TurnFinished {
                    session_id: finished_session_id,
                    finish_reason: Some(FinishReason::Error),
                    ..
                },
            } if *session_id == first_session.id && *finished_session_id == first_session.id
        )
    }));

    let health_records: Vec<CredentialHealthRecord> = client
        .get(format!("{base}/v1/credentials/health"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    let first_health = health_records
        .iter()
        .find(|record| record.provider_name == "ollama" && record.credential_id == "cred-1")
        .unwrap();
    let second_health = health_records
        .iter()
        .find(|record| record.provider_name == "ollama" && record.credential_id == "cred-2")
        .unwrap();
    assert_eq!(first_health.health.consecutive_errors, 1);
    assert!(first_health.health.last_error.is_some());
    assert_eq!(second_health.health.consecutive_errors, 0);
    assert!(second_health.health.last_error.is_none());

    let second_session = create_session(&client, &base, &project).await;
    let second_response = post_turn(
        &client,
        &base,
        &second_session,
        "use the healthy credential",
    )
    .await;

    assert_eq!(second_response.status(), reqwest::StatusCode::OK);
    let second_events = parse_ndjson_events(&second_response.text().await.unwrap());
    assert!(second_events.iter().any(|event| {
        matches!(
            event,
            CoreEvent::Turn {
                session_id,
                event: RuntimeEvent::TurnFinished {
                    session_id: finished_session_id,
                    ..
                },
            } if *session_id == second_session.id && *finished_session_id == second_session.id
        )
    }));
    assert!(!second_events.iter().any(|event| {
        matches!(
            event,
            CoreEvent::Turn {
                session_id,
                event: RuntimeEvent::Error { .. },
            } if *session_id == second_session.id
        )
    }));

    let requests = requests_rx.await.unwrap();
    assert_eq!(requests.len(), 2);
    assert!(
        requests[0]
            .to_lowercase()
            .contains("authorization: bearer sk-one")
    );
    assert!(
        requests[1]
            .to_lowercase()
            .contains("authorization: bearer sk-two")
    );
}

#[tokio::test]
async fn post_cancel_cancels_active_turn_and_allows_immediate_follow_up() {
    let base = start_server_with_provider(Arc::new(MockProvider::new().with_delay(250))).await;
    let client = reqwest::Client::new();
    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, &project).await;

    let turn_response = post_turn(&client, &base, &session, "cancel this active turn").await;
    assert_eq!(turn_response.status(), reqwest::StatusCode::OK);

    let cancel_response = post_cancel_turn(&client, &base, &session).await;
    assert_eq!(cancel_response.status(), reqwest::StatusCode::OK);

    let follow_up = post_turn(&client, &base, &session, "follow up after cancel").await;
    assert_eq!(follow_up.status(), reqwest::StatusCode::OK);
    let follow_up_events = parse_ndjson_events(&follow_up.text().await.unwrap());
    assert!(matches!(
        follow_up_events.last(),
        Some(CoreEvent::Turn {
            session_id,
            event: RuntimeEvent::TurnFinished {
                session_id: finished_session_id,
                finish_reason: Some(FinishReason::Stop),
                ..
            },
        }) if *session_id == session.id && *finished_session_id == session.id
    ));

    let events = parse_ndjson_events(&turn_response.text().await.unwrap());
    assert!(events.iter().any(|event| {
        matches!(
            event,
            CoreEvent::TurnCancelled { session_id } if *session_id == session.id
        )
    }));
    assert!(!events.iter().any(|event| {
        matches!(
            event,
            CoreEvent::Turn {
                session_id,
                event: RuntimeEvent::TurnFinished { .. },
            } if *session_id == session.id
        )
    }));
}

#[tokio::test]
async fn post_cancel_is_a_no_op_for_completed_turns() {
    let base = start_server().await;
    let client = reqwest::Client::new();
    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, &project).await;

    let completed = post_turn(&client, &base, &session, "complete before cancel").await;
    assert_eq!(completed.status(), reqwest::StatusCode::OK);
    let completed_events = parse_ndjson_events(&completed.text().await.unwrap());
    assert!(matches!(
        completed_events.last(),
        Some(CoreEvent::Turn {
            session_id,
            event: RuntimeEvent::TurnFinished {
                session_id: finished_session_id,
                finish_reason: Some(FinishReason::Stop),
                ..
            },
        }) if *session_id == session.id && *finished_session_id == session.id
    ));

    let cancel_response = post_cancel_turn(&client, &base, &session).await;
    assert_eq!(cancel_response.status(), reqwest::StatusCode::OK);

    let follow_up = post_turn(&client, &base, &session, "turn after idle cancel").await;
    assert_eq!(follow_up.status(), reqwest::StatusCode::OK);
    let follow_up_events = parse_ndjson_events(&follow_up.text().await.unwrap());
    assert!(matches!(
        follow_up_events.last(),
        Some(CoreEvent::Turn {
            session_id,
            event: RuntimeEvent::TurnFinished {
                session_id: finished_session_id,
                finish_reason: Some(FinishReason::Stop),
                ..
            },
        }) if *session_id == session.id && *finished_session_id == session.id
    ));
}

#[tokio::test]
async fn session_events_stream_sse_until_turn_finished() {
    let base = start_server().await;
    let client = reqwest::Client::new();
    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, &project).await;

    let events_response = get_session_events(&client, &base, &session).await;
    assert_eq!(events_response.status(), reqwest::StatusCode::OK);
    assert!(
        events_response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("text/event-stream")
    );

    let turn_response = post_turn(&client, &base, &session, "hello session events").await;
    assert_eq!(turn_response.status(), reqwest::StatusCode::OK);

    let events = read_sse_events_until(events_response, |events| {
        events.iter().any(|event| {
            matches!(
                event,
                CoreEvent::Turn {
                    session_id,
                    event: RuntimeEvent::TurnFinished {
                        session_id: finished_session_id,
                        finish_reason: Some(FinishReason::Stop),
                        ..
                    },
                } if *session_id == session.id && *finished_session_id == session.id
            )
        })
    })
    .await;

    assert!(!events.is_empty());
    assert!(events.iter().all(|event| {
        matches!(
            event,
            CoreEvent::Turn { session_id, .. } | CoreEvent::TurnCancelled { session_id }
                if *session_id == session.id
        )
    }));
    assert!(events.iter().any(|event| {
        matches!(
            event,
            CoreEvent::Turn {
                session_id,
                event: RuntimeEvent::TurnFinished {
                    session_id: finished_session_id,
                    finish_reason: Some(FinishReason::Stop),
                    ..
                },
            } if *session_id == session.id && *finished_session_id == session.id
        )
    }));
}

#[tokio::test]
async fn session_events_stream_recovers_after_backpressure() {
    let base = start_server().await;
    let client = reqwest::Client::new();
    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, &project).await;

    let events_response = get_session_events(&client, &base, &session).await;
    assert_eq!(events_response.status(), reqwest::StatusCode::OK);
    assert!(
        events_response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("text/event-stream")
    );

    let backlog_input = std::iter::repeat_n("backpressure", 1_500)
        .collect::<Vec<_>>()
        .join(" ");
    let backlog_response = post_turn(&client, &base, &session, &backlog_input).await;
    assert_eq!(backlog_response.status(), reqwest::StatusCode::OK);
    let backlog_events = parse_ndjson_events(&backlog_response.text().await.unwrap());
    assert!(backlog_events.iter().any(|event| {
        matches!(
            event,
            CoreEvent::Turn {
                session_id,
                event: RuntimeEvent::TurnFinished {
                    session_id: finished_session_id,
                    finish_reason: Some(FinishReason::Stop),
                    ..
                },
            } if *session_id == session.id && *finished_session_id == session.id
        )
    }));

    let delayed_client = client.clone();
    let delayed_base = base.clone();
    let delayed_session = session.clone();
    let recovery_marker = "recovery-sentinel";
    let follow_up_turn = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        let response = post_turn(
            &delayed_client,
            &delayed_base,
            &delayed_session,
            recovery_marker,
        )
        .await;
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let events = parse_ndjson_events(&response.text().await.unwrap());
        assert!(events.iter().any(|event| {
            matches!(
                event,
                CoreEvent::Turn {
                    session_id,
                    event: RuntimeEvent::OutputBlockDelta {
                        delta: provider::BlockDelta::Text { text },
                        ..
                    },
                } if *session_id == delayed_session.id && text == "recovery-sentinel "
            )
        }));
        assert!(events.iter().any(|event| {
            matches!(
                event,
                CoreEvent::Turn {
                    session_id,
                    event: RuntimeEvent::TurnFinished {
                        session_id: finished_session_id,
                        finish_reason: Some(FinishReason::Stop),
                        ..
                    },
                } if *session_id == delayed_session.id && *finished_session_id == delayed_session.id
            )
        }));
    });

    let events = read_sse_events_until(events_response, |events| {
        events.iter().any(|event| {
            matches!(
                event,
                CoreEvent::Turn {
                    session_id,
                    event: RuntimeEvent::OutputBlockDelta {
                        delta: provider::BlockDelta::Text { text },
                        ..
                    },
                } if *session_id == session.id && text == "recovery-sentinel "
            )
        })
    })
    .await;

    follow_up_turn.await.unwrap();

    assert!(events.iter().any(|event| {
        matches!(
            event,
            CoreEvent::Turn {
                session_id,
                event: RuntimeEvent::OutputBlockDelta {
                    delta: provider::BlockDelta::Text { text },
                    ..
                },
            } if *session_id == session.id && text == "recovery-sentinel "
        )
    }));
    assert!(events.iter().any(|event| {
        matches!(
            event,
            CoreEvent::Turn {
                session_id,
                event: RuntimeEvent::TurnFinished {
                    session_id: finished_session_id,
                    finish_reason: Some(FinishReason::Stop),
                    ..
                },
            } if *session_id == session.id && *finished_session_id == session.id
        )
    }));
}
