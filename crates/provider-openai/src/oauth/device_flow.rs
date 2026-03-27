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
