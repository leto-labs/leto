use std::sync::Arc;
use std::{fs, path::Path};

use agent_core::{AgentCore, AgentCoreNative, CoreEvent};
use agent_core_remote::{
    CredentialHealthRecord, ErrorResponse, ProviderCatalogEntry, ProviderModelRecord,
    UpdateCredentialHealthRequest,
};
use agent_runtime::RuntimeEvent;
use agent_server::{AgentServer, AgentServerStatus, build_router};
use agent_store::{CredentialEntry, CredentialHealth, Project, Session, StoredMessage};
use provider::{ContentBlock, FinishReason, MessageRole, MockProvider};
use provider_openai::ChatCompletionObject;

fn make_server() -> Arc<AgentServer> {
    let core: Arc<dyn AgentCore> = Arc::new(futures::executor::block_on(async {
        AgentCoreNative::builder(Arc::new(agent_store::InMemoryStore::new()))
            .with_provider("mock", Arc::new(MockProvider::new()))
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

fn parse_ndjson_events(body: &str) -> Vec<CoreEvent> {
    body.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[tokio::test]
async fn canonical_health_route_is_available() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let health = client
        .get(format!("{base}/v1/health"))
        .send()
        .await
        .unwrap();
    assert_eq!(health.status(), reqwest::StatusCode::OK);
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
