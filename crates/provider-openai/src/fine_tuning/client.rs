//! Concrete client for the Fine-Tuning API surface.
//!
//! Official reference:
//! - List fine-tuning jobs: <https://platform.openai.com/docs/api-reference/fine-tuning/jobs/list>

use crate::Error;
use crate::client::Client;
use crate::fine_tuning::types::FineTuningJobPage;
use crate::shared::{ensure_success, json_value};

/// Handle for Fine-Tuning operations scoped to a parent [`crate::Client`].
pub struct FineTuningClient<'a> {
    client: &'a Client,
}

impl<'a> FineTuningClient<'a> {
    pub(crate) fn new(client: &'a Client) -> Self {
        Self { client }
    }

    /// Lists fine-tuning jobs.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if the HTTP request fails or the response cannot be
    /// parsed into a [`FineTuningJobPage`].
    pub async fn list_jobs(&self) -> Result<FineTuningJobPage, Error> {
        let response = self
            .client
            .apply_default_headers(
                self.client
                    .http()
                    .get(self.client.endpoint_url("fine_tuning/jobs"))
                    .header("Authorization", self.client.auth_header()),
            )
            .send()
            .await
            .map_err(|e| Error::Inference(e.to_string()))?;

        let response = ensure_success(response).await?;
        serde_json::from_value(json_value(response).await?).map_err(Error::Json)
    }
}
