use futures::StreamExt;
use provider_openai::{
    Client, Config, Error, ResponseEvent, ResponseInputContentPart, ResponseInputItem,
    ResponseInputRole, ResponseRequest, ResponseStreamTransport,
};
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

async fn spawn_http_server(
    status_line: &str,
    content_type: &str,
    response_body: String,
) -> (String, oneshot::Receiver<CapturedRequest>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let status_line = status_line.to_owned();
    let content_type = content_type.to_owned();
    let (tx, rx) = oneshot::channel();

    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let request = read_http_request(&mut socket).await;
        let _ = tx.send(request);

        let response = format!(
            "HTTP/1.1 {status_line}\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            response_body.len(),
            response_body
        );
        socket.write_all(response.as_bytes()).await.unwrap();
        socket.shutdown().await.unwrap();
    });

    (format!("http://{addr}/v1"), rx)
}

#[tokio::test]
async fn create_response_sends_expected_headers_and_defaults() {
    let response_body = serde_json::json!({
        "id": "resp_test",
        "object": "response",
        "status": "completed",
        "output": [],
        "usage": {
            "input_tokens": 3,
            "output_tokens": 2,
            "total_tokens": 5
        }
    })
    .to_string();

    let (base_url, request_rx) =
        spawn_http_server("200 OK", "application/json", response_body).await;
    let client = Client::new(
        Config::new("sk-test")
            .with_base_url(base_url)
            .with_model("gpt-test")
            .with_default_header("x-provider", "provider-openai-test"),
    );

    let response = client
        .responses()
        .create(&ResponseRequest {
            input: vec![ResponseInputItem::message(
                ResponseInputRole::User,
                vec![ResponseInputContentPart::input_text("hello from test")],
            )],
            ..ResponseRequest::default()
        })
        .await
        .unwrap();

    let request = request_rx.await.unwrap();
    let request_head = request.head.to_lowercase();
    assert!(request_head.contains("post /v1/responses http/1.1"));
    assert!(request_head.contains("authorization: bearer sk-test"));
    assert!(request_head.contains("x-provider: provider-openai-test"));

    assert_eq!(
        request.body.get("model").and_then(Value::as_str),
        Some("gpt-test")
    );
    assert_eq!(
        request.body.get("store").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        request
            .body
            .pointer("/input/0/role")
            .and_then(Value::as_str),
        Some("user")
    );
    assert_eq!(
        request
            .body
            .pointer("/input/0/content/0/type")
            .and_then(Value::as_str),
        Some("input_text")
    );
    assert_eq!(
        request
            .body
            .pointer("/input/0/content/0/text")
            .and_then(Value::as_str),
        Some("hello from test")
    );

    assert_eq!(response.id.as_deref(), Some("resp_test"));
    assert_eq!(response.status.as_deref(), Some("completed"));
    assert_eq!(response.usage.as_ref().map(|usage| usage.total), Some(5));
}

#[tokio::test]
async fn create_response_surfaces_structured_api_errors() {
    let response_body = serde_json::json!({
        "error": {
            "message": "rate limit exceeded",
            "type": "rate_limit_error"
        }
    })
    .to_string();

    let (base_url, request_rx) =
        spawn_http_server("429 Too Many Requests", "application/json", response_body).await;
    let client = Client::new(
        Config::new("sk-test")
            .with_base_url(base_url)
            .with_model("gpt-test"),
    );

    let err = client
        .responses()
        .create(&ResponseRequest {
            input: vec![ResponseInputItem::message(
                ResponseInputRole::User,
                vec![ResponseInputContentPart::input_text("hello from test")],
            )],
            ..ResponseRequest::default()
        })
        .await
        .unwrap_err();

    let request = request_rx.await.unwrap();
    let request_head = request.head.to_lowercase();
    assert!(request_head.contains("post /v1/responses http/1.1"));

    match err {
        Error::Inference(message) => {
            assert_eq!(message, "429 Too Many Requests: rate limit exceeded");
        }
        other => panic!("expected inference error, got {other:?}"),
    }
}

