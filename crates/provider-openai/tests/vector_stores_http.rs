use std::collections::BTreeMap;

use provider_openai::{Client, Config, VectorStoreCreateRequest};
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

struct CapturedRequest {
    head: String,
    body: Value,
}

async fn read_http_request(socket: &mut tokio::net::TcpStream) -> CapturedRequest {
    let mut buf = vec![0u8; 16 * 1024];
    let mut request = Vec::new();
    let header_end;

    loop {
        let n = socket.read(&mut buf).await.unwrap();
        assert!(n > 0, "connection closed before request headers were read");
        request.extend_from_slice(&buf[..n]);

        if let Some(pos) = request.windows(4).position(|window| window == b"\r\n\r\n") {
            header_end = pos + 4;
            break;
        }
    }

    let head = String::from_utf8_lossy(&request[..header_end]).into_owned();
    let content_length = head
        .lines()
        .find_map(|line| {
            line.split_once(':').and_then(|(name, value)| {
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().unwrap())
            })
        })
        .unwrap_or_default();

    while request.len() < header_end + content_length {
        let n = socket.read(&mut buf).await.unwrap();
        assert!(n > 0, "connection closed before request body was read");
        request.extend_from_slice(&buf[..n]);
    }

    let body = serde_json::from_slice(&request[header_end..header_end + content_length]).unwrap();

    CapturedRequest { head, body }
}

async fn spawn_json_server(response_body: String) -> (String, oneshot::Receiver<CapturedRequest>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = oneshot::channel();

    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let request = read_http_request(&mut socket).await;
        let _ = tx.send(request);

        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            response_body.len(),
            response_body
        );
        socket.write_all(response.as_bytes()).await.unwrap();
        socket.shutdown().await.unwrap();
    });

    (format!("http://{addr}/v1"), rx)
}

#[tokio::test]
async fn create_vector_store_posts_request_and_parses_response() {
    let response_body = serde_json::json!({
        "id": "vs_123",
        "object": "vector_store",
        "created_at": 1_731_000_000_i64,
        "name": "Support FAQ",
        "description": "Contains support answers",
        "bytes": 2048,
        "file_counts": {
            "in_progress": 0,
            "completed": 3,
            "failed": 0,
            "cancelled": 0,
            "total": 3
        }
    })
    .to_string();

    let (base_url, request_rx) = spawn_json_server(response_body).await;
    let client = Client::new(
        Config::new("sk-test")
            .with_base_url(base_url)
            .with_default_header("x-provider", "provider-openai-test"),
    );

    let response = client
        .vector_stores()
        .create(&VectorStoreCreateRequest {
            name: Some("Support FAQ".into()),
            description: Some("Contains support answers".into()),
            metadata: BTreeMap::from([("team".into(), "support".into())]),
        })
        .await
        .unwrap();

    let request = request_rx.await.unwrap();
    let request_head = request.head.to_lowercase();
    assert!(request_head.contains("post /v1/vector_stores http/1.1"));
    assert!(request_head.contains("authorization: bearer sk-test"));
    assert!(request_head.contains("x-provider: provider-openai-test"));
    assert!(request_head.contains("openai-beta: assistants=v2"));

    assert_eq!(
        request.body.get("name").and_then(Value::as_str),
        Some("Support FAQ")
    );
    assert_eq!(
        request.body.get("description").and_then(Value::as_str),
        Some("Contains support answers")
    );
    assert_eq!(
        request
            .body
            .pointer("/metadata/team")
            .and_then(Value::as_str),
        Some("support")
    );

    assert_eq!(response.id, "vs_123");
    assert_eq!(response.object, "vector_store");
    assert_eq!(response.name.as_deref(), Some("Support FAQ"));
    assert_eq!(response.bytes, Some(2048));
    assert_eq!(
        response.file_counts.as_ref().map(|counts| counts.completed),
        Some(3)
    );
}
