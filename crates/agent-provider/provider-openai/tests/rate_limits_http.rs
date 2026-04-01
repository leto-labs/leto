use provider_openai::{Client, Config, ProjectRateLimitPage};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

struct CapturedRequest {
    head: String,
}

async fn read_http_request(socket: &mut tokio::net::TcpStream) -> CapturedRequest {
    let mut buf = vec![0u8; 16 * 1024];
    let mut request = Vec::new();

    loop {
        let n = socket.read(&mut buf).await.unwrap();
        assert!(n > 0, "connection closed before request headers were read");
        request.extend_from_slice(&buf[..n]);
        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }

    CapturedRequest {
        head: String::from_utf8_lossy(&request).into_owned(),
    }
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
async fn list_project_rate_limits_fetches_rate_limit_page() {
    let response_body = serde_json::json!({
        "object": "list",
        "data": [
            {
                "id": "rl_123",
                "object": "rate_limit",
                "model": "gpt-5.4",
                "max_requests_per_1_minute": 500,
                "max_tokens_per_1_minute": 40000,
                "max_images_per_1_minute": 25,
                "batch_1_day_max_input_tokens": 1000000,
                "scope": "project"
            }
        ]
    })
    .to_string();

    let (base_url, request_rx) = spawn_json_server(response_body).await;
    let client = Client::new(
        Config::new("sk-admin-test")
            .with_base_url(base_url)
            .with_default_header("x-provider", "provider-openai-test"),
    );

    let page: ProjectRateLimitPage = client.rate_limits().list_project("proj_123").await.unwrap();

    let request = request_rx.await.unwrap();
    let request_head = request.head.to_lowercase();
    assert!(request_head.contains("get /v1/organization/projects/proj_123/rate_limits http/1.1"));
    assert!(request_head.contains("authorization: bearer sk-admin-test"));
    assert!(request_head.contains("x-provider: provider-openai-test"));

    assert_eq!(page.object, "list");
    assert_eq!(page.data.len(), 1);
    assert_eq!(page.data[0].id, "rl_123");
    assert_eq!(page.data[0].object.as_deref(), Some("rate_limit"));
    assert_eq!(page.data[0].model.as_deref(), Some("gpt-5.4"));
    assert_eq!(page.data[0].max_requests_per_1_minute, Some(500));
    assert_eq!(page.data[0].max_tokens_per_1_minute, Some(40_000));
    assert_eq!(page.data[0].max_images_per_1_minute, Some(25));
    assert_eq!(page.data[0].batch_1_day_max_input_tokens, Some(1_000_000));
    assert_eq!(
        page.data[0]
            .extra
            .get("scope")
            .and_then(serde_json::Value::as_str),
        Some("project")
    );
}
