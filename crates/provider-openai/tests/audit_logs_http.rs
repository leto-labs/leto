use provider_openai::{AuditActor, AuditLogListParams, AuditLogPage, Client, Config};
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
async fn list_audit_logs_fetches_event_page() {
    let response_body = serde_json::json!({
        "object": "list",
        "has_more": false,
        "data": [
            {
                "id": "req_xxx_20240101",
                "type": "api_key.created",
                "effective_at": 1_720_804_090_i64,
                "actor": {
                    "type": "session",
                    "session": {
                        "user": {
                            "id": "user-123",
                            "email": "user@example.com"
                        },
                        "ip_address": "127.0.0.1",
                        "user_agent": "Mozilla/5.0"
                    }
                },
                "project": {
                    "id": "proj_123",
                    "name": "Default Project"
                },
                "api_key.created": {
                    "id": "key_123"
                }
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

    let page: AuditLogPage = client.audit_logs().list().await.unwrap();

    let request = request_rx.await.unwrap();
    let request_head = request.head.to_lowercase();
    assert!(request_head.contains("get /v1/organization/audit_logs http/1.1"));
    assert!(request_head.contains("authorization: bearer sk-admin-test"));
    assert!(request_head.contains("x-provider: provider-openai-test"));

    assert_eq!(page.object, "list");
    assert!(!page.has_more);
    assert_eq!(page.data.len(), 1);
    assert_eq!(page.data[0].id, "req_xxx_20240101");
    assert_eq!(page.data[0].event_type, "api_key.created");
    assert_eq!(page.data[0].effective_at, 1_720_804_090_i64);
    assert_eq!(
        page.data[0]
            .project
            .as_ref()
            .map(|project| project.id.as_str()),
        Some("proj_123")
    );
    assert!(matches!(
        &page.data[0].actor,
        AuditActor {
            actor_type,
            session: Some(_),
        } if actor_type == "session"
    ));
    assert!(page.data[0].extra.contains_key("api_key.created"));
}

#[tokio::test]
async fn list_audit_logs_sends_pagination_query_parameters() {
    let response_body = serde_json::json!({
        "object": "list",
        "has_more": true,
        "data": []
    })
    .to_string();

    let (base_url, request_rx) = spawn_json_server(response_body).await;
    let client = Client::new(
        Config::new("sk-admin-test")
            .with_base_url(base_url)
            .with_default_header("x-provider", "provider-openai-test"),
    );

    let page: AuditLogPage = client
        .audit_logs()
        .list_with_params(&AuditLogListParams {
            after: Some("req_after_123".into()),
            before: Some("req_before_456".into()),
            limit: Some(25),
        })
        .await
        .unwrap();

    let request = request_rx.await.unwrap();
    let request_head = request.head.to_lowercase();
    assert!(request_head.contains("get /v1/organization/audit_logs?"));
    assert!(request_head.contains("after=req_after_123"));
    assert!(request_head.contains("before=req_before_456"));
    assert!(request_head.contains("limit=25"));
    assert!(request_head.contains("authorization: bearer sk-admin-test"));
    assert!(request_head.contains("x-provider: provider-openai-test"));

    assert_eq!(page.object, "list");
    assert!(page.has_more);
    assert!(page.data.is_empty());
}
