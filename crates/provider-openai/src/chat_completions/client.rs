use std::pin::Pin;

use futures::Stream;

use crate::chat_completions::parser::{
    parse_chat_completion_object_value, sse_stream_from_response,
};
use crate::chat_completions::types::{ChatCompletionRequest, ChatCompletionStreamOptions};
use crate::client::Client;
use crate::shared::{ensure_success, json_value};
use crate::Error;

pub type ChatCompletionStream =
    Pin<Box<dyn Stream<Item = Result<crate::chat_completions::ChatCompletionChunk, Error>> + Send>>;

pub struct ChatCompletionsClient<'a> {
    client: &'a Client,
}

impl<'a> ChatCompletionsClient<'a> {
    pub(crate) fn new(client: &'a Client) -> Self {
        Self { client }
    }

    fn normalize_request(&self, request: &mut ChatCompletionRequest) {
        if request.model.is_none() {
            request.model = Some(self.client.config().default_model.clone());
        }
    }

    pub async fn create(
        &self,
        request: &ChatCompletionRequest,
    ) -> Result<crate::chat_completions::ChatCompletionObject, Error> {
        let mut body = request.clone();
        self.normalize_request(&mut body);

        let response = self
            .client
            .http()
            .post(self.client.endpoint_url("chat/completions"))
            .header("Authorization", self.client.auth_header())
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Inference(e.to_string()))?;

        let response = ensure_success(response).await?;
        parse_chat_completion_object_value(json_value(response).await?)
    }

    pub async fn stream(
        &self,
        request: &ChatCompletionRequest,
    ) -> Result<ChatCompletionStream, Error> {
        let mut body = request.clone();
        self.normalize_request(&mut body);
        if body.stream_options.is_none() {
            body.stream_options = Some(ChatCompletionStreamOptions {
                include_usage: true,
            });
        }

        let mut payload = serde_json::to_value(&body).map_err(Error::Json)?;
        let object = payload
            .as_object_mut()
            .ok_or_else(|| Error::Internal("chat completions request must serialize to an object".into()))?;
        object.insert("stream".into(), serde_json::Value::Bool(true));

        let response = self
            .client
            .http()
            .post(self.client.endpoint_url("chat/completions"))
            .header("Authorization", self.client.auth_header())
            .header("accept", "text/event-stream")
            .json(&payload)
            .send()
            .await
            .map_err(|e| Error::Inference(e.to_string()))?;

        let response = ensure_success(response).await?;
        Ok(sse_stream_from_response(response))
    }
}
