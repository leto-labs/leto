use std::time::Duration;

use chrono::Utc;
use rand::Rng;
use serde::Deserialize;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tracing::debug;

use brain_types::BrainError;

use brain_types::OAuthCredentials;
use super::jwt;
use super::pkce;

/// Configuration for the browser-based OAuth authorization code + PKCE flow.
pub struct BrowserFlowConfig {
    pub authorize_url: String,
    pub token_url: String,
    pub client_id: String,
    pub scopes: String,
    pub callback_port: u16,
    pub timeout: Duration,
}

impl BrowserFlowConfig {
    pub fn redirect_uri(&self) -> String {
        format!("http://localhost:{}/auth/callback", self.callback_port)
    }
}

/// Information for the user to complete the browser flow.
#[derive(Debug, Clone)]
pub struct BrowserFlowPrompt {
    pub url: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: i64,
}

/// Run the browser OAuth authorization code + PKCE flow.
///
/// 1. Start localhost callback server
/// 2. Call `on_prompt` with the authorization URL for the caller to display
/// 3. Optionally attempt to open the URL in the user's default browser
/// 4. Wait for callback with authorization code (up to timeout)
/// 5. Exchange code for tokens
/// 6. Return `OAuthCredentials`
pub async fn run<F>(
    client: &reqwest::Client,
    config: &BrowserFlowConfig,
    on_prompt: F,
) -> Result<OAuthCredentials, BrainError>
where
    F: FnOnce(BrowserFlowPrompt),
{
    let (verifier, challenge) = pkce::pkce_verifier();
    let state = generate_state();
    let redirect_uri = config.redirect_uri();

    let auth_url = format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&code_challenge={}&code_challenge_method=S256&scope={}&state={}",
        config.authorize_url,
        urlencoded(&config.client_id),
        urlencoded(&redirect_uri),
        urlencoded(&challenge),
        urlencoded(&config.scopes),
        urlencoded(&state),
    );

    let listener = TcpListener::bind(format!("127.0.0.1:{}", config.callback_port))
        .await
        .map_err(|e| BrainError::Auth(format!(
            "failed to bind port {}: {e}", config.callback_port
        )))?;

    on_prompt(BrowserFlowPrompt { url: auth_url.clone() });

    debug!(url = %auth_url, "waiting for OAuth callback");
    try_open_browser(&auth_url);

    let code = wait_for_callback(&listener, config.timeout, &state).await?;

    exchange_code(client, config, &code, &verifier).await
}

fn urlencoded(s: &str) -> String {
    form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

fn generate_state() -> String {
    let mut rng = rand::rng();
    (0..32)
        .map(|_| {
            let idx = rng.random_range(0..36u8);
            if idx < 10 { (b'0' + idx) as char } else { (b'a' + idx - 10) as char }
        })
        .collect()
}

fn try_open_browser(url: &str) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).spawn();
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd").args(["/c", "start", url]).spawn();
}

const SUCCESS_HTML: &str = r#"<html><body style="font-family:sans-serif;text-align:center;padding:60px">
<h2>Authorization Successful</h2>
<p>You can close this window and return to the terminal.</p>
<script>setTimeout(()=>window.close(),2000)</script>
</body></html>"#;

async fn wait_for_callback(
    listener: &TcpListener,
    timeout: Duration,
    expected_state: &str,
) -> Result<String, BrainError> {
    let deadline = tokio::time::Instant::now() + timeout;

    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err(BrainError::Auth("OAuth callback timed out".into()));
        }

        let (mut stream, _addr) = tokio::time::timeout(remaining, listener.accept())
            .await
            .map_err(|_| BrainError::Auth("OAuth callback timed out".into()))?
            .map_err(|e| BrainError::Auth(format!("callback accept failed: {e}")))?;

        let mut buf = vec![0u8; 4096];
        let n = stream
            .read(&mut buf)
            .await
            .map_err(|e| BrainError::Auth(format!("callback read failed: {e}")))?;
        let request = String::from_utf8_lossy(&buf[..n]);

        let path = request
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .unwrap_or("");

        if !path.starts_with("/auth/callback") {
            let response = "HTTP/1.1 404 Not Found\r\nConnection: close\r\n\r\n";
            let _ = stream.write_all(response.as_bytes()).await;
            continue;
        }

        let query = path.split_once('?').map(|(_, q)| q).unwrap_or("");

        let state = extract_param(query, "state");
        if state.as_deref() != Some(expected_state) {
            let response = "HTTP/1.1 403 Forbidden\r\nConnection: close\r\n\r\nstate mismatch";
            let _ = stream.write_all(response.as_bytes()).await;
            return Err(BrainError::Auth("OAuth state mismatch (possible CSRF)".into()));
        }

        let code = extract_param(query, "code")
            .ok_or_else(|| BrainError::Auth("no 'code' parameter in callback".into()))?;

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            SUCCESS_HTML.len(),
            SUCCESS_HTML,
        );
        let _ = stream.write_all(response.as_bytes()).await;

        return Ok(code);
    }
}

fn extract_param(query: &str, name: &str) -> Option<String> {
    let prefix = format!("{name}=");
    for param in query.split('&') {
        if let Some(value) = param.strip_prefix(&prefix) {
            return Some(value.to_owned());
        }
    }
    None
}

async fn exchange_code(
    client: &reqwest::Client,
    config: &BrowserFlowConfig,
    code: &str,
    verifier: &str,
) -> Result<OAuthCredentials, BrainError> {
    let redirect_uri = config.redirect_uri();

    let resp = client
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
        .map_err(|e| BrainError::Auth(format!("token exchange request failed: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(BrainError::Auth(format!(
            "token exchange failed ({status}): {body}"
        )));
    }

    let token_resp: TokenResponse = resp
        .json()
        .await
        .map_err(|e| BrainError::Auth(format!("failed to parse token response: {e}")))?;

    let account_id = jwt::extract_account_id(&token_resp.access_token);

    Ok(OAuthCredentials {
        access_token: token_resp.access_token,
        refresh_token: token_resp.refresh_token.unwrap_or_default(),
        expires_at: Utc::now() + chrono::Duration::seconds(token_resp.expires_in),
        client_id: config.client_id.clone(),
        token_endpoint: config.token_url.clone(),
        account_id,
    })
}
