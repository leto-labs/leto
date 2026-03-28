use provider_openai::{Client, Config};
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
async fn list_fine_tuning_jobs_fetches_job_page() {
    let response_body = serde_json::json!({
        "object": "list",
        "has_more": false,
        "data": [
            {
                "id": "ftjob_123",
                "object": "fine_tuning.job",
                "model": "gpt-4o-mini",
                "fine_tuned_model": "ft:gpt-4o-mini:custom",
                "status": "succeeded",
                "created_at": 1_731_000_000
            }
        ]
    })
    .to_string();

    let (base_url, request_rx) = spawn_json_server(response_body).await;
    let client = Client::new(
        Config::new("sk-test")
            .with_base_url(base_url)
            .with_default_header("x-provider", "provider-openai-test"),
    );

    let page = client.fine_tuning().list_jobs().await.unwrap();

    let request = request_rx.await.unwrap();
    let request_head = request.head.to_lowercase();
    assert!(request_head.contains("get /v1/fine_tuning/jobs http/1.1"));
    assert!(request_head.contains("authorization: bearer sk-test"));
    assert!(request_head.contains("x-provider: provider-openai-test"));

    assert_eq!(page.object, "list");
    assert!(!page.has_more);
    assert_eq!(page.data.len(), 1);
    assert_eq!(page.data[0].id, "ftjob_123");
    assert_eq!(page.data[0].object, "fine_tuning.job");
    assert_eq!(page.data[0].model.as_deref(), Some("gpt-4o-mini"));
    assert_eq!(
        page.data[0].fine_tuned_model.as_deref(),
        Some("ft:gpt-4o-mini:custom")
    );
    assert_eq!(page.data[0].status.as_deref(), Some("succeeded"));
}
