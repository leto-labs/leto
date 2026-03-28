use std::borrow::Cow;
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
use agent_server::{AgentInfoRecord, AgentServer, AgentServerStatus, HealthResponse, build_router};
use agent_store::{
    CredentialEntry, CredentialHealth, Project, ProviderCredential, Session, StoredMessage,
};
use futures::StreamExt;
use provider::{
    Block, BlockKind, ContentBlock, Event, EventStream, FinishReason, Message, MessageRole,
    MockProvider, Provider, ProviderCapabilities, ProviderInfo, Request, StreamGranularity, Usage,
};
use provider_openai::{
    AuditLogListParams, AuditLogPage, ChatCompletionObject, Client, Config, EmbeddingInput,
    EmbeddingRequest, VectorStoreCreateRequest, VideoCreateRequest,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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

#[tokio::test]
async fn canonical_health_route_returns_current_health_payload() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let health: HealthResponse = client
        .get(format!("{base}/v1/health"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert!(health.healthy);
    assert_eq!(health.version, env!("CARGO_PKG_VERSION"));
}

#[tokio::test]
async fn metrics_routes_expose_prometheus_text_payload() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let project = create_project(&client, &base).await;
    let _session = create_session(&client, &base, &project).await;

    for path in ["/metrics", "/v1/metrics"] {
        let response = client
            .get(format!("{base}{path}"))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();

        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        let body = response.text().await.unwrap();

        assert!(content_type.starts_with("text/plain"));
        assert!(body.contains("# HELP agent_server_info"));
        assert!(body.contains("agent_server_info{"));
        assert!(body.contains("agent_server_provider_count 1"));
        assert!(body.contains("agent_server_project_count 1"));
        assert!(body.contains("agent_server_session_count 1"));
    }
}

#[tokio::test]
async fn runtime_status_returns_server_status() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let project = create_project(&client, &base).await;
    let _session = create_session(&client, &base, &project).await;

    let status: AgentServerStatus = client
        .get(format!("{base}/v1/status"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(status.provider_names, vec!["mock".to_owned()]);
    assert_eq!(status.default_provider_name, "mock");
    assert_eq!(status.default_loop_name, "simple");
    assert!(status.loop_names.contains(&status.default_loop_name));
    assert_eq!(status.project_count, 1);
    assert_eq!(status.session_count, 1);
}

#[tokio::test]
async fn canonical_agents_route_returns_agent_list() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{base}/v1/agents"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let agents: Vec<AgentInfoRecord> = response.json().await.unwrap();

    assert_eq!(agents, expected_agent_records());
}

#[tokio::test]
async fn canonical_mcp_servers_route_returns_agent_list() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let tools: Vec<AgentInfoRecord> = client
        .get(format!("{base}/v1/mcp/servers"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(tools, expected_agent_records());
}

#[tokio::test]
async fn root_health_route_is_available() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let health = client.get(format!("{base}/health")).send().await.unwrap();
    assert_eq!(health.status(), reqwest::StatusCode::OK);
}

#[tokio::test]
async fn invalid_routes_return_not_found() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let canonical = client
        .get(format!("{base}/v1/does-not-exist"))
        .send()
        .await
        .unwrap();
    assert_eq!(canonical.status(), reqwest::StatusCode::NOT_FOUND);

    let compat = client
        .get(format!("{base}/v1/compat/opencode/does-not-exist"))
        .send()
        .await
        .unwrap();
    assert_eq!(compat.status(), reqwest::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn compat_auth_endpoints_require_bearer_token() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let provider_auth = client
        .get(format!("{base}/v1/compat/opencode/provider/auth"))
        .send()
        .await
        .unwrap();
    assert_eq!(provider_auth.status(), reqwest::StatusCode::UNAUTHORIZED);

    let oauth_authorize = client
        .post(format!(
            "{base}/v1/compat/opencode/provider/mock/oauth/authorize"
        ))
        .json(&serde_json::json!({ "method": 0 }))
        .send()
        .await
        .unwrap();
    assert_eq!(oauth_authorize.status(), reqwest::StatusCode::UNAUTHORIZED);

    let auth_set = client
        .put(format!("{base}/v1/compat/opencode/auth/mock"))
        .json(&serde_json::json!({
            "type": "api",
            "key": "sk-test"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(auth_set.status(), reqwest::StatusCode::UNAUTHORIZED);

    let mcp_auth = client
        .post(format!("{base}/v1/compat/opencode/mcp/test/auth"))
        .send()
        .await
        .unwrap();
    assert_eq!(mcp_auth.status(), reqwest::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn canonical_project_and_session_routes_round_trip() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, &project).await;

    let projects: Vec<Project> = client
        .get(format!("{base}/v1/projects"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(projects.len(), 1);

    let fetched: Session = client
        .get(format!("{base}/v1/sessions/{}", session.id))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(fetched.id, session.id);
}

#[tokio::test]
async fn canonical_projects_route_returns_empty_list() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{base}/v1/projects"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);

    let projects: Vec<Project> = response.json().await.unwrap();
    assert!(projects.is_empty());
}

#[tokio::test]
async fn canonical_projects_route_lists_created_projects() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let first = create_project(&client, &base).await;
    let second: Project = client
        .post(format!("{base}/v1/projects"))
        .json(&serde_json::json!({"name": "agent-server-test-two"}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    let projects: Vec<Project> = client
        .get(format!("{base}/v1/projects"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(projects.len(), 2);
    assert!(projects.iter().any(|project| project.id == first.id));
    assert!(projects.iter().any(|project| project.id == second.id));
}

#[tokio::test]
async fn canonical_session_management_routes_cover_project_and_session_reads() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let project = create_project(&client, &base).await;
    let first_session = create_session(&client, &base, &project).await;
    let second_session = create_session(&client, &base, &project).await;

    let fetched_project: Project = client
        .get(format!("{base}/v1/projects/{}", project.id))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(fetched_project.id, project.id);

    let projects: Vec<Project> = client
        .get(format!("{base}/v1/projects"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].id, project.id);

    let project_sessions: Vec<Session> = client
        .get(format!("{base}/v1/projects/{}/sessions", project.id))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(project_sessions.len(), 2);
    assert!(
        project_sessions
            .iter()
            .any(|session| session.id == first_session.id)
    );
    assert!(
        project_sessions
            .iter()
            .any(|session| session.id == second_session.id)
    );

    let sessions: Vec<Session> = client
        .get(format!("{base}/v1/sessions"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(sessions.len(), 2);
    assert!(
        sessions
            .iter()
            .any(|session| session.id == first_session.id)
    );
    assert!(
        sessions
            .iter()
            .any(|session| session.id == second_session.id)
    );

    let fetched_session: Session = client
        .get(format!("{base}/v1/sessions/{}", first_session.id))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(fetched_session.id, first_session.id);
    assert_eq!(fetched_session.project_id, project.id);
}

#[tokio::test]
async fn canonical_delete_session_route_removes_session_and_related_records() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, &project).await;
    let sibling_session = create_session(&client, &base, &project).await;
    let trajectory = sample_trajectory(&session);
    let message = StoredMessage::new(session.id, 0, Message::user_text("delete me"));

    let stored_messages: Vec<StoredMessage> = client
        .put(format!("{base}/v1/sessions/{}/messages", session.id))
        .json(&vec![message.clone()])
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(stored_messages, vec![message]);

    let stored_trajectory: atif::Trajectory = client
        .put(format!("{base}/v1/sessions/{}/trajectory", session.id))
        .json(&trajectory)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(stored_trajectory, trajectory);

    let response = client
        .delete(format!("{base}/v1/sessions/{}", session.id))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::NO_CONTENT);

    let fetch_deleted = client
        .get(format!("{base}/v1/sessions/{}", session.id))
        .send()
        .await
        .unwrap();
    assert_eq!(fetch_deleted.status(), reqwest::StatusCode::NOT_FOUND);
    let error: ErrorResponse = fetch_deleted.json().await.unwrap();
    assert_eq!(error.error.code, "store_not_found");
    assert!(error.error.message.contains(&session.id.to_string()));

    let project_sessions: Vec<Session> = client
        .get(format!("{base}/v1/projects/{}/sessions", project.id))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(project_sessions.len(), 1);
    assert_eq!(project_sessions[0].id, sibling_session.id);

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
    assert!(messages.is_empty());

    let fetched_trajectory: Option<atif::Trajectory> = client
        .get(format!("{base}/v1/sessions/{}/trajectory", session.id))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(fetched_trajectory, None);
}

#[tokio::test]
async fn canonical_trajectory_route_round_trips() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, &project).await;
    let trajectory = sample_trajectory(&session);

    let missing: Option<atif::Trajectory> = client
        .get(format!("{base}/v1/sessions/{}/trajectory", session.id))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(missing, None);

    let stored: atif::Trajectory = client
        .put(format!("{base}/v1/sessions/{}/trajectory", session.id))
        .json(&trajectory)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(stored, trajectory);

    let fetched: Option<atif::Trajectory> = client
        .get(format!("{base}/v1/sessions/{}/trajectory", session.id))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(fetched, Some(trajectory));
}

#[tokio::test]
async fn canonical_trajectories_route_lists_stored_trajectories() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let project = create_project(&client, &base).await;
    let first_session = create_session(&client, &base, &project).await;
    let second_session = create_session(&client, &base, &project).await;
    let first_trajectory = sample_trajectory(&first_session);
    let second_trajectory = sample_trajectory(&second_session);

    client
        .put(format!(
            "{base}/v1/sessions/{}/trajectory",
            first_session.id
        ))
        .json(&first_trajectory)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    client
        .put(format!(
            "{base}/v1/sessions/{}/trajectory",
            second_session.id
        ))
        .json(&second_trajectory)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let mut trajectories: Vec<TrajectoryRecord> = client
        .get(format!("{base}/v1/trajectories"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    trajectories.sort_by_key(|record| record.session_id);

    let mut expected = vec![
        TrajectoryRecord {
            session_id: first_session.id,
            trajectory: first_trajectory,
        },
        TrajectoryRecord {
            session_id: second_session.id,
            trajectory: second_trajectory,
        },
    ];
    expected.sort_by_key(|record| record.session_id);

    assert_eq!(trajectories, expected);
}

#[tokio::test]
async fn canonical_providers_route_returns_provider_inventory() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let providers: Vec<ProviderCatalogEntry> = client
        .get(format!("{base}/v1/providers"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(providers.len(), 1);
    assert_eq!(providers[0].name, "mock");
    assert_eq!(providers[0].model_ids, vec!["mock-echo".to_owned()]);
}

#[tokio::test]
async fn canonical_health_route_supports_keep_alive_connection_reuse() {
    let server = make_server();
    let router = build_router(server);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    let mut connection = BufferedTcpConnection::connect(addr).await;
    let request = format!(
        "GET /v1/health HTTP/1.1\r\nhost: {addr}\r\naccept: application/json\r\nconnection: keep-alive\r\n\r\n"
    );

    connection.send(&request).await;
    let first_response = connection.read_response().await;
    connection.send(&request).await;
    let second_response = connection.read_response().await;

    assert!(first_response.status_line.contains("200 OK"));
    assert!(second_response.status_line.contains("200 OK"));

    let first_health: HealthResponse = serde_json::from_slice(&first_response.body).unwrap();
    let second_health: HealthResponse = serde_json::from_slice(&second_response.body).unwrap();
    assert!(first_health.healthy);
    assert!(second_health.healthy);
    assert_eq!(first_health.version, env!("CARGO_PKG_VERSION"));
    assert_eq!(second_health.version, env!("CARGO_PKG_VERSION"));
}

#[tokio::test]
async fn canonical_credential_health_route_returns_health_records() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let credential = CredentialEntry::api_key("mock-key", "sk-test");
    client
        .post(format!("{base}/v1/credentials/mock/mock-key"))
        .json(&credential)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let mut health = CredentialHealth::default();
    health.record_error("bad gateway", Some("502".into()));
    client
        .patch(format!("{base}/v1/credentials/mock/mock-key/health"))
        .json(&UpdateCredentialHealthRequest {
            health: health.clone(),
        })
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let records: Vec<CredentialHealthRecord> = client
        .get(format!("{base}/v1/credentials/health"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].provider_name, "mock");
    assert_eq!(records[0].credential_id, "mock-key");
    assert_eq!(records[0].health.consecutive_errors, 1);
    assert_eq!(
        records[0]
            .health
            .last_error
            .as_ref()
            .and_then(|error| error.code.as_deref()),
        Some("502")
    );
}

#[tokio::test]
async fn canonical_models_route_returns_provider_inventory() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let models: Vec<ProviderModelRecord> = client
        .get(format!("{base}/v1/models"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(models.len(), 1);
    assert_eq!(models[0].provider_name, "mock");
    assert_eq!(models[0].model.id, "mock-echo");
}

#[tokio::test]
async fn canonical_model_route_returns_model_by_id() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let model: ProviderModelRecord = client
        .get(format!("{base}/v1/models/mock-echo"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(model.provider_name, "mock");
    assert_eq!(model.model.id, "mock-echo");
}

#[tokio::test]
async fn canonical_model_route_returns_not_found_for_unknown_id() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{base}/v1/models/does-not-exist"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
    let error: ErrorResponse = response.json().await.unwrap();
    assert_eq!(error.error.code, "model_not_found");
}

#[tokio::test]
async fn canonical_turn_endpoint_streams_ndjson() {
    let base = start_server().await;
    let client = reqwest::Client::new();
    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, &project).await;

    let response = client
        .post(format!("{base}/v1/sessions/{}/turns", session.id))
        .json(&serde_json::json!({
            "input": [
                {
                    "role": "user",
                    "content": [{"type": "text", "text": "hello canonical server"}]
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .unwrap(),
        "application/x-ndjson"
    );
    let body = response.text().await.unwrap();
    assert!(body.contains("turn_finished"));
}

#[tokio::test]
async fn canonical_batch_turns_endpoint_runs_turns_sequentially() {
    let base = start_server().await;
    let client = reqwest::Client::new();
    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, &project).await;

    let response = client
        .post(format!("{base}/v1/sessions/{}/batch-turns", session.id))
        .json(&serde_json::json!({
            "turns": [
                {
                    "input": [
                        {
                            "role": "user",
                            "content": [{"type": "text", "text": "first batch turn"}]
                        }
                    ]
                },
                {
                    "input": [
                        {
                            "role": "user",
                            "content": [{"type": "text", "text": "second batch turn"}]
                        }
                    ]
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .unwrap(),
        "application/x-ndjson"
    );

    let events = parse_ndjson_events(&response.text().await.unwrap());
    let finished = events
        .iter()
        .filter(|event| {
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
        .count();
    assert_eq!(finished, 2);
}

#[tokio::test]
async fn canonical_batch_turns_endpoint_emits_turn_lifecycle_events_in_order() {
    let base = start_server().await;
    let client = reqwest::Client::new();
    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, &project).await;

    let response = client
        .post(format!("{base}/v1/sessions/{}/batch-turns", session.id))
        .json(&serde_json::json!({
            "turns": [
                {
                    "input": [
                        {
                            "role": "user",
                            "content": [{"type": "text", "text": "first ordered batch turn"}]
                        }
                    ]
                },
                {
                    "input": [
                        {
                            "role": "user",
                            "content": [{"type": "text", "text": "second ordered batch turn"}]
                        }
                    ]
                }
            ]
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let lifecycle = parse_ndjson_events(&response.text().await.unwrap())
        .into_iter()
        .filter_map(|event| match event {
            CoreEvent::Turn {
                session_id,
                event:
                    RuntimeEvent::TurnStarted {
                        session_id: started_session_id,
                        turn_index,
                    },
            } if session_id == session.id && started_session_id == session.id => {
                Some(("started", turn_index))
            }
            CoreEvent::Turn {
                session_id,
                event:
                    RuntimeEvent::TurnFinished {
                        session_id: finished_session_id,
                        turn_index,
                        ..
                    },
            } if session_id == session.id && finished_session_id == session.id => {
                Some(("finished", turn_index))
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(
        lifecycle,
        vec![
            ("started", 1_u64),
            ("finished", 1_u64),
            ("started", 1_u64),
            ("finished", 1_u64),
        ]
    );
}

#[tokio::test]
async fn canonical_tool_calls_route_appends_tool_call_history() {
    let base = start_server().await;
    let client = reqwest::Client::new();
    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, &project).await;

    let appended: Vec<StoredMessage> = client
        .post(format!("{base}/v1/sessions/{}/tool-calls", session.id))
        .json(&serde_json::json!({
            "calls": [
                {
                    "id": "call_1",
                    "name": "echo",
                    "input": {"text": "hello tool"},
                    "output": {"text": "tool output"},
                    "is_error": false
                }
            ]
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(appended.len(), 2);
    assert_eq!(appended[0].session_id, session.id);
    assert_eq!(appended[0].ordinal, 0);
    assert_eq!(appended[0].message.role, MessageRole::Assistant);
    assert!(matches!(
        appended[0].message.content.as_slice(),
        [ContentBlock::ToolCall { id, name, input }]
            if id == "call_1" && name == "echo" && input == &serde_json::json!({"text": "hello tool"})
    ));
    assert_eq!(appended[1].session_id, session.id);
    assert_eq!(appended[1].ordinal, 1);
    assert_eq!(appended[1].message.role, MessageRole::User);
    assert!(matches!(
        appended[1].message.content.as_slice(),
        [ContentBlock::ToolResult {
            call_id,
            output,
            is_error: Some(false),
        }] if call_id == "call_1" && output == &serde_json::json!({"text": "tool output"})
    ));

    let history: Vec<StoredMessage> = client
        .get(format!("{base}/v1/sessions/{}/messages", session.id))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(history, appended);
}

#[tokio::test]
async fn canonical_chat_completions_route_returns_non_streaming_completion() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let completion: ChatCompletionObject = client
        .post(format!("{base}/v1/chat/completions"))
        .json(&serde_json::json!({
            "model": "mock-echo",
            "messages": [
                {
                    "role": "user",
                    "content": "hello chat completions"
                }
            ]
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(completion.object.as_deref(), Some("chat.completion"));
    assert_eq!(completion.model.as_deref(), Some("mock-echo"));
    assert_eq!(completion.choices.len(), 1);
    assert_eq!(
        completion.choices[0].message.role,
        provider_openai::ChatCompletionRole::Assistant
    );
    assert!(
        completion.choices[0]
            .message
            .content
            .as_ref()
            .is_some_and(|content| match content {
                provider_openai::ChatCompletionMessageContent::Text(text) => {
                    text.contains("hello chat completions")
                }
                provider_openai::ChatCompletionMessageContent::Parts(_) => false,
            })
    );
}

#[tokio::test]
async fn canonical_chat_completions_route_returns_cache_usage() {
    let base = start_server_with_provider(Arc::new(CacheUsageProvider)).await;
    let client = reqwest::Client::new();

    let completion: ChatCompletionObject = client
        .post(format!("{base}/v1/chat/completions"))
        .json(&serde_json::json!({
            "model": "mock-echo",
            "messages": [
                {
                    "role": "user",
                    "content": "hello cached completions"
                }
            ]
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    let usage = completion
        .usage
        .expect("chat completions should include usage");
    assert_eq!(usage.prompt, 11);
    assert_eq!(usage.completion, 7);
    assert_eq!(usage.total, 18);
    assert_eq!(usage.cache_read, Some(5));
    assert_eq!(usage.cache_write, Some(3));
    assert_eq!(usage.reasoning, Some(2));
}

#[tokio::test]
async fn canonical_chat_completions_route_surfaces_rate_limited_provider_errors() {
    let base = start_server_with_provider(Arc::new(RateLimitedProvider)).await;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{base}/v1/chat/completions"))
        .json(&serde_json::json!({
            "model": "mock-echo",
            "messages": [
                {
                    "role": "user",
                    "content": "trigger provider rate limit"
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::BAD_GATEWAY);
    let error: ErrorResponse = response.json().await.unwrap();
    assert_eq!(error.error.code, "runtime_error");
    assert!(error.error.message.contains("429 Too Many Requests"));
    assert!(error.error.message.contains("rate limit exceeded"));
}

#[tokio::test]
async fn canonical_chat_completions_route_surfaces_quota_exceeded_provider_errors() {
    let base = start_server_with_provider(Arc::new(QuotaExceededProvider)).await;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{base}/v1/chat/completions"))
        .json(&serde_json::json!({
            "model": "mock-echo",
            "messages": [
                {
                    "role": "user",
                    "content": "trigger provider quota exceeded"
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::BAD_GATEWAY);
    let error: ErrorResponse = response.json().await.unwrap();
    assert_eq!(error.error.code, "runtime_error");
    assert!(error.error.message.contains("429 Too Many Requests"));
    assert!(error.error.message.contains("current quota"));
}

#[tokio::test]
async fn canonical_chat_completions_route_surfaces_throttled_provider_errors() {
    let base = start_server_with_provider(Arc::new(ThrottledProvider)).await;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{base}/v1/chat/completions"))
        .json(&serde_json::json!({
            "model": "mock-echo",
            "messages": [
                {
                    "role": "user",
                    "content": "trigger provider throttle"
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::BAD_GATEWAY);
    let error: ErrorResponse = response.json().await.unwrap();
    assert_eq!(error.error.code, "runtime_error");
    assert!(error.error.message.contains("429 Too Many Requests"));
    assert!(error.error.message.contains("request throttled"));
}

#[tokio::test]
async fn canonical_chat_completions_route_surfaces_timeout_provider_errors() {
    let base = start_server_with_provider(Arc::new(TimeoutProvider)).await;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{base}/v1/chat/completions"))
        .json(&serde_json::json!({
            "model": "mock-echo",
            "messages": [
                {
                    "role": "user",
                    "content": "trigger provider timeout"
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::BAD_GATEWAY);
    let error: ErrorResponse = response.json().await.unwrap();
    assert_eq!(error.error.code, "runtime_error");
    assert!(error.error.message.contains("timed out"));
}

#[tokio::test]
async fn canonical_audit_logs_route_returns_openai_style_payload() {
    let base = start_server().await;
    let client = Client::new(Config::new("sk-test").with_base_url(format!("{base}/v1")));

    let page: AuditLogPage = client.audit_logs().list().await.unwrap();

    assert_eq!(page.object, "list");
    assert!(!page.has_more);
    assert_eq!(page.data.len(), 2);
    assert_eq!(page.data[0].id, "req_agent_server_20240301");
    assert_eq!(page.data[0].event_type, "api_key.created");
    assert_eq!(page.data[0].effective_at, 1_720_804_090_i64);
    assert_eq!(
        page.data[0]
            .project
            .as_ref()
            .map(|project| project.id.as_str()),
        Some("proj_agent_server")
    );
    assert_eq!(
        page.data[0]
            .actor
            .session
            .as_ref()
            .map(|session| session.user.email.as_str()),
        Some("agent@example.com")
    );
    assert!(page.data[0].extra.contains_key("api_key.created"));
}

#[tokio::test]
async fn canonical_audit_logs_route_supports_pagination_limit() {
    let base = start_server().await;
    let client = Client::new(Config::new("sk-test").with_base_url(format!("{base}/v1")));

    let page: AuditLogPage = client
        .audit_logs()
        .list_with_params(&AuditLogListParams {
            after: None,
            before: None,
            limit: Some(1),
        })
        .await
        .unwrap();

    assert_eq!(page.object, "list");
    assert!(page.has_more);
    assert_eq!(page.data.len(), 1);
    assert_eq!(page.data[0].id, "req_agent_server_20240301");
    assert_eq!(page.data[0].event_type, "api_key.created");
}

#[tokio::test]
async fn canonical_embeddings_route_returns_embedding_vectors() {
    let base = start_server().await;
    let client = Client::new(
        Config::new("sk-test")
            .with_base_url(format!("{base}/v1"))
            .with_model("mock-echo"),
    );

    let response = client
        .embeddings()
        .create(&EmbeddingRequest {
            input: EmbeddingInput::Text("hello embeddings".into()),
            ..EmbeddingRequest::default()
        })
        .await
        .unwrap();

    assert_eq!(response.object, "list");
    assert_eq!(response.model.as_deref(), Some("mock-echo"));
    assert_eq!(response.data[0].object, "embedding");
    assert_eq!(response.data[0].index, 0);

    let embedding = &response.data[0].embedding;
    assert_eq!(embedding.len(), 8);
    assert!(embedding.iter().all(|value| value.is_finite()));

    assert_eq!(
        response.usage.as_ref().map(|usage| usage.prompt_tokens),
        Some(4)
    );
    assert_eq!(
        response.usage.as_ref().map(|usage| usage.total_tokens),
        Some(4)
    );
}

#[tokio::test]
async fn canonical_vector_stores_route_returns_openai_style_payload() {
    let base = start_server().await;
    let client = Client::new(Config::new("sk-test").with_base_url(format!("{base}/v1")));

    let response = client
        .vector_stores()
        .create(&VectorStoreCreateRequest {
            name: Some("  Support FAQ  ".into()),
            description: Some("  Contains support answers  ".into()),
            metadata: Default::default(),
        })
        .await
        .unwrap();

    assert!(response.id.starts_with("vs_"));
    assert_eq!(response.object, "vector_store");
    assert_eq!(response.name.as_deref(), Some("Support FAQ"));
    assert_eq!(
        response.description.as_deref(),
        Some("Contains support answers")
    );
    assert_eq!(response.bytes, Some(0));
    assert_eq!(
        response.file_counts.as_ref().map(|counts| counts.total),
        Some(0)
    );
    assert!(response.created_at.is_some_and(|created_at| created_at > 0));
}

#[tokio::test]
async fn canonical_image_generation_route_returns_base64_images() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let response: serde_json::Value = client
        .post(format!("{base}/v1/images/generations"))
        .json(&serde_json::json!({
            "model": "mock-echo",
            "prompt": "  hello   image  world ",
            "n": 2,
            "response_format": "b64_json"
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert!(
        response["created"]
            .as_i64()
            .is_some_and(|created| created > 0)
    );

    let data = response["data"].as_array().unwrap();
    assert_eq!(data.len(), 2);
    assert!(data.iter().all(|item| {
        item["b64_json"]
            .as_str()
            .is_some_and(|image| !image.trim().is_empty())
            && item["revised_prompt"].as_str() == Some("hello image world")
    }));
}

#[tokio::test]
async fn canonical_video_route_returns_openai_style_payload() {
    let base = start_server().await;
    let client = Client::new(
        Config::new("sk-test")
            .with_base_url(format!("{base}/v1"))
            .with_model("mock-echo"),
    );

    let response = client
        .videos()
        .create(&VideoCreateRequest {
            prompt: "a calico cat playing piano".into(),
            seconds: Some("8".into()),
            size: Some("720x1280".into()),
            ..VideoCreateRequest::default()
        })
        .await
        .unwrap();

    assert_eq!(response.object, "video");
    assert_eq!(response.id, "video_agent_server");
    assert_eq!(response.model.as_deref(), Some("mock-echo"));
    assert_eq!(response.status.as_deref(), Some("queued"));
    assert_eq!(response.progress, Some(0));
    assert_eq!(response.seconds.as_deref(), Some("8"));
    assert_eq!(response.size.as_deref(), Some("720x1280"));
    assert_eq!(response.quality.as_deref(), Some("standard"));
    assert!(response.created_at.is_some_and(|created_at| created_at > 0));
}

#[tokio::test]
async fn canonical_moderations_route_returns_openai_style_payload() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let response: serde_json::Value = client
        .post(format!("{base}/v1/moderations"))
        .json(&serde_json::json!({
            "model": "mock-echo",
            "input": "please moderate this text"
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(response["id"].as_str(), Some("modr-agent-server"));
    assert_eq!(response["model"].as_str(), Some("mock-echo"));

    let results = response["results"].as_array().unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["flagged"].as_bool(), Some(false));
    assert_eq!(results[0]["categories"]["violence"].as_bool(), Some(false));
    assert_eq!(
        results[0]["category_scores"]["violence"].as_f64(),
        Some(0.0)
    );
}

#[tokio::test]
async fn canonical_and_compat_event_endpoints_are_sse() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let canonical = client
        .get(format!("{base}/v1/events"))
        .send()
        .await
        .unwrap();
    assert_eq!(canonical.status(), reqwest::StatusCode::OK);
    assert!(
        canonical
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("text/event-stream")
    );

    let compat = client
        .get(format!("{base}/v1/compat/opencode/global/event"))
        .send()
        .await
        .unwrap();
    assert_eq!(compat.status(), reqwest::StatusCode::OK);
    assert!(
        compat
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("text/event-stream")
    );
}

#[tokio::test]
async fn canonical_event_endpoint_streams_runtime_events() {
    let base = start_server().await;
    let client = reqwest::Client::new();
    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, &project).await;

    let response = client
        .get(format!("{base}/v1/events"))
        .send()
        .await
        .unwrap();
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

    let turn_response = client
        .post(format!("{base}/v1/sessions/{}/turns", session.id))
        .json(&serde_json::json!({
            "input": [
                {
                    "role": "user",
                    "content": [{"type": "text", "text": "hello runtime bus"}]
                }
            ]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(turn_response.status(), reqwest::StatusCode::OK);

    let events = read_sse_events_until(response, |events| {
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
async fn compat_config_provider_and_prompt_routes_are_real() {
    let base = start_server().await;
    let client = reqwest::Client::new();
    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, &project).await;

    let health = client
        .get(format!("{base}/v1/compat/opencode/global/health"))
        .send()
        .await
        .unwrap();
    assert_eq!(health.status(), reqwest::StatusCode::OK);

    let config = client
        .get(format!("{base}/v1/compat/opencode/config"))
        .send()
        .await
        .unwrap();
    assert_eq!(config.status(), reqwest::StatusCode::OK);

    let provider_auth = client
        .get(format!("{base}/v1/compat/opencode/provider/auth"))
        .header(reqwest::header::AUTHORIZATION, "Bearer test-token")
        .send()
        .await
        .unwrap();
    assert_eq!(provider_auth.status(), reqwest::StatusCode::OK);

    let prompt = client
        .post(format!(
            "{base}/v1/compat/opencode/session/{}/message",
            session.id
        ))
        .json(&serde_json::json!({
            "parts": [{"type": "text", "text": "hello compat"}]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(prompt.status(), reqwest::StatusCode::OK);

    let prompt_async = client
        .post(format!(
            "{base}/v1/compat/opencode/session/{}/prompt_async",
            session.id
        ))
        .json(&serde_json::json!({
            "parts": [{"type": "text", "text": "hello async compat"}]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(prompt_async.status(), reqwest::StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn compat_provider_oauth_callback_persists_oauth_credential() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let callback = client
        .post(format!(
            "{base}/v1/compat/opencode/provider/mock/oauth/callback"
        ))
        .header(reqwest::header::AUTHORIZATION, "Bearer test-token")
        .json(&serde_json::json!({
            "method": 0,
            "code": "oauth-code"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(callback.status(), reqwest::StatusCode::OK);
    assert_eq!(callback.json::<bool>().await.unwrap(), true);

    let credential: CredentialEntry = client
        .get(format!("{base}/v1/credentials/mock/default"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(credential.id, "default");
    assert_eq!(credential.label, "mock");
    assert!(credential.enabled);
    match credential.credential {
        ProviderCredential::OAuth(oauth) => {
            assert_eq!(oauth.access_token, "oauth-code");
            assert_eq!(oauth.refresh_token, "compat-refresh");
            assert_eq!(oauth.client_id, "compat-client");
            assert_eq!(oauth.token_endpoint, "https://example.invalid/oauth/token");
            assert_eq!(oauth.token_type.as_deref(), Some("bearer"));
            assert_eq!(oauth.account_id, None);
            assert!(oauth.expires_at > chrono::Utc::now());
        }
        other => panic!("expected oauth credential, got {other:?}"),
    }
}

#[tokio::test]
async fn compat_assistant_endpoint_returns_assistant_message_with_parts() {
    let base = start_server().await;
    let client = reqwest::Client::new();
    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, &project).await;

    let assistant: serde_json::Value = client
        .post(format!(
            "{base}/v1/compat/opencode/session/{}/message",
            session.id
        ))
        .json(&serde_json::json!({
            "parts": [{"type": "text", "text": "hello compat assistant"}]
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    let assistant_id = assistant["info"]["id"]
        .as_str()
        .expect("compat assistant response should include a message id")
        .to_owned();

    assert_eq!(assistant["info"]["role"], "assistant");
    assert_eq!(assistant["info"]["sessionID"], format!("ses{}", session.id));
    assert_eq!(assistant["info"]["modelID"], "compat");
    assert_eq!(assistant["info"]["providerID"], "compat");
    assert_eq!(assistant["info"]["agent"], "general");
    assert_eq!(assistant["info"]["mode"], "chat");
    assert!(
        assistant_id.starts_with("msg"),
        "compat assistant message ids should use the msg prefix"
    );

    let parts = assistant["parts"]
        .as_array()
        .expect("compat assistant response should include message parts");
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0]["type"], "text");
    assert_eq!(parts[0]["text"], "hello compat assistant ");
    assert_eq!(parts[0]["sessionID"], format!("ses{}", session.id));
    assert_eq!(parts[0]["messageID"], assistant_id);

    let fetched: serde_json::Value = client
        .get(format!(
            "{base}/v1/compat/opencode/session/{}/message/{}",
            session.id, assistant_id
        ))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(fetched["info"]["id"], assistant["info"]["id"]);
    assert_eq!(fetched["info"]["role"], "assistant");
    assert_eq!(fetched["parts"], assistant["parts"]);
}

#[tokio::test]
async fn compat_thread_endpoints_create_list_fetch_and_fork_sessions() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let temp_root = env::temp_dir().join(format!(
        "agent-server-thread-endpoint-{}",
        ulid::Ulid::new()
    ));
    fs::create_dir_all(&temp_root).unwrap();

    let created: serde_json::Value = client
        .post(format!("{base}/v1/compat/opencode/session"))
        .query(&[("directory", temp_root.display().to_string())])
        .json(&serde_json::json!({
            "title": "Compat thread",
            "workspaceID": "wrk_thread_test"
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    let session_id = created["id"]
        .as_str()
        .expect("compat thread create response should include a session id")
        .to_owned();

    assert!(
        session_id.starts_with("ses"),
        "compat thread session ids should use the ses prefix"
    );
    assert_eq!(created["title"], "Compat thread");
    assert_eq!(created["workspaceID"], "wrk_thread_test");
    assert_eq!(created["directory"], temp_root.display().to_string());
    assert_eq!(created["parentID"], serde_json::Value::Null);

    let listed: Vec<serde_json::Value> = client
        .get(format!("{base}/v1/compat/opencode/session"))
        .query(&[("directory", temp_root.display().to_string())])
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0]["id"], created["id"]);
    assert_eq!(listed[0]["title"], created["title"]);
    assert_eq!(listed[0]["directory"], created["directory"]);

    let fetched: serde_json::Value = client
        .get(format!("{base}/v1/compat/opencode/session/{session_id}"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(fetched["id"], created["id"]);
    assert_eq!(fetched["title"], created["title"]);
    assert_eq!(fetched["workspaceID"], created["workspaceID"]);
    assert_eq!(fetched["directory"], created["directory"]);

    let forked: serde_json::Value = client
        .post(format!(
            "{base}/v1/compat/opencode/session/{session_id}/fork"
        ))
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_ne!(forked["id"], created["id"]);
    assert_eq!(forked["parentID"], created["id"]);
    assert_eq!(forked["title"], created["title"]);
    assert_eq!(forked["directory"], created["directory"]);

    let children: Vec<serde_json::Value> = client
        .get(format!(
            "{base}/v1/compat/opencode/session/{session_id}/children"
        ))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(children.len(), 1);
    assert_eq!(children[0]["id"], forked["id"]);
    assert_eq!(children[0]["parentID"], created["id"]);
    assert_eq!(children[0]["title"], created["title"]);
}

#[tokio::test]
async fn compat_file_routes_list_directory_and_read_file_content() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let temp_root =
        env::temp_dir().join(format!("agent-server-file-endpoint-{}", ulid::Ulid::new()));
    let nested_dir = temp_root.join("nested");
    let file_path = temp_root.join("notes.txt");
    fs::create_dir_all(&nested_dir).unwrap();
    fs::write(&file_path, "hello from compat file route\n").unwrap();

    let file_list: serde_json::Value = client
        .get(format!("{base}/v1/compat/opencode/file"))
        .query(&[("path", temp_root.display().to_string())])
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    let entries = file_list.as_array().unwrap();
    assert!(entries.iter().any(|entry| {
        entry["name"] == "notes.txt"
            && entry["absolute"] == serde_json::json!(file_path.display().to_string())
            && entry["type"] == "file"
    }));
    assert!(entries.iter().any(|entry| {
        entry["name"] == "nested"
            && entry["absolute"] == serde_json::json!(nested_dir.display().to_string())
            && entry["type"] == "directory"
    }));

    let file_content: serde_json::Value = client
        .get(format!("{base}/v1/compat/opencode/file/content"))
        .query(&[("path", file_path.display().to_string())])
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(file_content["type"], "text");
    assert_eq!(file_content["content"], "hello from compat file route\n");
    assert_eq!(file_content["mimeType"], "text/plain");

    fs::remove_dir_all(&temp_root).unwrap();
}

#[tokio::test]
async fn compat_doc_serves_raw_aide_openapi() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let remote: serde_json::Value = client
        .get(format!("{base}/v1/compat/opencode/doc"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(remote["openapi"], serde_json::json!("3.1.1"));
    assert_eq!(remote["info"]["title"], serde_json::json!("opencode"));
    assert_eq!(
        remote["info"]["description"],
        serde_json::json!("opencode api")
    );
    assert_eq!(remote["info"]["version"], serde_json::json!("0.0.3"));

    let paths = remote["paths"]
        .as_object()
        .expect("compat /doc should return OpenAPI paths");
    assert!(paths.contains_key("/project"));
    assert!(paths.contains_key("/session"));
    assert!(paths.contains_key("/global/health"));
    assert!(!paths.contains_key("/doc"));
    assert!(!paths.contains_key("/v1/compat/opencode/project"));
}

#[test]
fn compat_runtime_does_not_import_pinned_openapi_contract() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/compat/opencode");

    fn scan(path: &Path) {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                scan(&path);
                continue;
            }
            if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
                continue;
            }
            // `test_utils.rs` is compiled only under `#[cfg(test)]` and is
            // allowed to load the pinned contract for parity tests.
            if path.file_name().and_then(|name| name.to_str()) == Some("test_utils.rs") {
                continue;
            }
            let source = fs::read_to_string(&path).unwrap();
            assert!(
                !source.contains("openapi/opencode.json"),
                "runtime compat source must not reference openapi/opencode.json: {}",
                path.display()
            );
            assert!(
                !source.contains("include_str!(\"../../../../../openapi/opencode.json\")"),
                "runtime compat source must not embed openapi/opencode.json: {}",
                path.display()
            );
        }
    }

    scan(&root);
}

#[tokio::test]
async fn compat_preflight_allows_browser_requests() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let response = client
        .request(
            reqwest::Method::OPTIONS,
            format!("{base}/v1/compat/opencode/config"),
        )
        .header(reqwest::header::ORIGIN, "http://localhost:3000")
        .header(reqwest::header::ACCESS_CONTROL_REQUEST_METHOD, "PATCH")
        .header(
            reqwest::header::ACCESS_CONTROL_REQUEST_HEADERS,
            "authorization,content-type",
        )
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success());
    assert!(
        response
            .headers()
            .get(reqwest::header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .is_some()
    );
}
