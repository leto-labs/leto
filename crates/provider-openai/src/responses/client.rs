//! Concrete client for the OpenAI Responses API resource family.
//!
//! Official references:
//! - Create a response: <https://developers.openai.com/api/reference/resources/responses/methods/create>
//! - Retrieve a response: <https://developers.openai.com/api/reference/resources/responses/methods/retrieve>
//! - List input items: <https://developers.openai.com/api/reference/resources/responses/subresources/input_items/methods/list>
//! - Count input tokens: <https://developers.openai.com/api/reference/resources/responses/subresources/input_tokens/methods/count>
//! - Cancel a response: <https://developers.openai.com/api/reference/resources/responses/methods/cancel>
//! - Compact a response: <https://developers.openai.com/api/reference/resources/responses/methods/compact>

use std::pin::Pin;

use futures::{SinkExt, Stream, StreamExt};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::{HeaderName, HeaderValue};

use crate::Error;
use crate::client::Client;
use crate::responses::parser::{
    parse_response_compaction_value, parse_response_input_token_count_value,
    parse_response_item_page_value, parse_response_object_value, parse_response_stream_event,
    sse_stream_from_response,
};
use crate::responses::types::{
    ResponseCompactRequest, ResponseCompaction, ResponseInputTokenCount,
    ResponseInputTokenCountRequest, ResponseItemListRequest, ResponseItemPage, ResponseObject,
    ResponseRequest, ResponseRetrieveRequest, ResponseStreamEvent,
};
use crate::shared::{ensure_success, json_value};

/// Streaming response type returned by [`ResponsesClient::stream`] and
/// [`ResponsesClient::stream_retrieve`].
pub type ResponseStream = Pin<Box<dyn Stream<Item = Result<ResponseStreamEvent, Error>> + Send>>;

/// Streaming transport used when creating a streamed response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseStreamTransport {
    /// Server-sent events over HTTP.
    Sse,
    /// WebSocket transport for response creation.
    WebSocket,
}

#[derive(serde::Serialize)]
struct StreamingResponseRequest<'a> {
    #[serde(flatten)]
    request: &'a ResponseRequest,
    stream: bool,
}

#[derive(serde::Serialize)]
struct WebSocketResponseCreateRequest<'a> {
    #[serde(rename = "type")]
    message_type: &'static str,
    #[serde(flatten)]
    request: &'a ResponseRequest,
}

/// Handle for Responses API operations scoped to a parent [`crate::Client`].
pub struct ResponsesClient<'a> {
    client: &'a Client,
}

impl<'a> ResponsesClient<'a> {
    pub(crate) fn new(client: &'a Client) -> Self {
        Self { client }
    }

    fn normalize_response_request(&self, request: &mut ResponseRequest) {
        if request.model.is_none() {
            request.model = Some(self.client.config().default_model.clone());
        }
        if request.store.is_none() {
            request.store = Some(false);
        }
    }

    fn normalize_input_token_count_request(&self, request: &mut ResponseInputTokenCountRequest) {
        if request.model.is_none() {
            request.model = Some(self.client.config().default_model.clone());
        }
    }

    fn responses_url(&self) -> String {
        self.client.endpoint_url("responses")
    }

    fn response_url(&self, response_id: &str) -> Result<reqwest::Url, Error> {
        self.client.endpoint_joined_url(&["responses", response_id])
    }

    fn responses_compact_url(&self) -> String {
        self.client.endpoint_url("responses/compact")
    }

    fn responses_input_tokens_url(&self) -> String {
        self.client.endpoint_url("responses/input_tokens")
    }

    fn response_input_items_url(&self, response_id: &str) -> Result<reqwest::Url, Error> {
        self.client
            .endpoint_joined_url(&["responses", response_id, "input_items"])
    }

    fn responses_websocket_url(&self) -> String {
        let base = self.client.config().base_url.trim_end_matches('/');
        if let Some(rest) = base.strip_prefix("https://") {
            format!("wss://{rest}/responses")
        } else if let Some(rest) = base.strip_prefix("http://") {
            format!("ws://{rest}/responses")
        } else if base.starts_with("wss://") || base.starts_with("ws://") {
            format!("{base}/responses")
        } else {
            format!("wss://{base}/responses")
        }
    }

    fn response_retrieve_query(
        &self,
        request: &ResponseRetrieveRequest,
        stream: bool,
    ) -> Vec<(String, String)> {
        let mut query = Vec::new();
        for include in &request.include {
            query.push(("include[]".to_owned(), include.clone()));
        }
        if let Some(include_obfuscation) = request.include_obfuscation {
            query.push((
                "include_obfuscation".to_owned(),
                include_obfuscation.to_string(),
            ));
        }
        if let Some(starting_after) = request.starting_after {
            query.push(("starting_after".to_owned(), starting_after.to_string()));
        }
        if stream {
            query.push(("stream".to_owned(), "true".to_owned()));
        }
        query
    }

