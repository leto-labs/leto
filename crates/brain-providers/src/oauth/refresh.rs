use chrono::{Duration, Utc};
use serde::Deserialize;

use brain_types::BrainError;

use super::jwt;
use brain_types::OAuthCredentials;

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: i64,
}

/// Refresh an OAuth access token using the refresh_token grant.
///
/// This function is stateless — callers are responsible for persisting
/// the returned credentials via `CredentialStore`.
pub async fn refresh_token(
    client: &reqwest::Client,
    creds: &OAuthCredentials,
) -> Result<OAuthCredentials, BrainError> {
    let resp = client
        .post(&creds.token_endpoint)
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", &creds.client_id),
            ("refresh_token", &creds.refresh_token),
        ])
        .send()
        .await
        .map_err(|e| BrainError::Auth(format!("refresh request failed: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(BrainError::Auth(format!(
            "token refresh failed ({status}): {body}"
        )));
    }

    let token_resp: TokenResponse = resp
        .json()
        .await
        .map_err(|e| BrainError::Auth(format!("failed to parse refresh response: {e}")))?;

    let account_id =
        jwt::extract_account_id(&token_resp.access_token).or_else(|| creds.account_id.clone());

    Ok(OAuthCredentials {
        access_token: token_resp.access_token,
        refresh_token: token_resp
            .refresh_token
            .unwrap_or_else(|| creds.refresh_token.clone()),
        expires_at: Utc::now() + Duration::seconds(token_resp.expires_in),
        client_id: creds.client_id.clone(),
        token_endpoint: creds.token_endpoint.clone(),
        account_id,
    })
}
