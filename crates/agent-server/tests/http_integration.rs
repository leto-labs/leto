use std::sync::Arc;

use agent_core::{AgentCore, AgentCoreNative};
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
async fn compat_health_is_real_and_prompt_async_is_stubbed() {
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

    let stub = client
        .post(format!(
            "{base}/v1/compat/opencode/session/{}/prompt_async",
            session.id
        ))
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(stub.status(), reqwest::StatusCode::NOT_IMPLEMENTED);

    let payload: serde_json::Value = stub.json().await.unwrap();
    assert_eq!(payload["error"]["code"], "not_implemented");
    assert_eq!(payload["error"]["stub"], true);
}