#[tokio::test]
async fn create_response_surfaces_insufficient_quota_errors() {
    let response_body = serde_json::json!({
        "error": {
            "message": "You exceeded your current quota, please check your plan and billing details.",
            "type": "insufficient_quota"
        }
    })
    .to_string();

    let (base_url, request_rx) =
        spawn_http_server("429 Too Many Requests", "application/json", response_body).await;
    let client = Client::new(
        Config::new("sk-test")
            .with_base_url(base_url)
            .with_model("gpt-test"),
    );

    let err = client
        .responses()
        .create(&ResponseRequest {
            input: vec![ResponseInputItem::message(
                ResponseInputRole::User,
                vec![ResponseInputContentPart::input_text(
                    "hello from quota test",
                )],
            )],
            ..ResponseRequest::default()
        })
        .await
        .unwrap_err();

    let request = request_rx.await.unwrap();
    let request_head = request.head.to_lowercase();
    assert!(request_head.contains("post /v1/responses http/1.1"));

    match err {
        Error::Inference(message) => {
            assert_eq!(
                message,
                "429 Too Many Requests: You exceeded your current quota, please check your plan and billing details."
            );
        }
        other => panic!("expected inference error, got {other:?}"),
    }
}

#[tokio::test]
async fn create_response_surfaces_plain_text_throttle_errors() {
    let response_body = "request throttled, retry after 30 seconds".to_owned();

    let (base_url, request_rx) =
        spawn_http_server("429 Too Many Requests", "text/plain", response_body).await;
    let client = Client::new(
        Config::new("sk-test")
            .with_base_url(base_url)
            .with_model("gpt-test"),
    );

    let err = client
        .responses()
        .create(&ResponseRequest {
            input: vec![ResponseInputItem::message(
                ResponseInputRole::User,
                vec![ResponseInputContentPart::input_text(
                    "hello from throttle test",
                )],
            )],
            ..ResponseRequest::default()
        })
        .await
        .unwrap_err();

    let request = request_rx.await.unwrap();
    let request_head = request.head.to_lowercase();
    assert!(request_head.contains("post /v1/responses http/1.1"));

    match err {
        Error::Inference(message) => {
            assert_eq!(
                message,
                "429 Too Many Requests: request throttled, retry after 30 seconds"
            );
        }
        other => panic!("expected inference error, got {other:?}"),
    }
}

#[tokio::test]
async fn stream_response_over_sse_parses_events_and_terminal_state() {
    let response_body = concat!(
        "data: {\"type\":\"response.output_text.delta\",\"sequence_number\":1,\"item_id\":\"msg_1\",\"output_index\":0,\"content_index\":0,\"delta\":\"hel\"}\n\n",
        "data: {\"type\":\"response.completed\",\"sequence_number\":2,\"response\":{\"id\":\"resp_test\",\"status\":\"completed\",\"output\":[],\"usage\":{\"input_tokens\":1,\"output_tokens\":2,\"total_tokens\":3}}}\n\n",
        "data: [DONE]\n\n"
    )
    .to_owned();

    let (base_url, request_rx) =
        spawn_http_server("200 OK", "text/event-stream", response_body).await;
    let client = Client::new(
        Config::new("sk-test")
            .with_base_url(base_url)
            .with_model("gpt-test"),
    );

    let mut stream = client
        .responses()
        .stream(
            &ResponseRequest {
                input: vec![ResponseInputItem::message(
                    ResponseInputRole::User,
                    vec![ResponseInputContentPart::input_text("hello from test")],
                )],
                ..ResponseRequest::default()
            },
            ResponseStreamTransport::Sse,
        )
        .await
        .unwrap();

    let mut events = Vec::new();
    while let Some(event) = stream.next().await {
        events.push(event.unwrap());
    }

    let request = request_rx.await.unwrap();
    let request_head = request.head.to_lowercase();
    assert!(request_head.contains("post /v1/responses http/1.1"));
    assert!(request_head.contains("accept: text/event-stream"));
    assert_eq!(
        request.body.get("stream").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        request.body.get("model").and_then(Value::as_str),
        Some("gpt-test")
    );

    assert_eq!(events.len(), 2);
    assert_eq!(events[0].sequence_number, Some(1));
    match &events[0].event {
        ResponseEvent::OutputTextDelta { delta, .. } => assert_eq!(delta, "hel"),
        other => panic!("expected output text delta, got {other:?}"),
    }

    assert_eq!(events[1].sequence_number, Some(2));
    match &events[1].event {
        ResponseEvent::ResponseCompleted { response } => {
            assert_eq!(response.id.as_deref(), Some("resp_test"));
            assert_eq!(response.status.as_deref(), Some("completed"));
            assert_eq!(response.usage.as_ref().map(|usage| usage.total), Some(3));
        }
        other => panic!("expected response completed event, got {other:?}"),
    }
}
