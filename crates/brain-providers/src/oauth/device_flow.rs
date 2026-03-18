use chrono::Utc;
use serde::Deserialize;
use tracing::debug;

use brain_types::BrainError;

use super::jwt;
use brain_types::OAuthCredentials;

/// Configuration for the device code OAuth flow.
pub struct DeviceFlowConfig {
    pub device_code_url: String,
    pub device_token_url: String,
    pub token_url: String,
    pub client_id: String,
    pub redirect_uri: String,
}

/// Information to display to the user during the device code flow.
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

/// Intermediate response from the device token poll on success.
/// OpenAI returns an authorization_code + code_verifier that must
/// then be exchanged at the standard /oauth/token endpoint.
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
}

/// Run the device code OAuth flow (OpenAI-specific).
///
/// 1. Request a device code
/// 2. Call `on_user_prompt` with the user code + verification URL for display
/// 3. Poll the device token endpoint until the user authorizes
/// 4. Exchange the authorization code for tokens
/// 5. Return `OAuthCredentials`
pub async fn run<F>(
    client: &reqwest::Client,
    config: &DeviceFlowConfig,
    verification_url: &str,
    on_user_prompt: F,
) -> Result<OAuthCredentials, BrainError>
where
    F: FnOnce(DeviceUserPrompt),
{
    let device_resp = request_device_code(client, config).await?;

    let interval_ms = device_resp.interval.parse::<u64>().unwrap_or(5).max(1) * 1000 + 3000; // safety margin like OpenCode

    let full_url = format!("{}?user_code={}", verification_url, device_resp.user_code);

    on_user_prompt(DeviceUserPrompt {
        user_code: device_resp.user_code.clone(),
        verification_url: full_url,
    });

    let auth = poll_for_auth(client, config, &device_resp, interval_ms).await?;

    exchange_code(client, config, &auth).await
}

async fn request_device_code(
    client: &reqwest::Client,
    config: &DeviceFlowConfig,
) -> Result<DeviceCodeResponse, BrainError> {
    let resp = client
        .post(&config.device_code_url)
        .header("User-Agent", "brain/0.1.0")
        .json(&serde_json::json!({ "client_id": config.client_id }))
        .send()
        .await
        .map_err(|e| BrainError::Auth(format!("device code request failed: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(BrainError::Auth(format!(
            "device code request failed ({status}): {body}"
        )));
    }

    resp.json()
        .await
        .map_err(|e| BrainError::Auth(format!("failed to parse device code response: {e}")))
}

async fn poll_for_auth(
    client: &reqwest::Client,
    config: &DeviceFlowConfig,
    device_resp: &DeviceCodeResponse,
    interval_ms: u64,
) -> Result<DeviceAuthSuccess, BrainError> {
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(interval_ms)).await;

        let resp = client
            .post(&config.device_token_url)
            .header("User-Agent", "brain/0.1.0")
            .json(&serde_json::json!({
                "device_auth_id": device_resp.device_auth_id,
                "user_code": device_resp.user_code,
            }))
            .send()
            .await
            .map_err(|e| BrainError::Auth(format!("device token poll failed: {e}")))?;

        let status = resp.status();

        if status.is_success() {
            return resp.json().await.map_err(|e| {
                BrainError::Auth(format!("failed to parse device auth response: {e}"))
            });
        }

        // 403/404 = authorization pending, keep polling
        if status.as_u16() == 403 || status.as_u16() == 404 {
            debug!("device flow: authorization pending, polling...");
            continue;
        }

        let body = resp.text().await.unwrap_or_default();
        return Err(BrainError::Auth(format!(
            "device token poll failed ({status}): {body}"
        )));
    }
}

async fn exchange_code(
    client: &reqwest::Client,
    config: &DeviceFlowConfig,
    auth: &DeviceAuthSuccess,
) -> Result<OAuthCredentials, BrainError> {
    let resp = client
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
        .map_err(|e| BrainError::Auth(format!("token exchange failed: {e}")))?;

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
        expires_at: Utc::now() + chrono::Duration::seconds(token_resp.expires_in.unwrap_or(3600)),
        client_id: config.client_id.clone(),
        token_endpoint: config.token_url.clone(),
        account_id,
    })
}
