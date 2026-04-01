//! Concrete client for Anthropic's Messages API.
//!
//! Official references:
//! - Messages create: <https://platform.claude.com/docs/en/api/messages/create>
//! - Streaming: <https://platform.claude.com/docs/en/build-with-claude/streaming>

use std::pin::Pin;

use futures::Stream;

use crate::Error;
use crate::client::Client;
use crate::messages::parser::{parse_message_object_value, sse_stream_from_response};
use crate::messages::types::MessageRequest;
use crate::shared::{ensure_success, json_value};

/// Streaming type returned by [`MessagesClient::stream`].
pub type MessageStream =
    Pin<Box<dyn Stream<Item = Result<crate::messages::MessageStreamEvent, Error>> + Send>>;

/// Handle for Messages API operations scoped to a parent [`crate::Client`].
pub struct MessagesClient<'a> {
    client: &'a Client,
}

impl<'a> MessagesClient<'a> {
    pub(crate) fn new(client: &'a Client) -> Self {
        Self { client }
    }

    fn normalize_request(&self, request: &mut MessageRequest) {
        if request.model.is_none() {
            request.model = Some(self.client.config().default_model.clone());
        }
    }

    /// Creates a non-streaming message response.
    pub async fn create(
        &self,
        request: &MessageRequest,
    ) -> Result<crate::messages::MessageObject, Error> {
        let mut body = request.clone();
        self.normalize_request(&mut body);
        body.stream = Some(false);

        let response = self
            .client
            .http()
            .post(self.client.endpoint_url("v1/messages"))
            .header("x-api-key", self.client.config().api_key.as_str())
            .header("anthropic-version", self.client.config().version.as_str())
            .header("accept", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Inference(e.to_string()))?;

        let response = ensure_success(response).await?;
        parse_message_object_value(json_value(response).await?)
    }

    /// Creates a streaming message response over server-sent events.
    pub async fn stream(&self, request: &MessageRequest) -> Result<MessageStream, Error> {
        let mut body = request.clone();
        self.normalize_request(&mut body);
        body.stream = Some(true);

        let response = self
            .client
            .http()
            .post(self.client.endpoint_url("v1/messages"))
            .header("x-api-key", self.client.config().api_key.as_str())
            .header("anthropic-version", self.client.config().version.as_str())
            .header("accept", "text/event-stream")
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Inference(e.to_string()))?;

        let response = ensure_success(response).await?;
        Ok(sse_stream_from_response(response))
    }
}
