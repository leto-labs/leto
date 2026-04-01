//! Concrete client for the Embeddings API surface.
//!
//! Official reference:
//! - Create an embedding: <https://platform.openai.com/docs/api-reference/embeddings/create>

use crate::Error;
use crate::client::Client;
use crate::embeddings::types::{EmbeddingRequest, EmbeddingResponse};
use crate::shared::{ensure_success, json_value};

/// Handle for Embeddings operations scoped to a parent [`crate::Client`].
pub struct EmbeddingsClient<'a> {
    client: &'a Client,
}

impl<'a> EmbeddingsClient<'a> {
    pub(crate) fn new(client: &'a Client) -> Self {
        Self { client }
    }

    fn normalize_request(&self, request: &mut EmbeddingRequest) {
        if request.model.is_none() {
            request.model = Some(self.client.config().default_model.clone());
        }
    }

    /// Creates embeddings for the provided input.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if the HTTP request fails or the response cannot be
    /// parsed into an [`EmbeddingResponse`].
    pub async fn create(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse, Error> {
        let mut body = request.clone();
        self.normalize_request(&mut body);

        let response = self
            .client
            .apply_default_headers(
                self.client
                    .http()
                    .post(self.client.endpoint_url("embeddings"))
                    .header("Authorization", self.client.auth_header()),
            )
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Inference(e.to_string()))?;

        let response = ensure_success(response).await?;
        serde_json::from_value(json_value(response).await?).map_err(Error::Json)
    }
}
