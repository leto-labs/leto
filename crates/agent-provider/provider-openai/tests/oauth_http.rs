use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{Duration, Utc};
use provider_openai::{OpenAiOAuthCredentials, refresh_access_token};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

struct CapturedFormRequest {
    head: String,
    body: String,
}

fn make_jwt(payload_json: &str) -> String {
    let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"RS256","typ":"JWT"}"#);
    let payload = URL_SAFE_NO_PAD.encode(payload_json);
    let sig = URL_SAFE_NO_PAD.encode("sig");
    format!("{header}.{payload}.{sig}")
}

async fn read_http_form_request(socket: &mut tokio::net::TcpStream) -> CapturedFormRequest {
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

    CapturedFormRequest { head, body }
}

async fn spawn_token_server(
    response_body: String,
) -> (String, oneshot::Receiver<CapturedFormRequest>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = oneshot::channel();

    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let request = read_http_form_request(&mut socket).await;
        let _ = tx.send(request);

        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            response_body.len(),
            response_body
        );
        socket.write_all(response.as_bytes()).await.unwrap();
        socket.shutdown().await.unwrap();
    });

    (format!("http://{addr}/oauth/token"), rx)
}

#[tokio::test]
async fn refresh_access_token_posts_refresh_form_and_parses_credentials() {
    let access_token = make_jwt(r#"{"chatgpt_account_id":"acct_123"}"#);
    let response_body = serde_json::json!({
        "access_token": access_token,
        "refresh_token": "refresh-next",
        "expires_in": 3600,
        "token_type": "Bearer",
        "scope": "openid profile email offline_access"
    })
    .to_string();

    let (token_endpoint, request_rx) = spawn_token_server(response_body).await;
    let credentials = OpenAiOAuthCredentials {
        access_token: "access-old".into(),
        refresh_token: "refresh-old".into(),
        expires_at: Utc::now() - Duration::minutes(5),
        client_id: "client-123".into(),
        token_endpoint: token_endpoint.clone(),
        account_id: Some("acct_old".into()),
        token_type: Some("Bearer".into()),
        scopes: vec!["openid".into()],
    };

    let refreshed = refresh_access_token(&reqwest::Client::new(), &credentials)
        .await
        .unwrap();

    let request = request_rx.await.unwrap();
    let request_head = request.head.to_lowercase();
    assert!(request_head.contains("post /oauth/token http/1.1"));
    assert!(request_head.contains("content-type: application/x-www-form-urlencoded"));
    assert!(request.body.contains("grant_type=refresh_token"));
    assert!(request.body.contains("client_id=client-123"));
    assert!(request.body.contains("refresh_token=refresh-old"));

    assert_eq!(refreshed.access_token, access_token);
    assert_eq!(refreshed.refresh_token, "refresh-next");
    assert_eq!(refreshed.client_id, "client-123");
    assert_eq!(refreshed.token_endpoint, token_endpoint);
    assert_eq!(refreshed.account_id.as_deref(), Some("acct_123"));
    assert_eq!(refreshed.token_type.as_deref(), Some("Bearer"));
    assert_eq!(
        refreshed.scopes,
        vec!["openid", "profile", "email", "offline_access"]
    );
    assert!(refreshed.expires_at > Utc::now() + Duration::minutes(50));
}
