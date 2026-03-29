use futures::StreamExt;
use provider_openai::{
    Client, Config, Error, ResponseEvent, ResponseInputContentPart, ResponseInputItem,
    ResponseInputRole, ResponseRequest, ResponseStreamTransport,
};
use serde_json::Value;
use std::collections::BTreeMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::time::Duration;

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

async fn spawn_concurrent_http_server() -> (String, oneshot::Receiver<Vec<CapturedRequest>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = oneshot::channel();

    tokio::spawn(async move {
        let mut tasks = Vec::new();
        for _ in 0..2 {
            let (mut socket, _) = listener.accept().await.unwrap();
            tasks.push(tokio::spawn(async move {
                let request = read_http_request(&mut socket).await;
                let request_text = request
                    .body
                    .pointer("/input/0/content/0/text")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown");
                let response_body = serde_json::json!({
                    "id": format!("resp_{request_text}"),
                    "object": "response",
                    "status": "completed",
                    "output": [],
                    "usage": {
                        "input_tokens": 1,
                        "output_tokens": 1,
                        "total_tokens": 2
                    }
                })
                .to_string();
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                    response_body.len(),
                    response_body
                );
                socket.write_all(response.as_bytes()).await.unwrap();
                socket.shutdown().await.unwrap();
                request
            }));
        }

        let mut requests = Vec::with_capacity(2);
        for task in tasks {
            requests.push(task.await.unwrap());
        }
        let _ = tx.send(requests);
    });

    (format!("http://{addr}/v1"), rx)
}

async fn spawn_connection_reuse_http_server() -> (String, oneshot::Receiver<Vec<CapturedRequest>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = oneshot::channel();

    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        drop(listener);

        let first_request = read_http_request(&mut socket).await;
        let first_response_body = serde_json::json!({
            "id": "resp_first",
            "object": "response",
            "status": "completed",
            "output": [],
            "usage": {
                "input_tokens": 1,
                "output_tokens": 1,
                "total_tokens": 2
            }
        })
        .to_string();
        let first_response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: keep-alive\r\n\r\n{}",
            first_response_body.len(),
            first_response_body
        );
        let _ = socket.write_all(first_response.as_bytes()).await;

        let second_request = read_http_request(&mut socket).await;
        let second_response_body = serde_json::json!({
            "id": "resp_second",
            "object": "response",
            "status": "completed",
            "output": [],
            "usage": {
                "input_tokens": 1,
                "output_tokens": 1,
                "total_tokens": 2
            }
        })
        .to_string();
        let second_response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            second_response_body.len(),
            second_response_body
        );
        let _ = socket.write_all(second_response.as_bytes()).await;
        let _ = socket.shutdown().await;

        let _ = tx.send(vec![first_request, second_request]);
    });

    (format!("http://{addr}/v1"), rx)
}

