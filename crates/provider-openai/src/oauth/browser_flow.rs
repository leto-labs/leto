use std::time::Duration;

use chrono::Utc;
use serde::Deserialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::Error;

use super::jwt::extract_account_id;
use super::pkce::pkce_verifier;
use super::refresh::OpenAiOAuthCredentials;

pub struct BrowserOAuthConfig {
    pub authorize_url: String,
    pub token_url: String,
    pub client_id: String,
    pub scopes: String,
    pub callback_port: u16,
    pub timeout: Duration,
}

impl BrowserOAuthConfig {
    pub fn redirect_uri(&self) -> String {
        format!("http://localhost:{}/auth/callback", self.callback_port)
    }
}

#[derive(Debug, Clone)]
pub struct BrowserFlowPrompt {
    pub url: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: i64,
    #[serde(default)]
    token_type: Option<String>,
    #[serde(default)]
    scope: Option<String>,
}

pub async fn run_browser_flow<F>(
    client: &reqwest::Client,
    config: &BrowserOAuthConfig,
    on_prompt: F,
) -> Result<OpenAiOAuthCredentials, Error>
where
    F: FnOnce(BrowserFlowPrompt),
{
    let (verifier, challenge) = pkce_verifier();
    let state = format!("{}", rand::random::<u128>());
    let redirect_uri = config.redirect_uri();
    let authorize_url = format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&code_challenge={}&code_challenge_method=S256&scope={}&state={}",
        config.authorize_url,
        form_urlencoded::byte_serialize(config.client_id.as_bytes()).collect::<String>(),
        form_urlencoded::byte_serialize(redirect_uri.as_bytes()).collect::<String>(),
        form_urlencoded::byte_serialize(challenge.as_bytes()).collect::<String>(),
        form_urlencoded::byte_serialize(config.scopes.as_bytes()).collect::<String>(),
        state,
    );

    let listener = TcpListener::bind(format!("127.0.0.1:{}", config.callback_port))
        .await
        .map_err(|e| Error::Auth(format!("failed to bind callback port: {e}")))?;

    on_prompt(BrowserFlowPrompt {
        url: authorize_url.clone(),
    });

    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open")
        .arg(&authorize_url)
        .spawn();

    let code = wait_for_callback(&listener, config.timeout, &state).await?;
    exchange_code(client, config, &code, &verifier).await
}

async fn wait_for_callback(
    listener: &TcpListener,
    timeout: Duration,
    expected_state: &str,
) -> Result<String, Error> {
    let deadline = tokio::time::Instant::now() + timeout;
    let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
    if remaining.is_zero() {
        return Err(Error::Auth("OAuth callback timed out".into()));
    }
    let (mut stream, _) = tokio::time::timeout(remaining, listener.accept())
        .await
        .map_err(|_| Error::Auth("OAuth callback timed out".into()))?
        .map_err(|e| Error::Auth(format!("callback accept failed: {e}")))?;

    let mut buf = vec![0u8; 4096];
    let n = stream
        .read(&mut buf)
        .await
        .map_err(|e| Error::Auth(format!("callback read failed: {e}")))?;
    let request = String::from_utf8_lossy(&buf[..n]);
    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("");
    let query = path.split_once('?').map(|(_, value)| value).unwrap_or("");
    let state = extract_param(query, "state");
    if state.as_deref() != Some(expected_state) {
        let _ = stream
            .write_all(b"HTTP/1.1 403 Forbidden\r\nConnection: close\r\n\r\nstate mismatch")
            .await;
        return Err(Error::Auth("OAuth state mismatch".into()));
    }
    let code =
        extract_param(query, "code").ok_or_else(|| Error::Auth("missing callback code".into()))?;
    let _ = stream
        .write_all(b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\nok")
        .await;
    Ok(code)
}

fn extract_param(query: &str, name: &str) -> Option<String> {
    let prefix = format!("{name}=");
    query
        .split('&')
        .find_map(|part| part.strip_prefix(&prefix).map(ToOwned::to_owned))
}

async fn exchange_code(
    client: &reqwest::Client,
    config: &BrowserOAuthConfig,
    code: &str,
    verifier: &str,
) -> Result<OpenAiOAuthCredentials, Error> {
    let redirect_uri = config.redirect_uri();
    let response = client
        .post(&config.token_url)
        .form(&[
            ("grant_type", "authorization_code"),
            ("client_id", config.client_id.as_str()),
            ("code", code),
            ("redirect_uri", redirect_uri.as_str()),
            ("code_verifier", verifier),
        ])
        .send()
        .await
        .map_err(|e| Error::Auth(format!("token exchange request failed: {e}")))?;
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
        expires_at: Utc::now() + chrono::Duration::seconds(token.expires_in),
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
