use std::sync::Arc;

use futures::StreamExt;
use provider::{CredentialEntry, CredentialFailure, CredentialPool, Request, StickyRoundRobin};
use provider_openai::{Config, OpenAiOAuthPreset, OpenAiOAuthProvider, OpenAiProvider};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

fn completed_sse_body() -> String {
    concat!(
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_test\",\"status\":\"completed\",\"output\":[],\"usage\":{\"input_tokens\":1,\"output_tokens\":1,\"total_tokens\":2}}}\n\n",
        "data: [DONE]\n\n"
    )
    .to_owned()
}

fn created_only_sse_body() -> String {
    "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_test\",\"status\":\"in_progress\",\"output\":[]}}\n\n".to_owned()
}

async fn spawn_mock_sse_server(body: String) -> (String, oneshot::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = oneshot::channel();

    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0u8; 16 * 1024];
        let mut request = Vec::new();

        loop {
            let n = socket.read(&mut buf).await.unwrap();
            if n == 0 {
                break;
            }
            request.extend_from_slice(&buf[..n]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }

        let _ = tx.send(String::from_utf8_lossy(&request).into_owned());

        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        socket.write_all(response.as_bytes()).await.unwrap();
        socket.shutdown().await.unwrap();
    });

    (format!("http://{addr}/v1"), rx)
}

