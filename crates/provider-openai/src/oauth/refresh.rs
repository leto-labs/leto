use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::Error;

use super::jwt::extract_account_id;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenAiOAuthCredentials {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: chrono::DateTime<Utc>,
    pub client_id: String,
    pub token_endpoint: String,
    pub account_id: Option<String>,
    pub token_type: Option<String>,
    #[serde(default)]
    pub scopes: Vec<String>,
}

impl OpenAiOAuthCredentials {
    pub fn needs_refresh(&self) -> bool {
        Utc::now() >= self.expires_at - Duration::seconds(60)
    }
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

pub async fn refresh_access_token(
    client: &reqwest::Client,
    creds: &OpenAiOAuthCredentials,
) -> Result<OpenAiOAuthCredentials, Error> {
    let response = client
        .post(&creds.token_endpoint)
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", creds.client_id.as_str()),
            ("refresh_token", creds.refresh_token.as_str()),
        ])
        .send()
        .await
        .map_err(|e| Error::Auth(format!("refresh request failed: {e}")))?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(Error::Auth(format!(
            "token refresh failed ({status}): {body}"
        )));
    }
    let token: TokenResponse = response
        .json()
        .await
        .map_err(|e| Error::Auth(format!("failed to parse refresh response: {e}")))?;
    Ok(OpenAiOAuthCredentials {
        access_token: token.access_token.clone(),
        refresh_token: token
            .refresh_token
            .unwrap_or_else(|| creds.refresh_token.clone()),
        expires_at: Utc::now() + Duration::seconds(token.expires_in),
        client_id: creds.client_id.clone(),
        token_endpoint: creds.token_endpoint.clone(),
        account_id: extract_account_id(&token.access_token).or_else(|| creds.account_id.clone()),
        token_type: token.token_type.or_else(|| creds.token_type.clone()),
        scopes: token
            .scope
            .map(|value| value.split_whitespace().map(ToOwned::to_owned).collect())
            .unwrap_or_else(|| creds.scopes.clone()),
    })
}
