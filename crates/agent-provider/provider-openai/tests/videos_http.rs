use provider_openai::{Client, Config, VideoCreateRequest};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

struct CapturedRequest {
    head: String,
    body: String,
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

    let body = String::from_utf8_lossy(&request[header_end..header_end + content_length]).into();

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
async fn create_video_posts_request_and_parses_response() {
    let response_body = serde_json::json!({
        "id": "video_123",
        "object": "video",
        "model": "sora-2",
        "status": "queued",
        "progress": 0,
        "created_at": 1_731_000_000_i64,
        "size": "720x1280",
        "seconds": "8",
        "quality": "standard"
    })
    .to_string();

    let (base_url, request_rx) = spawn_json_server(response_body).await;
    let client = Client::new(
        Config::new("sk-test")
            .with_base_url(base_url)
            .with_model("sora-2")
            .with_default_header("x-provider", "provider-openai-test"),
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

    let request = request_rx.await.unwrap();
    let request_head = request.head.to_lowercase();
    assert!(request_head.contains("post /v1/videos http/1.1"));
    assert!(request_head.contains("authorization: bearer sk-test"));
    assert!(request_head.contains("x-provider: provider-openai-test"));
    assert!(request_head.contains("content-type: multipart/form-data;"));

    assert!(request.body.contains("name=\"model\""));
    assert!(request.body.contains("sora-2"));
    assert!(request.body.contains("name=\"prompt\""));
    assert!(request.body.contains("a calico cat playing piano"));
    assert!(request.body.contains("name=\"seconds\""));
    assert!(request.body.contains("8"));
    assert!(request.body.contains("name=\"size\""));
    assert!(request.body.contains("720x1280"));

    assert_eq!(response.id, "video_123");
    assert_eq!(response.object, "video");
    assert_eq!(response.model.as_deref(), Some("sora-2"));
    assert_eq!(response.status.as_deref(), Some("queued"));
    assert_eq!(response.progress, Some(0));
    assert_eq!(response.seconds.as_deref(), Some("8"));
    assert_eq!(response.size.as_deref(), Some("720x1280"));
}