    fn response_item_list_query(&self, request: &ResponseItemListRequest) -> Vec<(String, String)> {
        let mut query = Vec::new();
        if let Some(after) = &request.after {
            query.push(("after".to_owned(), after.clone()));
        }
        if let Some(limit) = request.limit {
            query.push(("limit".to_owned(), limit.to_string()));
        }
        for include in &request.include {
            query.push(("include[]".to_owned(), include.clone()));
        }
        if let Some(order) = &request.order {
            query.push(("order".to_owned(), order.clone()));
        }
        query
    }

    /// Creates a non-streaming response.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if the HTTP request fails or the payload cannot be
    /// parsed into a [`ResponseObject`].
    pub async fn create(&self, request: &ResponseRequest) -> Result<ResponseObject, Error> {
        let mut body = request.clone();
        self.normalize_response_request(&mut body);

        let response = self
            .client
            .apply_default_headers(
                self.client
                    .http()
                    .post(self.responses_url())
                    .header("Authorization", self.client.auth_header()),
            )
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Inference(e.to_string()))?;

        let response = ensure_success(response).await?;
        Ok(parse_response_object_value(json_value(response).await?))
    }

    pub async fn stream(
        &self,
        request: &ResponseRequest,
        transport: ResponseStreamTransport,
    ) -> Result<ResponseStream, Error> {
        let mut body = request.clone();
        self.normalize_response_request(&mut body);

        match transport {
            ResponseStreamTransport::Sse => {
                let response = self
                    .client
                    .apply_default_headers(
                        self.client
                            .http()
                            .post(self.responses_url())
                            .header("Authorization", self.client.auth_header())
                            .header("accept", "text/event-stream"),
                    )
                    .json(&StreamingResponseRequest {
                        request: &body,
                        stream: true,
                    })
                    .send()
                    .await
                    .map_err(|e| Error::Inference(e.to_string()))?;

                let response = ensure_success(response).await?;
                Ok(sse_stream_from_response(response))
            }
            ResponseStreamTransport::WebSocket => {
                let mut request = self
                    .responses_websocket_url()
                    .into_client_request()
                    .map_err(|e| Error::Internal(e.to_string()))?;
                request.headers_mut().insert(
                    "Authorization",
                    HeaderValue::from_str(&self.client.auth_header())
                        .map_err(|e| Error::Internal(e.to_string()))?,
                );
                request
                    .headers_mut()
                    .insert("originator", HeaderValue::from_static("provider-openai"));
                for (name, value) in self.client.default_headers() {
                    let header_name = HeaderName::from_bytes(name.as_bytes())
                        .map_err(|e| Error::Internal(format!("invalid header name: {e}")))?;
                    request.headers_mut().insert(
                        header_name,
                        HeaderValue::from_str(value).map_err(|e| Error::Internal(e.to_string()))?,
                    );
                }

                let (mut socket, _) = connect_async(request)
                    .await
                    .map_err(|e| Error::Inference(e.to_string()))?;

                let payload = serde_json::to_string(&WebSocketResponseCreateRequest {
                    message_type: "response.create",
                    request: &body,
                })?;

                socket
                    .send(WsMessage::Text(payload.into()))
                    .await
                    .map_err(|e| Error::Inference(e.to_string()))?;

                Ok(stream_responses_from_websocket(socket))
            }
        }
    }

    /// Retrieves an existing response by identifier.
    pub async fn retrieve(
        &self,
        response_id: &str,
        request: &ResponseRetrieveRequest,
    ) -> Result<ResponseObject, Error> {
        let response = self
            .client
            .apply_default_headers(
                self.client
                    .http()
                    .get(self.response_url(response_id)?)
                    .header("Authorization", self.client.auth_header()),
            )
            .query(&self.response_retrieve_query(request, false))
            .send()
            .await
            .map_err(|e| Error::Inference(e.to_string()))?;

        let response = ensure_success(response).await?;
        Ok(parse_response_object_value(json_value(response).await?))
    }

    /// Retrieves an existing response and streams follow-up events over SSE.
    pub async fn stream_retrieve(
        &self,
        response_id: &str,
        request: &ResponseRetrieveRequest,
    ) -> Result<ResponseStream, Error> {
        let response = self
            .client
            .apply_default_headers(
                self.client
                    .http()
                    .get(self.response_url(response_id)?)
                    .header("Authorization", self.client.auth_header())
                    .header("accept", "text/event-stream"),
            )
            .query(&self.response_retrieve_query(request, true))
            .send()
            .await
            .map_err(|e| Error::Inference(e.to_string()))?;

        let response = ensure_success(response).await?;
        Ok(sse_stream_from_response(response))
    }

    /// Cancels a background response.
    pub async fn cancel(&self, response_id: &str) -> Result<ResponseObject, Error> {
        let url = self
            .client
            .endpoint_joined_url(&["responses", response_id, "cancel"])?;
        let response = self
            .client
            .apply_default_headers(
                self.client
                    .http()
                    .post(url)
                    .header("Authorization", self.client.auth_header()),
            )
            .send()
            .await
            .map_err(|e| Error::Inference(e.to_string()))?;

        let response = ensure_success(response).await?;
        Ok(parse_response_object_value(json_value(response).await?))
    }

    /// Deletes a stored response.
    pub async fn delete(&self, response_id: &str) -> Result<(), Error> {
        let response = self
            .client
            .apply_default_headers(
                self.client
                    .http()
                    .delete(self.response_url(response_id)?)
                    .header("Authorization", self.client.auth_header()),
            )
            .send()
            .await
            .map_err(|e| Error::Inference(e.to_string()))?;

        ensure_success(response).await?;
        Ok(())
    }

    /// Compacts a stored response into a shorter artifact.
    pub async fn compact(
        &self,
        request: &ResponseCompactRequest,
    ) -> Result<ResponseCompaction, Error> {
        let response = self
            .client
            .apply_default_headers(
                self.client
                    .http()
                    .post(self.responses_compact_url())
                    .header("Authorization", self.client.auth_header()),
            )
            .json(request)
            .send()
            .await
            .map_err(|e| Error::Inference(e.to_string()))?;

        let response = ensure_success(response).await?;
        Ok(parse_response_compaction_value(json_value(response).await?))
    }

    /// Lists input items associated with a stored response.
    pub async fn list_input_items(
        &self,
        response_id: &str,
        request: &ResponseItemListRequest,
    ) -> Result<ResponseItemPage, Error> {
        let response = self
            .client
            .apply_default_headers(
                self.client
                    .http()
                    .get(self.response_input_items_url(response_id)?)
                    .header("Authorization", self.client.auth_header()),
            )
            .query(&self.response_item_list_query(request))
            .send()
            .await
            .map_err(|e| Error::Inference(e.to_string()))?;

        let response = ensure_success(response).await?;
        Ok(parse_response_item_page_value(json_value(response).await?))
    }

    /// Counts input tokens for a hypothetical response request.
    pub async fn count_input_tokens(
        &self,
        request: &ResponseInputTokenCountRequest,
    ) -> Result<ResponseInputTokenCount, Error> {
        let mut body = request.clone();
        self.normalize_input_token_count_request(&mut body);

        let response = self
            .client
            .apply_default_headers(
                self.client
                    .http()
                    .post(self.responses_input_tokens_url())
                    .header("Authorization", self.client.auth_header()),
            )
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Inference(e.to_string()))?;

        let response = ensure_success(response).await?;
        Ok(parse_response_input_token_count_value(
            json_value(response).await?,
        ))
    }
}

