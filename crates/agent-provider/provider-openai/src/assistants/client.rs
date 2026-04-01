//! Concrete client for the Assistants API surface.
//!
//! Official reference:
//! - Create assistant: <https://platform.openai.com/docs/api-reference/assistants>

use crate::Error;
use crate::assistants::types::{AssistantCreateRequest, AssistantObject};
use crate::client::Client;
use crate::shared::{ensure_success, json_value};

/// Handle for Assistant operations scoped to a parent [`crate::Client`].
pub struct AssistantsClient<'a> {
    client: &'a Client,
}

impl<'a> AssistantsClient<'a> {
    pub(crate) fn new(client: &'a Client) -> Self {
        Self { client }
    }

    fn normalize_request(&self, request: &mut AssistantCreateRequest) {
        if request.model.is_none() {
            request.model = Some(self.client.config().default_model.clone());
        }
    }

    /// Creates an assistant.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if the HTTP request fails or the response cannot be
    /// parsed into an [`AssistantObject`].
    pub async fn create(&self, request: &AssistantCreateRequest) -> Result<AssistantObject, Error> {
        let mut body = request.clone();
        self.normalize_request(&mut body);

        let response = self
            .client
            .apply_default_headers(
                self.client
                    .http()
                    .post(self.client.endpoint_url("assistants"))
                    .header("Authorization", self.client.auth_header())
                    .header("OpenAI-Beta", "assistants=v2"),
            )
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Inference(e.to_string()))?;

        let response = ensure_success(response).await?;
        serde_json::from_value(json_value(response).await?).map_err(Error::Json)
    }
}
