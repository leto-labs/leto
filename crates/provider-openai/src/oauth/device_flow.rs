use chrono::Utc;
use serde::Deserialize;

use crate::Error;

use super::jwt::extract_account_id;
use super::refresh::OpenAiOAuthCredentials;

pub struct DeviceFlowConfig {
    pub device_code_url: String,
    pub device_token_url: String,
    pub token_url: String,
    pub client_id: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone)]
pub struct DeviceUserPrompt {
    pub user_code: String,
    pub verification_url: String,
}

#[derive(Deserialize)]
struct DeviceCodeResponse {
    device_auth_id: String,
    user_code: String,
    interval: String,
}

#[derive(Deserialize)]
struct DeviceAuthSuccess {
    authorization_code: String,
    code_verifier: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
    #[serde(default)]
    token_type: Option<String>,
    #[serde(default)]
    scope: Option<String>,
}

pub async fn run_device_flow<F>(
    client: &reqwest::Client,
    config: &DeviceFlowConfig,
    verification_url: &str,
    on_prompt: F,
) -> Result<OpenAiOAuthCredentials, Error>
where
    F: FnOnce(DeviceUserPrompt),
{
    let device: DeviceCodeResponse = client
        .post(&config.device_code_url)
        .header("User-Agent", "provider-openai/0.1.0")
        .json(&serde_json::json!({ "client_id": config.client_id }))
        .send()
        .await
        .map_err(|e| Error::Auth(format!("device code request failed: {e}")))?
        .error_for_status()
        .map_err(|e| Error::Auth(format!("device code request failed: {e}")))?
        .json()
        .await
        .map_err(|e| Error::Auth(format!("failed to parse device code response: {e}")))?;

    on_prompt(DeviceUserPrompt {
        user_code: device.user_code.clone(),
        verification_url: format!("{}?user_code={}", verification_url, device.user_code),
    });

    let mut interval_ms = device.interval.parse::<u64>().unwrap_or(5).max(1) * 1000;
    let auth = loop {
        tokio::time::sleep(std::time::Duration::from_millis(interval_ms)).await;
        let response = client
            .post(&config.device_token_url)
            .header("User-Agent", "provider-openai/0.1.0")
            .json(&serde_json::json!({
                "device_auth_id": device.device_auth_id,
                "user_code": device.user_code,
            }))
            .send()
            .await
            .map_err(|e| Error::Auth(format!("device token poll failed: {e}")))?;
        if response.status().is_success() {
            break response
                .json::<DeviceAuthSuccess>()
                .await
                .map_err(|e| Error::Auth(format!("failed to parse device auth response: {e}")))?;
        }
        if response.status().as_u16() == 403 || response.status().as_u16() == 404 {
            continue;
        }
        if response.status().as_u16() == 429 {
            interval_ms += 5_000;
            continue;
        }
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(Error::Auth(format!(
            "device token poll failed ({status}): {body}"
        )));
    };

    let response = client
        .post(&config.token_url)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", auth.authorization_code.as_str()),
            ("redirect_uri", config.redirect_uri.as_str()),
            ("client_id", config.client_id.as_str()),
            ("code_verifier", auth.code_verifier.as_str()),
        ])
        .send()
        .await
        .map_err(|e| Error::Auth(format!("token exchange failed: {e}")))?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(Error::Auth(format!(
            "token exchange failed ({status}): {body}"
        )));
    }
    let token: TokenResponse = response
        .json()
        .await
        .map_err(|e| Error::Auth(format!("failed to parse token response: {e}")))?;
    Ok(OpenAiOAuthCredentials {
        access_token: token.access_token.clone(),
        refresh_token: token.refresh_token.unwrap_or_default(),
        expires_at: Utc::now() + chrono::Duration::seconds(token.expires_in.unwrap_or(3600)),
        client_id: config.client_id.clone(),
        token_endpoint: config.token_url.clone(),
        account_id: extract_account_id(&token.access_token),
        token_type: token.token_type,
        scopes: token
            .scope
            .map(|value| value.split_whitespace().map(ToOwned::to_owned).collect())
            .unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use chrono::{Duration, Utc};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    use super::*;

    fn make_jwt(payload_json: &str) -> String {
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"RS256","typ":"JWT"}"#);
        let payload = URL_SAFE_NO_PAD.encode(payload_json);
        let sig = URL_SAFE_NO_PAD.encode("sig");
        format!("{header}.{payload}.{sig}")
    }

    fn json_response(status_line: &str, body: String) -> String {
        format!(
            "HTTP/1.1 {status_line}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            body.len(),
            body
        )
    }

    async fn read_request(socket: &mut tokio::net::TcpStream) -> (String, String) {
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

        (
            head,
            String::from_utf8_lossy(&request[header_end..header_end + content_length]).into(),
        )
    }

    #[tokio::test]
    async fn device_flow_retries_after_rate_limit_poll() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let base_url = format!("http://{addr}");
        let poll_attempts = Arc::new(AtomicUsize::new(0));
        let poll_attempts_for_server = poll_attempts.clone();
        let access_token = make_jwt(r#"{"chatgpt_account_id":"acct_retry"}"#);

        let server = tokio::spawn(async move {
            for step in 0..4 {
                let (mut socket, _) = listener.accept().await.unwrap();
                let (head, body) = read_request(&mut socket).await;
                let request_head = head.to_lowercase();

                let response = match step {
                    0 => {
                        assert!(request_head.contains("post /oauth/device/code http/1.1"));
                        assert!(body.contains("\"client_id\":\"client-123\""));
                        json_response(
                            "200 OK",
                            serde_json::json!({
                                "device_auth_id": "device-auth-1",
                                "user_code": "ABCD-1234",
                                "interval": "1"
                            })
                            .to_string(),
                        )
                    }
                    1 => {
                        poll_attempts_for_server.fetch_add(1, Ordering::SeqCst);
                        assert!(request_head.contains("post /oauth/device/token http/1.1"));
                        assert!(body.contains("\"device_auth_id\":\"device-auth-1\""));
                        assert!(body.contains("\"user_code\":\"ABCD-1234\""));
                        json_response("429 Too Many Requests", "{}".to_owned())
                    }
                    2 => {
                        poll_attempts_for_server.fetch_add(1, Ordering::SeqCst);
                        assert!(request_head.contains("post /oauth/device/token http/1.1"));
                        json_response(
                            "200 OK",
                            serde_json::json!({
                                "authorization_code": "auth-code-1",
                                "code_verifier": "verifier-1"
                            })
                            .to_string(),
                        )
                    }
                    3 => {
                        assert!(request_head.contains("post /oauth/token http/1.1"));
                        assert!(body.contains("grant_type=authorization_code"));
                        assert!(body.contains("code=auth-code-1"));
                        assert!(body.contains("client_id=client-123"));
                        assert!(body.contains("code_verifier=verifier-1"));
                        json_response(
                            "200 OK",
                            serde_json::json!({
                                "access_token": access_token,
                                "refresh_token": "refresh-next",
                                "expires_in": 3600,
                                "token_type": "Bearer",
                                "scope": "openid profile email offline_access"
                            })
                            .to_string(),
                        )
                    }
                    _ => unreachable!(),
                };

                socket.write_all(response.as_bytes()).await.unwrap();
                socket.shutdown().await.unwrap();
            }
        });

        let config = DeviceFlowConfig {
            device_code_url: format!("{base_url}/oauth/device/code"),
            device_token_url: format!("{base_url}/oauth/device/token"),
            token_url: format!("{base_url}/oauth/token"),
            client_id: "client-123".into(),
            redirect_uri: "https://auth.openai.com/deviceauth/callback".into(),
        };

        let mut prompted = None;
        let credentials = run_device_flow(
            &reqwest::Client::new(),
            &config,
            "https://auth.openai.com/codex/device",
            |prompt| prompted = Some(prompt),
        )
        .await
        .unwrap();

        server.await.unwrap();

        let prompt = prompted.expect("device flow should emit prompt");
        assert_eq!(prompt.user_code, "ABCD-1234");
        assert_eq!(
            prompt.verification_url,
            "https://auth.openai.com/codex/device?user_code=ABCD-1234"
        );
        assert_eq!(poll_attempts.load(Ordering::SeqCst), 2);
        assert_eq!(credentials.refresh_token, "refresh-next");
        assert_eq!(credentials.account_id.as_deref(), Some("acct_retry"));
        assert_eq!(credentials.token_type.as_deref(), Some("Bearer"));
        assert_eq!(
            credentials.scopes,
            vec!["openid", "profile", "email", "offline_access"]
        );
        assert!(credentials.expires_at > Utc::now() + Duration::minutes(50));
    }
}
