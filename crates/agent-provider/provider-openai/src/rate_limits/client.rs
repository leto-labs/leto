//! Concrete client for the Rate Limits API surface.
//!
//! Official reference:
//! - List project rate limits: <https://platform.openai.com/docs/api-reference/project-rate-limits/list>

use crate::Error;
use crate::client::Client;
use crate::rate_limits::types::ProjectRateLimitPage;
use crate::shared::{ensure_success, json_value};

/// Handle for Rate Limit operations scoped to a parent [`crate::Client`].
pub struct RateLimitsClient<'a> {
    client: &'a Client,
}

impl<'a> RateLimitsClient<'a> {
    pub(crate) fn new(client: &'a Client) -> Self {
        Self { client }
    }

    /// Lists rate limits configured for an organization project.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if the HTTP request fails, the project id cannot be
    /// joined into a valid endpoint path, or the response cannot be parsed into
    /// a [`ProjectRateLimitPage`].
    pub async fn list_project(&self, project_id: &str) -> Result<ProjectRateLimitPage, Error> {
        let url = self.client.endpoint_joined_url(&[
            "organization",
            "projects",
            project_id,
            "rate_limits",
        ])?;
        let response = self
            .client
            .apply_default_headers(
                self.client
                    .http()
                    .get(url)
                    .header("Authorization", self.client.auth_header()),
            )
            .send()
            .await
            .map_err(|e| Error::Inference(e.to_string()))?;

        let response = ensure_success(response).await?;
        serde_json::from_value(json_value(response).await?).map_err(Error::Json)
    }
}