fn stream_responses_from_websocket<S>(
    socket: tokio_tungstenite::WebSocketStream<S>,
) -> ResponseStream
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let s = async_stream::stream! {
        let mut socket = socket;
        let mut saw_terminal = false;

        while let Some(message) = socket.next().await {
            let message = match message {
                Ok(message) => message,
                Err(err) => {
                    yield Err(Error::Inference(err.to_string()));
                    break;
                }
            };

            let raw = match message {
                WsMessage::Text(text) => serde_json::from_str::<serde_json::Value>(text.as_ref()),
                WsMessage::Binary(bytes) => serde_json::from_slice::<serde_json::Value>(&bytes),
                WsMessage::Close(_) => break,
                WsMessage::Ping(_) | WsMessage::Pong(_) | WsMessage::Frame(_) => continue,
            };

            let raw = match raw {
                Ok(raw) => raw,
                Err(err) => {
                    yield Err(Error::Json(err));
                    break;
                }
            };

            match parse_response_stream_event(raw) {
                Ok(Some(mapped)) => {
                    let is_terminal = matches!(
                        mapped.event,
                        crate::responses::ResponseEvent::ResponseCompleted { .. }
                            | crate::responses::ResponseEvent::ResponseIncomplete { .. }
                            | crate::responses::ResponseEvent::ResponseFailed { .. }
                    );
                    if is_terminal {
                        saw_terminal = true;
                    }
                    yield Ok(mapped);
                    if is_terminal {
                        break;
                    }
                }
                Ok(None) => {}
                Err(err) => {
                    yield Err(err);
                    break;
                }
            }
        }

        if !saw_terminal {
            yield Err(Error::Inference("websocket closed before terminal response event".into()));
        }
    };

    Box::pin(s)
}
