use std::borrow::Cow;
use std::collections::BTreeSet;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use std::{fs, path::Path};

use agent_core::{AgentCore, AgentCoreNative, CoreEvent};
use agent_core_remote::{
    CredentialHealthRecord, ErrorResponse, ProviderCatalogEntry, ProviderModelRecord,
    TrajectoryRecord, UpdateCredentialHealthRequest,
};
use agent_runtime::RuntimeEvent;
use agent_server::{
    AgentInfoRecord, AgentServer, AgentServerStatus, HealthResponse, build_router,
    serve_with_shutdown,
};
use agent_store::{
    CredentialEntry, CredentialHealth, Project, ProviderCredential, Session, StoredMessage,
};
use futures::{StreamExt, future::join_all};
use provider::{
    Block, BlockKind, ContentBlock, Event, EventStream, FinishReason, Message, MessageRole,
    MockProvider, Provider, ProviderCapabilities, ProviderInfo, Request, StreamGranularity, Usage,
};
use provider_openai::{
    AuditLogListParams, AuditLogPage, ChatCompletionObject, Client, Config, EmbeddingInput,
    EmbeddingRequest, VectorStoreCreateRequest, VideoCreateRequest,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::oneshot;

fn make_server() -> Arc<AgentServer> {
    make_server_with_provider(Arc::new(MockProvider::new()))
}

fn make_server_with_provider(provider: Arc<dyn Provider>) -> Arc<AgentServer> {
    let core: Arc<dyn AgentCore> = Arc::new(futures::executor::block_on(async {
        AgentCoreNative::builder(Arc::new(agent_store::InMemoryStore::new()))
            .with_provider("mock", provider)
            .build()
            .await
            .unwrap()
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

async fn wait_for_server_ready(client: &reqwest::Client, base: &str) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Ok(response) = client.get(format!("{base}/v1/health")).send().await {
                if response.status().is_success() {
                    break;
                }
            }

            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}

async fn start_server_with_shutdown(
    provider: Arc<dyn Provider>,
) -> (String, oneshot::Sender<()>, tokio::task::JoinHandle<()>) {
    let server = make_server_with_provider(provider);
    let addr = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap()
    };
    let base = format!("http://{addr}");
    let addr_string = addr.to_string();
    let listen_addr = addr_string.clone();
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let server_task = tokio::spawn(async move {
        serve_with_shutdown(server, &listen_addr, async move {
            let _ = shutdown_rx.await;
        })
        .await
        .unwrap();
    });
    wait_for_server_ready(&reqwest::Client::new(), &base).await;
    (base, shutdown_tx, server_task)
}

struct BufferedTcpConnection {
    stream: tokio::net::TcpStream,
    pending: Vec<u8>,
}

impl BufferedTcpConnection {
    async fn connect(addr: std::net::SocketAddr) -> Self {
        Self {
            stream: tokio::net::TcpStream::connect(addr).await.unwrap(),
            pending: Vec::new(),
        }
    }

    async fn send(&mut self, request: &str) {
        self.stream.write_all(request.as_bytes()).await.unwrap();
    }

    async fn read_response(&mut self) -> CapturedHttpResponse {
        let header_end = loop {
            if let Some(index) = find_bytes(&self.pending, b"\r\n\r\n") {
                break index;
            }

            let mut chunk = [0_u8; 1024];
            let bytes_read = self.stream.read(&mut chunk).await.unwrap();
            assert!(
                bytes_read > 0,
                "connection closed before response headers were read"
            );
            self.pending.extend_from_slice(&chunk[..bytes_read]);
        };

        let headers = std::str::from_utf8(&self.pending[..header_end]).unwrap();
        let mut lines = headers.split("\r\n");
        let status_line = lines.next().unwrap().to_owned();
        let parsed_headers = lines
            .filter_map(|line| line.split_once(':'))
            .map(|(name, value)| (name.to_owned(), value.trim().to_owned()))
            .collect::<Vec<_>>();
        let content_length = parsed_headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
            .map(|(_, value)| value.parse::<usize>().unwrap())
            .unwrap_or_default();

        let body_start = header_end + 4;
        let body_end = body_start + content_length;
        while self.pending.len() < body_end {
            let mut chunk = [0_u8; 1024];
            let bytes_read = self.stream.read(&mut chunk).await.unwrap();
            assert!(
                bytes_read > 0,
                "connection closed before response body was read"
            );
            self.pending.extend_from_slice(&chunk[..bytes_read]);
        }

        let body = self.pending[body_start..body_end].to_vec();
        self.pending.drain(..body_end);

        CapturedHttpResponse { status_line, body }
    }
}

struct CapturedHttpResponse {
    status_line: String,
    body: Vec<u8>,
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn build_json_request(method: &str, path: &str, addr: std::net::SocketAddr, body: &str) -> String {
    format!(
        "{method} {path} HTTP/1.1\r\nhost: {addr}\r\naccept: application/json\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: keep-alive\r\n\r\n{body}",
        body.len()
    )
}

struct RateLimitedProvider;

impl Provider for RateLimitedProvider {
    fn stream<'a>(
        &'a self,
        _request: &'a Request,
    ) -> futures::future::BoxFuture<'a, Result<EventStream<'a>, provider::Error>> {
        Box::pin(async {
            Err(provider::Error::Inference(
                "429 Too Many Requests: rate limit exceeded".into(),
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
            models: vec![provider::ModelInfo {
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

struct QuotaExceededProvider;

impl Provider for QuotaExceededProvider {
    fn stream<'a>(
        &'a self,
        _request: &'a Request,
    ) -> futures::future::BoxFuture<'a, Result<EventStream<'a>, provider::Error>> {
        Box::pin(async {
            Err(provider::Error::Inference(
                "429 Too Many Requests: You exceeded your current quota, please check your plan and billing details.".into(),
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
            models: vec![provider::ModelInfo {
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

struct ThrottledProvider;

impl Provider for ThrottledProvider {
    fn stream<'a>(
        &'a self,
        _request: &'a Request,
    ) -> futures::future::BoxFuture<'a, Result<EventStream<'a>, provider::Error>> {
        Box::pin(async {
            Err(provider::Error::Inference(
                "429 Too Many Requests: request throttled, retry after 30 seconds".into(),
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
            models: vec![provider::ModelInfo {
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
            models: vec![provider::ModelInfo {
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

struct CacheUsageProvider;

impl Provider for CacheUsageProvider {
    fn stream<'a>(
        &'a self,
        request: &'a Request,
    ) -> futures::future::BoxFuture<'a, Result<EventStream<'a>, provider::Error>> {
        Box::pin(async move {
            let last_user = request.last_user_text_lossy().unwrap_or_default();
            let events = vec![
                Ok(Event::ResponseStart {
                    response_id: Some("cache-response-1".into()),
                    model: Some(request.model.clone().unwrap_or_else(|| "mock-echo".into())),
                }),
                Ok(Event::BlockStart {
                    block: Block {
                        id: "cache-text-1".into(),
                        output_index: 0,
                        kind: BlockKind::Text,
                        item_id: Some("cache-item-1".into()),
                    },
                }),
                Ok(Event::text_delta(
                    "cache-text-1",
                    format!("{last_user} from cache "),
                )),
                Ok(Event::BlockStop {
                    id: "cache-text-1".into(),
                }),
                Ok(Event::Usage {
                    usage: Usage {
                        input_tokens: Some(11),
                        output_tokens: Some(7),
                        total_tokens: Some(18),
                        cache_read_tokens: Some(5),
                        cache_write_tokens: Some(3),
                        reasoning_tokens: Some(2),
                    },
                }),
                Ok(Event::Completed {
                    response_id: Some("cache-response-1".into()),
                    finish_reason: Some(FinishReason::Stop),
                }),
            ];

            Ok(Box::pin(futures::stream::iter(events)) as EventStream<'a>)
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
            models: vec![provider::ModelInfo {
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

async fn create_project(client: &reqwest::Client, base: &str) -> Project {
    client
        .post(format!("{base}/v1/projects"))
        .json(&serde_json::json!({"name": "agent-server-test"}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap()
}

async fn create_project_with_root(client: &reqwest::Client, base: &str, root: &Path) -> Project {
    client
        .post(format!("{base}/v1/projects"))
        .json(&serde_json::json!({
            "name": "agent-server-test",
            "root": root.display().to_string(),
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

fn sample_trajectory(session: &Session) -> atif::Trajectory {
    atif::Trajectory {
        schema_version: atif::SchemaVersion::default(),
        session_id: session.id.to_string(),
        agent: atif::Agent {
            name: "brain".into(),
            version: "0.1.0".into(),
            model_name: Some("mock-echo".into()),
            tool_definitions: None,
            extra: None,
        },
        steps: vec![atif::Step {
            step_id: 1,
            timestamp: None,
            source: atif::StepSource::User,
            model_name: None,
            reasoning_effort: None,
            message: "hello trajectory".into(),
            reasoning_content: None,
            tool_calls: None,
            observation: None,
            metrics: None,
            is_copied_context: None,
            extra: None,
        }],
        notes: Some("integration test".into()),
        final_metrics: None,
        continued_trajectory_ref: None,
        extra: None,
    }
}

fn expected_agent_records() -> Vec<AgentInfoRecord> {
    [
        "plan",
        "build",
        "general",
        "explore",
        "title",
        "summary",
        "compaction",
    ]
    .into_iter()
    .map(|name| AgentInfoRecord {
        name: name.to_owned(),
        description: Some(name.to_owned()),
    })
    .collect()
}

fn parse_ndjson_events(body: &str) -> Vec<CoreEvent> {
    body.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
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

include!("http/canonical_core.rs");
include!("http/canonical_turns.rs");
include!("http/canonical_chat.rs");
include!("http/compat_surface.rs");