async fn spawn_resilient_http_server() -> (String, oneshot::Receiver<Vec<CapturedRequest>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = oneshot::channel();

    tokio::spawn(async move {
        let mut requests = Vec::with_capacity(2);

        for (status_line, response_body) in [
            (
                "500 Internal Server Error",
                serde_json::json!({
                    "error": {
                        "message": "transient upstream failure",
                        "type": "server_error"
                    }
                })
                .to_string(),
            ),
            (
                "200 OK",
                serde_json::json!({
                    "id": "resp_recovered",
                    "object": "response",
                    "status": "completed",
                    "output": [],
                    "usage": {
                        "input_tokens": 1,
                        "output_tokens": 1,
                        "total_tokens": 2
                    }
                })
                .to_string(),
            ),
        ] {
            let (mut socket, _) = listener.accept().await.unwrap();
            let request = read_http_request(&mut socket).await;
            requests.push(request);

            let response = format!(
                "HTTP/1.1 {status_line}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                response_body.len(),
                response_body
            );
            socket.write_all(response.as_bytes()).await.unwrap();
            socket.shutdown().await.unwrap();
        }

        let _ = tx.send(requests);
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
async fn create_response_preserves_trace_metadata_on_the_wire() {
    let response_body = serde_json::json!({
        "id": "resp_trace",
        "object": "response",
        "status": "completed",
        "output": [],
        "usage": {
            "input_tokens": 2,
            "output_tokens": 1,
            "total_tokens": 3
        }
    })
    .to_string();

    let (base_url, request_rx) =
        spawn_http_server("200 OK", "application/json", response_body).await;
    let client = Client::new(
        Config::new("sk-test")
            .with_base_url(base_url)
            .with_model("gpt-trace"),
    );

    let response = client
        .responses()
        .create(&ResponseRequest {
            input: vec![ResponseInputItem::message(
                ResponseInputRole::User,
                vec![ResponseInputContentPart::input_text("trace this request")],
            )],
            metadata: BTreeMap::from([
                (
                    "trace_id".into(),
                    serde_json::Value::String("trace-req-123".into()),
                ),
                (
                    "span".into(),
                    serde_json::json!({
                        "id": "span-9",
                        "parent": "root-1"
                    }),
                ),
            ]),
            ..ResponseRequest::default()
        })
        .await
        .unwrap();

    let request = request_rx.await.unwrap();
    assert_eq!(
        request
            .body
            .pointer("/metadata/trace_id")
            .and_then(Value::as_str),
        Some("trace-req-123")
    );
    assert_eq!(
        request
            .body
            .pointer("/metadata/span/id")
            .and_then(Value::as_str),
        Some("span-9")
    );
    assert_eq!(
        request
            .body
            .pointer("/metadata/span/parent")
            .and_then(Value::as_str),
        Some("root-1")
    );
    assert_eq!(response.id.as_deref(), Some("resp_trace"));
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
async fn stream_response_treats_terminal_event_then_socket_close_as_graceful_shutdown() {
    let response_body = concat!(
        "data: {\"type\":\"response.completed\",\"sequence_number\":1,\"response\":{\"id\":\"resp_shutdown\",\"status\":\"completed\",\"output\":[],\"usage\":{\"input_tokens\":2,\"output_tokens\":1,\"total_tokens\":3}}}\n\n"
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
                    vec![ResponseInputContentPart::input_text(
                        "hello from shutdown test",
                    )],
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
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].sequence_number, Some(1));
    match &events[0].event {
        ResponseEvent::ResponseCompleted { response } => {
            assert_eq!(response.id.as_deref(), Some("resp_shutdown"));
            assert_eq!(response.status.as_deref(), Some("completed"));
            assert_eq!(response.usage.as_ref().map(|usage| usage.total), Some(3));
        }
        other => panic!("expected response completed event, got {other:?}"),
    }
}

#[tokio::test]
async fn create_response_supports_concurrent_requests() {
    let (base_url, requests_rx) = spawn_concurrent_http_server().await;
    let client = Client::new(
        Config::new("sk-test")
            .with_base_url(base_url)
            .with_model("gpt-test"),
    );
    let client_clone = client.clone();

    let first = async {
        client
            .responses()
            .create(&ResponseRequest {
                input: vec![ResponseInputItem::message(
                    ResponseInputRole::User,
                    vec![ResponseInputContentPart::input_text("first")],
                )],
                ..ResponseRequest::default()
            })
            .await
            .unwrap()
    };
    let second = async {
        client_clone
            .responses()
            .create(&ResponseRequest {
                input: vec![ResponseInputItem::message(
                    ResponseInputRole::User,
                    vec![ResponseInputContentPart::input_text("second")],
                )],
                ..ResponseRequest::default()
            })
            .await
            .unwrap()
    };

    let (first_response, second_response) = tokio::join!(first, second);

    let mut request_texts = requests_rx
        .await
        .unwrap()
        .into_iter()
        .map(|request| {
            request
                .body
                .pointer("/input/0/content/0/text")
                .and_then(Value::as_str)
                .unwrap()
                .to_owned()
        })
        .collect::<Vec<_>>();
    request_texts.sort();

    let mut response_ids = vec![
        first_response.id.as_deref().unwrap().to_owned(),
        second_response.id.as_deref().unwrap().to_owned(),
    ];
    response_ids.sort();

    assert_eq!(request_texts, vec!["first", "second"]);
    assert_eq!(response_ids, vec!["resp_first", "resp_second"]);
}

#[tokio::test]
async fn create_response_reuses_http_connection() {
    let (base_url, requests_rx) = spawn_connection_reuse_http_server().await;
    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_millis(250))
        .pool_max_idle_per_host(1)
        .build()
        .unwrap();
    let client = Client::with_http_client(
        Config::new("sk-test")
            .with_base_url(base_url)
            .with_model("gpt-test"),
        http_client,
    );

    let first_response = client
        .responses()
        .create(&ResponseRequest {
            input: vec![ResponseInputItem::message(
                ResponseInputRole::User,
                vec![ResponseInputContentPart::input_text("first")],
            )],
            ..ResponseRequest::default()
        })
        .await
        .unwrap();
    let second_response = client
        .responses()
        .create(&ResponseRequest {
            input: vec![ResponseInputItem::message(
                ResponseInputRole::User,
                vec![ResponseInputContentPart::input_text("second")],
            )],
            ..ResponseRequest::default()
        })
        .await
        .unwrap();

    let requests = requests_rx.await.unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(first_response.id.as_deref(), Some("resp_first"));
    assert_eq!(second_response.id.as_deref(), Some("resp_second"));

    let request_heads = requests
        .iter()
        .map(|request| request.head.to_lowercase())
        .collect::<Vec<_>>();
    assert!(request_heads[0].contains("post /v1/responses http/1.1"));
    assert!(request_heads[1].contains("post /v1/responses http/1.1"));

    let request_texts = requests
        .iter()
        .map(|request| {
            request
                .body
                .pointer("/input/0/content/0/text")
                .and_then(Value::as_str)
                .unwrap()
        })
        .collect::<Vec<_>>();
    assert_eq!(request_texts, vec!["first", "second"]);
}

#[tokio::test]
async fn create_response_preserves_large_request_bodies() {
    let response_body = serde_json::json!({
        "id": "resp_large",
        "object": "response",
        "status": "completed",
        "output": [],
        "usage": {
            "input_tokens": 16385,
            "output_tokens": 1,
            "total_tokens": 16386
        }
    })
    .to_string();

    let (base_url, request_rx) =
        spawn_http_server("200 OK", "application/json", response_body).await;
    let client = Client::new(
        Config::new("sk-test")
            .with_base_url(base_url)
            .with_model("gpt-test"),
    );
    let oversized_text = "request-bloat-".repeat(2_048);

    let response = client
        .responses()
        .create(&ResponseRequest {
            input: vec![ResponseInputItem::message(
                ResponseInputRole::User,
                vec![ResponseInputContentPart::input_text(&oversized_text)],
            )],
            ..ResponseRequest::default()
        })
        .await
        .unwrap();

    let request = request_rx.await.unwrap();
    let request_text = request
        .body
        .pointer("/input/0/content/0/text")
        .and_then(Value::as_str)
        .unwrap();

    assert!(
        request
            .head
            .to_lowercase()
            .contains("post /v1/responses http/1.1")
    );
    assert!(oversized_text.len() > 16 * 1024);
    assert_eq!(request_text.len(), oversized_text.len());
    assert_eq!(request_text, oversized_text);
    assert_eq!(response.id.as_deref(), Some("resp_large"));
}

#[tokio::test]
async fn create_response_recovers_after_transient_http_failure() {
    let (base_url, requests_rx) = spawn_resilient_http_server().await;
    let client = Client::new(
        Config::new("sk-test")
            .with_base_url(base_url)
            .with_model("gpt-test"),
    );

    let first_error = client
        .responses()
        .create(&ResponseRequest {
            input: vec![ResponseInputItem::message(
                ResponseInputRole::User,
                vec![ResponseInputContentPart::input_text("first attempt")],
            )],
            ..ResponseRequest::default()
        })
        .await
        .unwrap_err();

    let second_response = client
        .responses()
        .create(&ResponseRequest {
            input: vec![ResponseInputItem::message(
                ResponseInputRole::User,
                vec![ResponseInputContentPart::input_text("second attempt")],
            )],
            ..ResponseRequest::default()
        })
        .await
        .unwrap();

    match first_error {
        Error::Inference(message) => {
            assert_eq!(
                message,
                "500 Internal Server Error: transient upstream failure"
            );
        }
        other => panic!("expected inference error, got {other:?}"),
    }

    let requests = requests_rx.await.unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(second_response.id.as_deref(), Some("resp_recovered"));

    let request_texts = requests
        .iter()
        .map(|request| {
            request
                .body
                .pointer("/input/0/content/0/text")
                .and_then(Value::as_str)
                .unwrap()
        })
        .collect::<Vec<_>>();
    assert_eq!(request_texts, vec!["first attempt", "second attempt"]);
}

#[tokio::test]
async fn create_response_uses_reloaded_client_config_with_shared_http_transport() {
    let first_response_body = serde_json::json!({
        "id": "resp_initial",
        "object": "response",
        "status": "completed",
        "output": [],
        "usage": {
            "input_tokens": 1,
            "output_tokens": 1,
            "total_tokens": 2
        }
    })
    .to_string();
    let second_response_body = serde_json::json!({
        "id": "resp_reloaded",
        "object": "response",
        "status": "completed",
        "output": [],
        "usage": {
            "input_tokens": 1,
            "output_tokens": 1,
            "total_tokens": 2
        }
    })
    .to_string();

    let (first_base_url, first_request_rx) =
        spawn_http_server("200 OK", "application/json", first_response_body).await;
    let (second_base_url, second_request_rx) =
        spawn_http_server("200 OK", "application/json", second_response_body).await;

    let shared_http = reqwest::Client::builder()
        .pool_max_idle_per_host(1)
        .build()
        .unwrap();

    let initial_client = Client::with_http_client(
        Config::new("sk-initial")
            .with_base_url(first_base_url)
            .with_model("gpt-initial")
            .with_default_header("x-config-revision", "initial"),
        shared_http.clone(),
    );
    let reloaded_client = Client::with_http_client(
        Config::new("sk-reloaded")
            .with_base_url(second_base_url)
            .with_model("gpt-reloaded")
            .with_default_header("x-config-revision", "reloaded"),
        shared_http,
    );

    let initial_response = initial_client
        .responses()
        .create(&ResponseRequest {
            input: vec![ResponseInputItem::message(
                ResponseInputRole::User,
                vec![ResponseInputContentPart::input_text("initial config")],
            )],
            ..ResponseRequest::default()
        })
        .await
        .unwrap();
    let reloaded_response = reloaded_client
        .responses()
        .create(&ResponseRequest {
            input: vec![ResponseInputItem::message(
                ResponseInputRole::User,
                vec![ResponseInputContentPart::input_text("reloaded config")],
            )],
            ..ResponseRequest::default()
        })
        .await
        .unwrap();

    let first_request = first_request_rx.await.unwrap();
    let second_request = second_request_rx.await.unwrap();
    let first_head = first_request.head.to_lowercase();
    let second_head = second_request.head.to_lowercase();

    assert!(first_head.contains("authorization: bearer sk-initial"));
    assert!(first_head.contains("x-config-revision: initial"));
    assert_eq!(
        first_request.body.get("model").and_then(Value::as_str),
        Some("gpt-initial")
    );

    assert!(second_head.contains("authorization: bearer sk-reloaded"));
    assert!(second_head.contains("x-config-revision: reloaded"));
    assert_eq!(
        second_request.body.get("model").and_then(Value::as_str),
        Some("gpt-reloaded")
    );

    assert_eq!(initial_response.id.as_deref(), Some("resp_initial"));
    assert_eq!(reloaded_response.id.as_deref(), Some("resp_reloaded"));
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
