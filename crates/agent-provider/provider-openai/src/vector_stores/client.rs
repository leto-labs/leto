//! Concrete client for the Vector Stores API surface.
//!
//! Official reference:
//! - Create vector store: <https://platform.openai.com/docs/api-reference/vector-stores/create>

use crate::Error;
use crate::client::Client;
use crate::shared::{ensure_success, json_value};
use crate::vector_stores::types::{VectorStoreCreateRequest, VectorStoreObject};

/// Handle for Vector Store operations scoped to a parent [`crate::Client`].
pub struct VectorStoresClient<'a> {
    client: &'a Client,
}

impl<'a> VectorStoresClient<'a> {
    pub(crate) fn new(client: &'a Client) -> Self {
        Self { client }
    }

    /// Creates a vector store.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if the HTTP request fails or the response cannot be
    /// parsed into a [`VectorStoreObject`].
    pub async fn create(
        &self,
        request: &VectorStoreCreateRequest,
    ) -> Result<VectorStoreObject, Error> {
        let response = self
            .client
            .apply_default_headers(
                self.client
                    .http()
                    .post(self.client.endpoint_url("vector_stores"))
                    .header("Authorization", self.client.auth_header())
                    .header("OpenAI-Beta", "assistants=v2"),
            )
            .json(request)
            .send()
            .await
            .map_err(|e| Error::Inference(e.to_string()))?;

        let response = ensure_success(response).await?;
        serde_json::from_value(json_value(response).await?).map_err(Error::Json)
    }
}