async fn spawn_two_request_sse_server(body: String) -> (String, oneshot::Receiver<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = oneshot::channel();

    tokio::spawn(async move {
        let mut requests = Vec::with_capacity(2);
        for _ in 0..2 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 16 * 1024];
            let mut request = Vec::new();

            loop {
                let n = socket.read(&mut buf).await.unwrap();
                if n == 0 {
                    break;
                }
                request.extend_from_slice(&buf[..n]);
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

    (format!("http://{addr}/v1"), rx)
}

fn oauth_preset_for_base_url(base_url: String) -> OpenAiOAuthPreset {
    OpenAiOAuthPreset {
        name: "openai-oauth",
        authorize_url: "https://auth.openai.com/oauth/authorize",
        token_url: "https://auth.openai.com/oauth/token",
        device_code_url: "https://auth.openai.com/api/accounts/deviceauth/usercode",
        device_token_url: "https://auth.openai.com/api/accounts/deviceauth/token",
        device_verification_url: "https://auth.openai.com/codex/device",
        device_redirect_uri: "https://auth.openai.com/deviceauth/callback",
        client_id: "test-client-id",
        scopes: "openid profile email offline_access",
        api_base_url: Box::leak(base_url.into_boxed_str()),
        default_model: "gpt-5.3-codex",
        callback_port: 1455,
        models: &[],
    }
}

async fn drain_to_terminal(
    provider: &dyn provider::Provider,
    request: &Request,
) -> Result<(), provider::Error> {
    let mut stream = provider.stream(request).await?;
    let mut saw_completed = false;
    while let Some(event) = stream.next().await {
        match event? {
            provider::Event::Completed { .. } => saw_completed = true,
            provider::Event::ResponseStart { .. }
            | provider::Event::BlockStart { .. }
            | provider::Event::BlockDelta { .. }
            | provider::Event::BlockStop { .. }
            | provider::Event::Usage { .. } => {}
        }
    }
    if saw_completed {
        Ok(())
    } else {
        Err(provider::Error::Inference(
            "stream ended without completed event".into(),
        ))
    }
}

#[tokio::test]
async fn openai_provider_uses_shared_pool_headers_and_marks_success() {
    let pool = Arc::new(CredentialPool::new(Arc::new(StickyRoundRobin::new())));
    let mut entry = CredentialEntry::bearer("cred-api", "sk-pool");
    entry
        .health
        .record_error(CredentialFailure::new("previous failure"));
    pool.insert("openai", entry).await;

    let (base_url, request_rx) = spawn_mock_sse_server(completed_sse_body()).await;
    let provider = OpenAiProvider::from_pool(
        Config::new("placeholder").with_base_url(base_url),
        pool.clone(),
    );

    drain_to_terminal(&provider, &Request::user_text("hello"))
        .await
        .unwrap();

    let request = request_rx.await.unwrap().to_lowercase();
    assert!(request.contains("post /v1/responses"));
    assert!(request.contains("authorization: bearer sk-pool"));

    let entries = pool.entries("openai").await;
    assert_eq!(entries[0].health.consecutive_errors, 0);
    assert!(entries[0].health.last_error.is_none());
}

#[tokio::test]
async fn openai_oauth_provider_uses_shared_pool_metadata() {
    let pool = Arc::new(CredentialPool::new(Arc::new(StickyRoundRobin::new())));
    pool.insert(
        "openai-oauth",
        CredentialEntry::openai_oauth("cred-oauth", "oauth-token", Some("acct-123".into())),
    )
    .await;

    let (base_url, request_rx) = spawn_mock_sse_server(completed_sse_body()).await;
    let provider = OpenAiOAuthProvider::from_pool(oauth_preset_for_base_url(base_url), pool);

    drain_to_terminal(&provider, &Request::user_text("hello"))
        .await
        .unwrap();

    let request = request_rx.await.unwrap().to_lowercase();
    assert!(request.contains("post /v1/responses"));
    assert!(request.contains("authorization: bearer oauth-token"));
    assert!(request.contains("chatgpt-account-id: acct-123"));
    assert!(request.contains("openai-beta: responses=experimental"));
}

#[tokio::test]
async fn openai_provider_rotates_after_pool_mark_error() {
    let pool = Arc::new(CredentialPool::new(Arc::new(StickyRoundRobin::new())));
    pool.insert("openai", CredentialEntry::bearer("cred-1", "sk-one"))
        .await;
    pool.insert("openai", CredentialEntry::bearer("cred-2", "sk-two"))
        .await;

    let (first_base_url, first_request_rx) = spawn_mock_sse_server(created_only_sse_body()).await;
    let mut request = Request::user_text("hello");
    request.options.metadata.insert(
        "session_id".into(),
        serde_json::Value::String("session-1".into()),
    );

    let first_provider = OpenAiProvider::from_pool(
        Config::new("placeholder").with_base_url(first_base_url),
        pool.clone(),
    );
    let err = drain_to_terminal(&first_provider, &request)
        .await
        .unwrap_err();
    assert!(
        err.to_string()
            .contains("stream closed before terminal response event")
    );

    let first_request = first_request_rx.await.unwrap().to_lowercase();
    assert!(first_request.contains("authorization: bearer sk-one"));

    let (second_base_url, second_request_rx) = spawn_mock_sse_server(completed_sse_body()).await;
    let second_provider = OpenAiProvider::from_pool(
        Config::new("placeholder").with_base_url(second_base_url),
        pool.clone(),
    );
    drain_to_terminal(&second_provider, &request).await.unwrap();

    let second_request = second_request_rx.await.unwrap().to_lowercase();
    assert!(second_request.contains("authorization: bearer sk-two"));

    let entries = pool.entries("openai").await;
    let first = entries.iter().find(|entry| entry.id == "cred-1").unwrap();
    assert!(first.health.consecutive_errors > 0);
}

#[tokio::test]
async fn openai_provider_skips_tripped_credential_for_new_sessions() {
    let pool = Arc::new(CredentialPool::new(Arc::new(StickyRoundRobin::new())));
    pool.insert("openai", CredentialEntry::bearer("cred-1", "sk-one"))
        .await;
    pool.insert("openai", CredentialEntry::bearer("cred-2", "sk-two"))
        .await;

    let (first_base_url, first_request_rx) = spawn_mock_sse_server(created_only_sse_body()).await;
    let mut first_request = Request::user_text("hello");
    first_request.options.metadata.insert(
        "session_id".into(),
        serde_json::Value::String("session-a".into()),
    );

    let first_provider = OpenAiProvider::from_pool(
        Config::new("placeholder").with_base_url(first_base_url),
        pool.clone(),
    );
    let err = drain_to_terminal(&first_provider, &first_request)
        .await
        .unwrap_err();
    assert!(
        err.to_string()
            .contains("stream closed before terminal response event")
    );

    let first_wire_request = first_request_rx.await.unwrap().to_lowercase();
    assert!(first_wire_request.contains("authorization: bearer sk-one"));

    let (second_base_url, second_request_rx) = spawn_mock_sse_server(completed_sse_body()).await;
    let mut second_request = Request::user_text("hello again");
    second_request.options.metadata.insert(
        "session_id".into(),
        serde_json::Value::String("session-b".into()),
    );

    let second_provider = OpenAiProvider::from_pool(
        Config::new("placeholder").with_base_url(second_base_url),
        pool.clone(),
    );
    drain_to_terminal(&second_provider, &second_request)
        .await
        .unwrap();

    let second_wire_request = second_request_rx.await.unwrap().to_lowercase();
    assert!(second_wire_request.contains("authorization: bearer sk-two"));

    let entries = pool.entries("openai").await;
    let first = entries.iter().find(|entry| entry.id == "cred-1").unwrap();
    let second = entries.iter().find(|entry| entry.id == "cred-2").unwrap();
    assert!(!first.health.is_healthy());
    assert!(second.health.is_healthy());
}

#[tokio::test]
async fn openai_provider_picks_up_reloaded_pool_credentials() {
    let pool = Arc::new(CredentialPool::new(Arc::new(StickyRoundRobin::new())));
    pool.insert("openai", CredentialEntry::bearer("cred-old", "sk-old"))
        .await;

    let (base_url, requests_rx) = spawn_two_request_sse_server(completed_sse_body()).await;
    let provider = OpenAiProvider::from_pool(
        Config::new("placeholder").with_base_url(base_url),
        pool.clone(),
    );

    let mut request = Request::user_text("hello");
    request.options.metadata.insert(
        "session_id".into(),
        serde_json::Value::String("session-hot-reload".into()),
    );

    drain_to_terminal(&provider, &request).await.unwrap();

    pool.set_entries(
        "openai",
        vec![CredentialEntry::bearer("cred-new", "sk-new")],
    )
    .await;

    drain_to_terminal(&provider, &request).await.unwrap();

    let requests = requests_rx.await.unwrap();
    assert_eq!(requests.len(), 2);
    assert!(
        requests[0]
            .to_lowercase()
            .contains("authorization: bearer sk-old")
    );
    assert!(
        requests[1]
            .to_lowercase()
            .contains("authorization: bearer sk-new")
    );

    let entries = pool.entries("openai").await;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, "cred-new");
    assert_eq!(
        entries[0]
            .material
            .headers
            .get("Authorization")
            .map(String::as_str),
        Some("Bearer sk-new")
    );
}
