use std::sync::Arc;

use brain_core::Brain;
use brain_loops::SimpleLoop;
use brain_providers::MockProvider;
use brain_server::{build_router, BrainServer, ServerEvent, ServerStatus};
use brain_stores::InMemoryStore;
use brain_types::*;
use futures::StreamExt;

fn make_server() -> Arc<BrainServer> {
    let provider: Arc<dyn Provider> = Arc::new(MockProvider::new());
    let store = Arc::new(InMemoryStore::new());
    let agent_loop: Arc<dyn AgentLoop> = Arc::new(SimpleLoop);
    let brain = Brain::new(provider, store, agent_loop, vec![]);
    Arc::new(BrainServer::new(brain))
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

/// Helper: create a project via HTTP and return it.
async fn create_project(client: &reqwest::Client, base: &str) -> Project {
    client
        .post(format!("{base}/projects"))
        .json(&serde_json::json!({"name": "http-test"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap()
}

/// Helper: create a session under a project via HTTP.
async fn create_session(client: &reqwest::Client, base: &str, project_id: ProjectId) -> Session {
    client
        .post(format!("{base}/projects/{project_id}/sessions"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap()
}

// -- Project tests --

#[tokio::test]
async fn create_project_returns_201() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/projects"))
        .json(&serde_json::json!({"name": "myproject"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let project: Project = resp.json().await.unwrap();
    assert_eq!(project.name.as_deref(), Some("myproject"));
}

#[tokio::test]
async fn list_projects_returns_200() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    create_project(&client, &base).await;
    create_project(&client, &base).await;

    let resp = client.get(format!("{base}/projects")).send().await.unwrap();
    assert_eq!(resp.status(), 200);

    let projects: Vec<Project> = resp.json().await.unwrap();
    assert_eq!(projects.len(), 2);
}

#[tokio::test]
async fn get_project_returns_200() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let project = create_project(&client, &base).await;

    let resp = client
        .get(format!("{base}/projects/{}", project.id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let got: Project = resp.json().await.unwrap();
    assert_eq!(got.id, project.id);
}

#[tokio::test]
async fn delete_project_returns_204() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let project = create_project(&client, &base).await;

    let resp = client
        .delete(format!("{base}/projects/{}", project.id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    let list: Vec<Project> = client
        .get(format!("{base}/projects"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(list.is_empty());
}

// -- Session tests --

#[tokio::test]
async fn create_session_returns_201() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let project = create_project(&client, &base).await;

    let resp = client
        .post(format!("{base}/projects/{}/sessions", project.id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let session: Session = resp.json().await.unwrap();
    assert_eq!(session.project_id, project.id);
}

#[tokio::test]
async fn list_sessions_returns_200() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let project = create_project(&client, &base).await;

    create_session(&client, &base, project.id).await;
    create_session(&client, &base, project.id).await;

    let resp = client
        .get(format!("{base}/projects/{}/sessions", project.id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let sessions: Vec<Session> = resp.json().await.unwrap();
    assert_eq!(sessions.len(), 2);
}

#[tokio::test]
async fn get_session_returns_200() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, project.id).await;

    let resp = client
        .get(format!("{base}/sessions/{}", session.id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let got: Session = resp.json().await.unwrap();
    assert_eq!(got.id, session.id);
}

#[tokio::test]
async fn delete_session_returns_204() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, project.id).await;

    let resp = client
        .delete(format!("{base}/sessions/{}", session.id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    let list: Vec<Session> = client
        .get(format!("{base}/projects/{}/sessions", project.id))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(list.is_empty());
}

// -- Turn & message tests --

#[tokio::test]
async fn send_message_returns_202() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, project.id).await;

    let resp = client
        .post(format!("{base}/sessions/{}/message", session.id))
        .json(&serde_json::json!({"content": "hello"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 202);
}

// -- Status --

#[tokio::test]
async fn status_returns_200() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let resp = client.get(format!("{base}/status")).send().await.unwrap();
    assert_eq!(resp.status(), 200);

    let status: ServerStatus = resp.json().await.unwrap();
    assert_eq!(status.active_sessions, 0);
    assert!(status.active_turns.is_empty());
}

// -- Error cases --

#[tokio::test]
async fn invalid_session_id_returns_400() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{base}/sessions/not-a-ulid"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn invalid_project_id_returns_400() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{base}/projects/not-a-ulid"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
}

// -- SSE --

#[tokio::test]
async fn sse_receives_events() {
    use futures::StreamExt;

    let base = start_server().await;
    let client = reqwest::Client::new();

    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, project.id).await;

    let sse_resp = client
        .get(format!("{base}/events"))
        .send()
        .await
        .unwrap();
    assert_eq!(sse_resp.status(), 200);

    client
        .post(format!("{base}/sessions/{}/message", session.id))
        .json(&serde_json::json!({"content": "hello"}))
        .send()
        .await
        .unwrap();

    let mut byte_stream = sse_resp.bytes_stream();
    let mut accumulated = String::new();
    let mut found_event = false;

    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(3);
    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout_at(deadline, byte_stream.next()).await {
            Ok(Some(Ok(chunk))) => {
                accumulated.push_str(&String::from_utf8_lossy(&chunk));
                if let Some(data_line) = accumulated.lines().find(|l| l.starts_with("data:")) {
                    let json = data_line.strip_prefix("data:").unwrap().trim();
                    if let Ok(event) = serde_json::from_str::<ServerEvent>(json) {
                        assert_eq!(event.session_id, session.id);
                        found_event = true;
                        break;
                    }
                }
            }
            Ok(Some(Err(e))) => panic!("SSE stream error: {e}"),
            Ok(None) => break,
            Err(_) => break,
        }
    }

    assert!(found_event, "should have received at least one SSE event, got: {accumulated}");
}

// -- Session update --

#[tokio::test]
async fn update_session_returns_204() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, project.id).await;
    assert!(session.title.is_none());

    let resp = client
        .patch(format!("{base}/sessions/{}", session.id))
        .json(&serde_json::json!({"title": "renamed"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    let got: Session = client
        .get(format!("{base}/sessions/{}", session.id))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(got.title.as_deref(), Some("renamed"));
}

// -- Message history --

#[tokio::test]
async fn list_messages_returns_200() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, project.id).await;

    let resp = client
        .get(format!("{base}/sessions/{}/messages", session.id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let messages: Vec<brain_types::Message> = resp.json().await.unwrap();
    assert!(messages.is_empty());
}

#[tokio::test]
async fn list_messages_after_turn() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, project.id).await;

    // Send a message and wait for the turn to complete via SSE
    let sse_resp = client.get(format!("{base}/events")).send().await.unwrap();

    client
        .post(format!("{base}/sessions/{}/message", session.id))
        .json(&serde_json::json!({"content": "hello"}))
        .send()
        .await
        .unwrap();

    // Wait for TurnDone in SSE stream
    let mut byte_stream = sse_resp.bytes_stream();
    let mut accumulated = String::new();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout_at(deadline, byte_stream.next()).await {
            Ok(Some(Ok(chunk))) => {
                accumulated.push_str(&String::from_utf8_lossy(&chunk));
                if accumulated.contains("turn_done") {
                    break;
                }
            }
            _ => break,
        }
    }

    let resp = client
        .get(format!("{base}/sessions/{}/messages", session.id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let messages: Vec<brain_types::Message> = resp.json().await.unwrap();
    assert!(!messages.is_empty(), "should have messages after a turn");
}

// -- Streaming message --

#[tokio::test]
async fn send_message_stream_returns_ndjson() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, project.id).await;

    let resp = client
        .post(format!("{base}/sessions/{}/message/stream", session.id))
        .json(&serde_json::json!({"content": "hello"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers().get("content-type").unwrap().to_str().unwrap(),
        "application/x-ndjson"
    );

    let mut byte_stream = resp.bytes_stream();
    let mut accumulated = String::new();
    let mut saw_token = false;
    let mut saw_done = false;

    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout_at(deadline, byte_stream.next()).await {
            Ok(Some(Ok(chunk))) => {
                accumulated.push_str(&String::from_utf8_lossy(&chunk));
                for line in accumulated.lines() {
                    if line.trim().is_empty() { continue; }
                    if let Ok(event) = serde_json::from_str::<brain_types::Event>(line) {
                        match event {
                            brain_types::Event::Token { .. } => saw_token = true,
                            brain_types::Event::TurnDone { .. } => saw_done = true,
                            _ => {}
                        }
                    }
                }
                if saw_done { break; }
            }
            _ => break,
        }
    }

    assert!(saw_token, "should have received token events in NDJSON stream");
    assert!(saw_done, "should have received TurnDone in NDJSON stream");
}

// -- Providers --

#[tokio::test]
async fn list_providers_returns_200() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let resp = client.get(format!("{base}/providers")).send().await.unwrap();
    assert_eq!(resp.status(), 200);

    let providers: Vec<brain_types::ProviderInfo> = resp.json().await.unwrap();
    assert_eq!(providers.len(), 1);
    assert_eq!(providers[0].name, "mock");
}

// -- Credentials --

#[tokio::test]
async fn credential_crud_via_http() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    // List empty
    let resp = client.get(format!("{base}/credentials")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let list: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(list.is_empty());

    // Get non-existent (returns empty array, not 404)
    let resp = client.get(format!("{base}/credentials/openai")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let entries: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(entries.is_empty());

    // Save (CredentialEntry)
    let entry = brain_types::CredentialEntry::api_key("key-1", "sk-test");
    let resp = client
        .put(format!("{base}/credentials/openai"))
        .json(&entry)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    // Get
    let resp = client.get(format!("{base}/credentials/openai")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let entries: Vec<brain_types::CredentialEntry> = resp.json().await.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, "key-1");
    match &entries[0].credential {
        brain_types::ProviderCredential::ApiKey { api_key } => assert_eq!(api_key, "sk-test"),
        _ => panic!("expected ApiKey"),
    }

    // List
    let resp = client.get(format!("{base}/credentials")).send().await.unwrap();
    let list: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["provider"], "openai");

    // Delete by provider + credential_id
    let resp = client.delete(format!("{base}/credentials/openai/key-1")).send().await.unwrap();
    assert_eq!(resp.status(), 204);

    let resp = client.get(format!("{base}/credentials/openai")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let entries: Vec<brain_types::CredentialEntry> = resp.json().await.unwrap();
    assert!(entries.is_empty());
}

// -- Status includes provider info --

#[tokio::test]
async fn status_includes_providers() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let resp = client.get(format!("{base}/status")).send().await.unwrap();
    assert_eq!(resp.status(), 200);

    let status: brain_server::ServerStatus = resp.json().await.unwrap();
    assert_eq!(status.providers.len(), 1);
    assert_eq!(status.providers[0].name, "mock");
}
