use std::sync::Arc;
use std::{fs, path::Path};

use agent_core::{AgentCore, AgentCoreNative};
use agent_core_remote::ProviderModelRecord;
use agent_server::{AgentServer, build_router};
use agent_store::{Project, Session};
use provider::MockProvider;

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

#[tokio::test]
async fn canonical_health_and_status_are_available() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let health = client
        .get(format!("{base}/v1/health"))
        .send()
        .await
        .unwrap();
    assert_eq!(health.status(), reqwest::StatusCode::OK);

    let status = client
        .get(format!("{base}/v1/status"))
        .send()
        .await
        .unwrap();
    assert_eq!(status.status(), reqwest::StatusCode::OK);
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
