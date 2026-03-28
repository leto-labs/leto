//! Shared-provider adapter built on top of the supported OpenAI-compatible API
//! surfaces.
//!
//! Official references:
//! - Responses create: <https://developers.openai.com/api/reference/resources/responses/methods/create>
//! - Responses streaming events: <https://developers.openai.com/api/reference/resources/responses/streaming-events>
//! - Chat Completions create: <https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create>
//! - Chat Completions streaming events: <https://developers.openai.com/api/reference/resources/chat/subresources/completions/streaming-events>

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use async_stream::stream;
use futures::{StreamExt, future::BoxFuture};
use provider::{
    Block, BlockDelta, BlockKind, CredentialFailure, CredentialPool as SharedCredentialPool,
    Error as ProviderError, Event, EventStream, FinishReason, Message, MessageRole, Provider,
    ProviderCapabilities, ProviderInfo, Request, StreamGranularity, ToolChoice, Usage,
};

use crate::chat_completions::{
    ChatCompletionContentPart, ChatCompletionFunctionCall, ChatCompletionImageUrlPart,
    ChatCompletionMessage, ChatCompletionMessageContent, ChatCompletionRequest, ChatCompletionRole,
    ChatCompletionTool, ChatCompletionToolCall,
};
use crate::oauth::{
    config_with_resolved_credential, mark_credential_error, mark_credential_ok, resolve_credential,
};
use crate::responses::{
    ResponseEvent, ResponseInputContentPart, ResponseInputItem, ResponseInputRole,
    ResponseOutputContentPart, ResponseOutputItem, ResponseReasoningConfig,
    ResponseReasoningSummaryPart, ResponseRequest, ResponseStreamTransport, ResponseTextConfig,
    ResponseTool, ResponseToolCallItem,
};
use crate::{Client, Config, Error, OpenAiApiSurface};

const DEFAULT_CODEX_INSTRUCTIONS: &str = "You are a concise and helpful coding assistant.";

/// Shared [`provider::Provider`] adapter backed by the resolved
/// OpenAI-compatible API surface.
#[derive(Clone)]
pub struct OpenAiProvider {
    client: Client,
    transport: ResponseStreamTransport,
    credential_pool: Option<Arc<SharedCredentialPool>>,
}

impl OpenAiProvider {
    /// Creates an adapter from an OpenAI configuration.
    pub fn new(config: Config) -> Self {
        Self::from_client(Client::new(config))
    }

    /// Wraps an existing protocol-native [`Client`].
    pub fn from_client(client: Client) -> Self {
        Self {
            client,
            transport: ResponseStreamTransport::Sse,
            credential_pool: None,
        }
    }

    /// Creates an adapter that resolves credentials per request using the
    /// shared standalone credential pool.
    pub fn from_pool(config: Config, pool: Arc<SharedCredentialPool>) -> Self {
        Self {
            client: Client::new(config),
            transport: ResponseStreamTransport::Sse,
            credential_pool: Some(pool),
        }
    }

    /// Selects the streaming transport used for Responses streaming.
    pub fn with_transport(mut self, transport: ResponseStreamTransport) -> Self {
        self.transport = transport;
        self
    }

    fn session_id_for_request(request: &Request) -> Option<&str> {
        request
            .options
            .metadata
            .get("session_id")
            .and_then(|value| value.as_str())
    }

    pub(crate) async fn stream_for_surface<'a>(
        client: &Client,
        transport: ResponseStreamTransport,
        request: &'a Request,
    ) -> Result<EventStream<'a>, ProviderError> {
        match client.config().resolved_api_surface().ok_or_else(|| {
            ProviderError::Configuration("OpenAI config resolved no supported API surface".into())
        })? {
            OpenAiApiSurface::Responses => {
                Self::stream_responses_with(client, transport, request).await
            }
            OpenAiApiSurface::ChatCompletions => {
                Self::stream_chat_completions_with(client, request).await
            }
        }
    }

    fn map_responses_request(
        request: &Request,
        require_instructions: bool,
    ) -> Result<ResponseRequest, ProviderError> {
        let mut input = Vec::new();
        for message in &request.messages {
            if matches!(message.role, MessageRole::System | MessageRole::Developer) {
                continue;
            }
            input.extend(map_responses_message(message)?);
        }

        let tools = request
            .tools
            .iter()
            .map(|tool| {
                ResponseTool::function(
                    tool.name.clone(),
                    tool.description.clone().unwrap_or_default(),
                    tool.input_schema.clone(),
                )
            })
            .collect();

        let tool_choice = request
            .options
            .tool_choice
            .as_ref()
            .map(map_tool_choice_value);

        let reasoning =
            request
                .options
                .reasoning
                .as_ref()
                .map(|reasoning| ResponseReasoningConfig {
                    effort: reasoning.effort.clone(),
                    summary: reasoning.summary.clone(),
                });

        let instructions = collect_responses_instructions(&request.messages)
            .or_else(|| require_instructions.then_some(DEFAULT_CODEX_INSTRUCTIONS.to_owned()));

        Ok(ResponseRequest {
            model: request.model.clone(),
            instructions,
            input,
            tools,
            tool_choice,
            parallel_tool_calls: request.options.parallel_tool_calls,
            reasoning,
            text: Some(ResponseTextConfig {
                format: None,
                verbosity: None,
            }),
            max_output_tokens: request.options.max_output_tokens,
            temperature: request.options.temperature,
            top_p: request.options.top_p,
            metadata: request.options.metadata.clone(),
            ..ResponseRequest::default()
        })
    }

    fn map_chat_completions_request(
        request: &Request,
    ) -> Result<ChatCompletionRequest, ProviderError> {
        let mut messages = Vec::new();
        for message in &request.messages {
            messages.push(map_chat_completions_message(message)?);
        }

        let tools = request
            .tools
            .iter()
            .map(|tool| {
                ChatCompletionTool::function(
                    tool.name.clone(),
                    tool.description.clone().unwrap_or_default(),
                    tool.input_schema.clone(),
                )
            })
            .collect();

        Ok(ChatCompletionRequest {
            model: request.model.clone(),
            messages,
            tools,
            tool_choice: request
                .options
                .tool_choice
                .as_ref()
                .map(map_tool_choice_value),
            parallel_tool_calls: request.options.parallel_tool_calls,
            reasoning_effort: request
                .options
                .reasoning
                .as_ref()
                .and_then(|reasoning| reasoning.effort.clone()),
            max_tokens: request.options.max_output_tokens,
            temperature: request.options.temperature.map(|value| value as f32),
            top_p: request.options.top_p.map(|value| value as f32),
            ..ChatCompletionRequest::default()
        })
    }

    async fn stream_responses_with<'a>(
        client: &Client,
        transport: ResponseStreamTransport,
        request: &'a Request,
    ) -> Result<EventStream<'a>, ProviderError> {
        let require_instructions = client.config().base_url.contains("backend-api/codex");
        let mapped_request = Self::map_responses_request(request, require_instructions)?;
        let response_stream = client
            .responses()
            .stream(&mapped_request, transport)
            .await
            .map_err(map_openai_error)?;

        let initial_model = request
            .model
            .clone()
            .or_else(|| Some(client.config().default_model.clone()));

        let output_stream = stream! {
            let mut stream = response_stream;
            let mut started = HashMap::<String, Block>::new();
            let mut saw_delta = HashSet::<String>::new();

            yield Ok(Event::ResponseStart {
                response_id: None,
                model: initial_model,
            });

            while let Some(event) = stream.next().await {
                let event = match event {
                    Ok(event) => event.event,
                    Err(err) => {
                        yield Err(map_openai_error(err));
                        break;
                    }
                };

                match event {
                    ResponseEvent::ResponseCreated { .. } => {}
                    ResponseEvent::ResponseQueued { .. }
                    | ResponseEvent::ResponseInProgress { .. } => {}
                    ResponseEvent::ResponseCompleted { response } => {
                        if let Some(usage) = map_openai_usage(response.usage.as_ref()) {
                            yield Ok(Event::Usage { usage });
                        }
                        yield Ok(Event::Completed {
                            response_id: response.id.clone(),
                            finish_reason: Some(FinishReason::Stop),
                        });
                    }
                    ResponseEvent::ResponseIncomplete { response } => {
                        if let Some(usage) = map_openai_usage(response.usage.as_ref()) {
                            yield Ok(Event::Usage { usage });
                        }
                        let finish_reason = response
                            .incomplete_details
                            .as_ref()
                            .and_then(|details| details.reason.clone())
                            .map(map_openai_finish_reason)
                            .or(Some(FinishReason::Incomplete));
                        yield Ok(Event::Completed {
                            response_id: response.id.clone(),
                            finish_reason,
                        });
                    }
                    ResponseEvent::ResponseFailed { response } => {
                        if let Some(usage) = map_openai_usage(response.usage.as_ref()) {
                            yield Ok(Event::Usage { usage });
                        }
                        yield Ok(Event::Completed {
                            response_id: response.id.clone(),
                            finish_reason: Some(FinishReason::Error),
                        });
                    }
                    ResponseEvent::Error { error, .. } => {
                        let message = error.message.unwrap_or_else(|| "OpenAI stream error".into());
                        yield Err(ProviderError::Remote(message));
                        break;
                    }
                    ResponseEvent::OutputItemAdded { output_index, item } => {
                        if let Some(block) = map_openai_output_item_start(output_index, &item)
                            && let Some(start_event) = start_block_if_needed(&mut started, block) {
                            yield Ok(start_event);
                        }
                    }
                    ResponseEvent::OutputItemDone { output_index, item } => {
                        if let Some(block) = map_openai_output_item_start(output_index, &item) {
                            let block_id = block.id.clone();
                            if let Some(start_event) = start_block_if_needed(&mut started, block) {
                                yield Ok(start_event);
                            }
                            if let Some(final_delta) = map_openai_item_done_delta(&item, saw_delta.contains(&block_id)) {
                                saw_delta.insert(block_id.clone());
                                yield Ok(Event::BlockDelta { id: block_id.clone(), delta: final_delta });
                            }
                            if started.remove(&block_id).is_some() {
                                yield Ok(Event::BlockStop { id: block_id });
                            }
                        }
                    }
                    ResponseEvent::ContentPartAdded { item_id, output_index, content_index, part } => {
                        let block = map_openai_content_part_start(&item_id, output_index, content_index, &part);
                        if let Some(start_event) = start_block_if_needed(&mut started, block) {
                            yield Ok(start_event);
                        }
                    }
                    ResponseEvent::ContentPartDone { item_id, content_index, part, .. } => {
                        let block_id = openai_content_block_id(&item_id, content_index, &part);
                        if let Some(delta) = map_openai_content_done_delta(&part, saw_delta.contains(&block_id)) {
                            saw_delta.insert(block_id.clone());
                            yield Ok(Event::BlockDelta { id: block_id.clone(), delta });
                        }
                        if started.remove(&block_id).is_some() {
                            yield Ok(Event::BlockStop { id: block_id });
                        }
                    }
                    ResponseEvent::OutputTextDelta { item_id, content_index, delta, .. } => {
                        let block_id = openai_text_block_id(&item_id, content_index);
                        if let Some(start_event) = start_block_if_needed(
                            &mut started,
                            Block {
                                id: block_id.clone(),
                                output_index: 0,
                                kind: BlockKind::Text,
                                item_id: Some(item_id.clone()),
                            },
                        ) {
                            yield Ok(start_event);
                        }
                        saw_delta.insert(block_id.clone());
                        yield Ok(Event::BlockDelta { id: block_id, delta: BlockDelta::Text { text: delta } });
                    }
                    ResponseEvent::OutputTextDone { item_id, content_index, text, .. } => {
                        let block_id = openai_text_block_id(&item_id, content_index);
                        if !saw_delta.contains(&block_id) && !text.is_empty() {
                            saw_delta.insert(block_id.clone());
                            yield Ok(Event::BlockDelta { id: block_id.clone(), delta: BlockDelta::Text { text } });
                        }
                        if started.remove(&block_id).is_some() {
                            yield Ok(Event::BlockStop { id: block_id });
                        }
                    }
                    ResponseEvent::RefusalDelta { item_id, content_index, delta, .. } => {
                        let block_id = openai_refusal_block_id(&item_id, content_index);
                        if let Some(start_event) = start_block_if_needed(
                            &mut started,
                            Block {
                                id: block_id.clone(),
                                output_index: 0,
                                kind: BlockKind::Refusal,
                                item_id: Some(item_id.clone()),
                            },
                        ) {
                            yield Ok(start_event);
                        }
                        saw_delta.insert(block_id.clone());
                        yield Ok(Event::BlockDelta { id: block_id, delta: BlockDelta::Refusal { text: delta } });
                    }
                    ResponseEvent::RefusalDone { item_id, content_index, refusal, .. } => {
                        let block_id = openai_refusal_block_id(&item_id, content_index);
                        if !saw_delta.contains(&block_id) && !refusal.is_empty() {
                            saw_delta.insert(block_id.clone());
                            yield Ok(Event::BlockDelta { id: block_id.clone(), delta: BlockDelta::Refusal { text: refusal } });
                        }
                        if started.remove(&block_id).is_some() {
                            yield Ok(Event::BlockStop { id: block_id });
                        }
                    }
                    ResponseEvent::FunctionCallArgumentsDelta { item_id, output_index, delta } => {
                        let block_id = openai_tool_call_block_id(&item_id);
                        if let Some(start_event) = start_block_if_needed(
                            &mut started,
                            Block {
                                id: block_id.clone(),
                                output_index,
                                kind: BlockKind::ToolCall {
                                    name: None,
                                    call_id: None,
                                },
                                item_id: Some(item_id.clone()),
                            },
                        ) {
                            yield Ok(start_event);
                        }
                        saw_delta.insert(block_id.clone());
                        yield Ok(Event::BlockDelta { id: block_id, delta: BlockDelta::Json { partial_json: delta } });
                    }
                    ResponseEvent::FunctionCallArgumentsDone { item_id, arguments, .. } => {
                        let block_id = openai_tool_call_block_id(&item_id);
                        if !saw_delta.contains(&block_id) && !arguments.is_empty() {
                            saw_delta.insert(block_id.clone());
                            yield Ok(Event::BlockDelta { id: block_id.clone(), delta: BlockDelta::Json { partial_json: arguments } });
                        }
                        if started.remove(&block_id).is_some() {
                            yield Ok(Event::BlockStop { id: block_id });
                        }
                    }
                    ResponseEvent::ReasoningSummaryPartAdded { item_id, output_index, summary_index, part } => {
                        let block = Block {
                            id: openai_reasoning_block_id(&item_id, summary_index),
                            output_index,
                            kind: BlockKind::Reasoning,
                            item_id: Some(item_id.clone()),
                        };
                        let block_id = block.id.clone();
                        if let Some(start_event) = start_block_if_needed(&mut started, block) {
                            yield Ok(start_event);
                        }
                        if let Some(delta) = map_openai_reasoning_part_delta(&part) {
                            saw_delta.insert(block_id.clone());
                            yield Ok(Event::BlockDelta { id: block_id, delta });
                        }
                    }
                    ResponseEvent::ReasoningSummaryTextDelta { item_id, summary_index, delta, .. } => {
                        let block_id = openai_reasoning_block_id(&item_id, summary_index);
                        if let Some(start_event) = start_block_if_needed(
                            &mut started,
                            Block {
                                id: block_id.clone(),
                                output_index: 0,
                                kind: BlockKind::Reasoning,
                                item_id: Some(item_id.clone()),
                            },
                        ) {
                            yield Ok(start_event);
                        }
                        saw_delta.insert(block_id.clone());
                        yield Ok(Event::BlockDelta { id: block_id, delta: BlockDelta::Reasoning { text: delta } });
                    }
                    ResponseEvent::ReasoningSummaryTextDone { item_id, summary_index, text, .. } => {
                        let block_id = openai_reasoning_block_id(&item_id, summary_index);
                        if !saw_delta.contains(&block_id) && !text.is_empty() {
                            saw_delta.insert(block_id.clone());
                            yield Ok(Event::BlockDelta { id: block_id.clone(), delta: BlockDelta::Reasoning { text } });
                        }
                        if started.remove(&block_id).is_some() {
                            yield Ok(Event::BlockStop { id: block_id });
                        }
                    }
                    ResponseEvent::ReasoningSummaryPartDone { item_id, summary_index, part, .. } => {
                        let block_id = openai_reasoning_block_id(&item_id, summary_index);
                        if let Some(delta) = map_openai_reasoning_part_delta(&part)
                            && !saw_delta.contains(&block_id) {
                            saw_delta.insert(block_id.clone());
                            yield Ok(Event::BlockDelta { id: block_id.clone(), delta });
                        }
                        if started.remove(&block_id).is_some() {
                            yield Ok(Event::BlockStop { id: block_id });
                        }
                    }
                    ResponseEvent::WebSearchCallSearching { item_id, output_index }
                    | ResponseEvent::FileSearchSearching { item_id, output_index } => {
                        let block_id = openai_tool_call_block_id(&item_id);
                        if !started.contains_key(&block_id) {
                            let block = Block {
                                id: block_id.clone(),
                                output_index,
                                kind: BlockKind::Unknown { kind: "hosted_tool".into() },
                                item_id: Some(item_id),
                            };
                            if let Some(start_event) = start_block_if_needed(&mut started, block) {
                                yield Ok(start_event);
                            }
                        }
                    }
                    ResponseEvent::CodeInterpreterCodeDelta { item_id, output_index, delta } => {
                        let block_id = openai_tool_call_block_id(&item_id);
                        if !started.contains_key(&block_id) {
                            let block = Block {
                                id: block_id.clone(),
                                output_index,
                                kind: BlockKind::Unknown { kind: "code_interpreter".into() },
                                item_id: Some(item_id.clone()),
                            };
                            if let Some(start_event) = start_block_if_needed(&mut started, block) {
                                yield Ok(start_event);
                            }
                        }
                        saw_delta.insert(block_id.clone());
                        yield Ok(Event::BlockDelta { id: block_id, delta: BlockDelta::Unknown { raw: serde_json::json!({ "code": delta }) } });
                    }
                    ResponseEvent::CodeInterpreterCodeDone { item_id, code, .. } => {
                        let block_id = openai_tool_call_block_id(&item_id);
                        if !saw_delta.contains(&block_id) && !code.is_empty() {
                            saw_delta.insert(block_id.clone());
                            yield Ok(Event::BlockDelta { id: block_id.clone(), delta: BlockDelta::Unknown { raw: serde_json::json!({ "code": code }) } });
                        }
                        if started.remove(&block_id).is_some() {
                            yield Ok(Event::BlockStop { id: block_id });
                        }
                    }
                    ResponseEvent::OutputAudioDelta { .. }
                    | ResponseEvent::OutputAudioDone
                    | ResponseEvent::OutputAudioTranscriptDelta { .. }
                    | ResponseEvent::OutputAudioTranscriptDone
                    | ResponseEvent::Unknown { .. } => {}
                }
            }
        };

        Ok(Box::pin(output_stream) as EventStream<'a>)
    }

    async fn stream_chat_completions_with<'a>(
        client: &Client,
        request: &'a Request,
    ) -> Result<EventStream<'a>, ProviderError> {
        let mapped_request = Self::map_chat_completions_request(request)?;
        let completion_stream = client
            .chat_completions()
            .stream(&mapped_request)
            .await
            .map_err(map_openai_error)?;

        let initial_model = request
            .model
            .clone()
            .or_else(|| Some(client.config().default_model.clone()));

        let output_stream = stream! {
            let mut stream = completion_stream;
            let mut started = HashMap::<String, Block>::new();
            let mut finish_reason = None;
            let mut response_id = None;
            let mut tool_states = HashMap::<(u32, u32), ChatToolCallState>::new();

            yield Ok(Event::ResponseStart {
                response_id: None,
                model: initial_model,
            });

            while let Some(chunk) = stream.next().await {
                let chunk = match chunk {
                    Ok(chunk) => chunk,
                    Err(err) => {
                        yield Err(map_openai_error(err));
                        break;
                    }
                };

                if response_id.is_none() {
                    response_id = chunk.id.clone();
                }

                for choice in chunk.choices {
                    if let Some(text) = choice.delta.content {
                        let block_id = openai_chat_text_block_id(choice.index);
                        if let Some(start_event) = start_block_if_needed(
                            &mut started,
                            Block {
                                id: block_id.clone(),
                                output_index: choice.index,
                                kind: BlockKind::Text,
                                item_id: None,
                            },
                        ) {
                            yield Ok(start_event);
                        }
                        yield Ok(Event::BlockDelta {
                            id: block_id,
                            delta: BlockDelta::Text { text },
                        });
                    }

                    for tool_call in choice.delta.tool_calls {
                        let tool_index = tool_call.index.unwrap_or(0);
                        let state = tool_states
                            .entry((choice.index, tool_index))
                            .or_default();

                        if let Some(id) = tool_call.id {
                            state.call_id = Some(id);
                        }

                        if let Some(function) = tool_call.function {
                            if let Some(name) = function.name {
                                state.name = Some(name);
                            }

                            let block_id = openai_chat_tool_call_block_id(choice.index, tool_index);
                            if let Some(start_event) = start_block_if_needed(
                                &mut started,
                                Block {
                                    id: block_id.clone(),
                                    output_index: choice.index,
                                    kind: BlockKind::ToolCall {
                                        name: state.name.clone(),
                                        call_id: state.call_id.clone(),
                                    },
                                    item_id: state.call_id.clone(),
                                },
                            ) {
                                yield Ok(start_event);
                            }

                            if let Some(arguments) = function.arguments.filter(|arguments| !arguments.is_empty()) {
                                yield Ok(Event::BlockDelta {
                                    id: block_id,
                                    delta: BlockDelta::Json {
                                        partial_json: arguments,
                                    },
                                });
                            }
                        } else {
                            let block_id = openai_chat_tool_call_block_id(choice.index, tool_index);
                            if let Some(start_event) = start_block_if_needed(
                                &mut started,
                                Block {
                                    id: block_id,
                                    output_index: choice.index,
                                    kind: BlockKind::ToolCall {
                                        name: state.name.clone(),
                                        call_id: state.call_id.clone(),
                                    },
                                    item_id: state.call_id.clone(),
                                },
                            ) {
                                yield Ok(start_event);
                            }
                        }
                    }

                    if let Some(choice_finish_reason) = choice.finish_reason {
                        if finish_reason.is_none() {
                            finish_reason = Some(map_openai_finish_reason(choice_finish_reason));
                        }

                        for block_id in openai_chat_choice_block_ids(&started, choice.index) {
                            started.remove(&block_id);
                            yield Ok(Event::BlockStop { id: block_id });
                        }
                    }
                }

                if let Some(usage) = map_openai_usage(chunk.usage.as_ref()) {
                    yield Ok(Event::Usage { usage });
                }
            }

            for block_id in openai_chat_all_block_ids(&started) {
                yield Ok(Event::BlockStop { id: block_id });
            }

            yield Ok(Event::Completed {
                response_id,
                finish_reason,
            });
        };

        Ok(Box::pin(output_stream) as EventStream<'a>)
    }
}

impl Provider for OpenAiProvider {
    fn stream<'a>(
        &'a self,
        request: &'a Request,
    ) -> BoxFuture<'a, Result<EventStream<'a>, ProviderError>> {
        Box::pin(async move {
            if let Some(credential_pool) = &self.credential_pool {
                let resolved = resolve_credential(
                    credential_pool.as_ref(),
                    &self.client.config().name,
                    Self::session_id_for_request(request),
                )
                .await
                .map_err(map_openai_error)?;
                let credential_id = resolved.credential_id.clone();
                let client = Client::with_http_client(
                    config_with_resolved_credential(self.client.config().clone(), &resolved)
                        .map_err(map_openai_error)?,
                    self.client.http_client(),
                );
                let provider_name = self.client.config().name.clone();
                let credential_pool = credential_pool.clone();
                let mut stream = Self::stream_for_surface(&client, self.transport, request).await?;
                return Ok(Box::pin(stream! {
                    let mut saw_completed = false;
                    while let Some(event) = stream.next().await {
                        match event {
                            Ok(event) => {
                                if matches!(event, Event::Completed { .. }) {
                                    saw_completed = true;
                                }
                                yield Ok(event);
                            }
                            Err(err) => {
                                mark_credential_error(
                                    credential_pool.as_ref(),
                                    &provider_name,
                                    &credential_id,
                                    CredentialFailure::new(err.to_string()),
                                ).await;
                                yield Err(err);
                                return;
                            }
                        }
                    }
                    if saw_completed {
                        mark_credential_ok(credential_pool.as_ref(), &provider_name, &credential_id).await;
                    } else {
                        mark_credential_error(
                            credential_pool.as_ref(),
                            &provider_name,
                            &credential_id,
                            CredentialFailure::new("stream closed before terminal event"),
                        ).await;
                    }
                }) as EventStream<'a>);
            }
            Self::stream_for_surface(&self.client, self.transport, request).await
        })
    }

    fn info(&self) -> ProviderInfo {
        let surface = self
            .client
            .config()
            .resolved_api_surface()
            .or_else(|| self.client.config().supported_api_surfaces.first().copied());

        ProviderInfo {
            name: self.client.config().name.clone(),
            default_model_id: Some(self.client.config().default_model.clone()),
            capabilities: surface
                .map(capabilities_for_surface)
                .unwrap_or_else(ProviderCapabilities::text_only),
            models: self.client.config().models.to_vec(),
        }
    }
}

fn map_openai_error(err: Error) -> ProviderError {
    match err {
        Error::Auth(message) => ProviderError::Remote(message),
        Error::Inference(message) => ProviderError::Inference(message),
        Error::Internal(message) => ProviderError::Configuration(message),
        Error::Json(err) => ProviderError::Json(err),
    }
}

fn collect_responses_instructions(messages: &[Message]) -> Option<String> {
    let instructions: Vec<String> = messages
        .iter()
        .filter(|message| matches!(message.role, MessageRole::System | MessageRole::Developer))
        .map(Message::plain_text_lossy)
        .filter(|text| !text.trim().is_empty())
        .collect();

    if instructions.is_empty() {
        None
    } else {
        Some(instructions.join("\n\n"))
    }
}

fn map_responses_message(message: &Message) -> Result<Vec<ResponseInputItem>, ProviderError> {
    let has_tool_result = message
        .content
        .iter()
        .any(|block| matches!(block, provider::ContentBlock::ToolResult { .. }));
    let has_tool_call = message
        .content
        .iter()
        .any(|block| matches!(block, provider::ContentBlock::ToolCall { .. }));

    if has_tool_result {
        if message.content.len() != 1 {
            return Err(ProviderError::Unsupported(
                "OpenAI adapter requires tool-result messages to contain a single tool_result block"
                    .into(),
            ));
        }

        if let provider::ContentBlock::ToolResult {
            call_id, output, ..
        } = &message.content[0]
        {
            return Ok(vec![ResponseInputItem::FunctionCallOutput {
                call_id: call_id.clone(),
                output: json_value_to_string(output),
            }]);
        }
    }

    if has_tool_call {
        let mut items = Vec::new();
        for block in &message.content {
            match block {
                provider::ContentBlock::ToolCall { id, name, input } => {
                    items.push(ResponseInputItem::FunctionCall {
                        id: id.clone(),
                        call_id: id.clone(),
                        name: name.clone(),
                        arguments: serde_json::to_string(input)?,
                    });
                }
                _ => {
                    return Err(ProviderError::Unsupported(
                        "OpenAI adapter requires tool-call messages to contain only tool_call blocks"
                            .into(),
                    ));
                }
            }
        }
        return Ok(items);
    }

    let mut content = Vec::new();
    for block in &message.content {
        match block {
            provider::ContentBlock::Text { text } => {
                content.push(ResponseInputContentPart::input_text(text.clone()));
            }
            provider::ContentBlock::ImageUrl { url } => {
                content.push(ResponseInputContentPart::input_image(url.clone()));
            }
            provider::ContentBlock::Reasoning { text }
            | provider::ContentBlock::Refusal { text } => {
                content.push(ResponseInputContentPart::input_text(text.clone()));
            }
            provider::ContentBlock::ToolCall { .. } | provider::ContentBlock::ToolResult { .. } => {
                return Err(ProviderError::Unsupported(
                    "OpenAI adapter could not map tool blocks into a standard message".into(),
                ));
            }
        }
    }

    Ok(vec![ResponseInputItem::message(
        map_role(message.role),
        content,
    )])
}

fn map_chat_completions_message(message: &Message) -> Result<ChatCompletionMessage, ProviderError> {
    let has_tool_result = message
        .content
        .iter()
        .any(|block| matches!(block, provider::ContentBlock::ToolResult { .. }));
    if has_tool_result {
        if message.content.len() != 1 {
            return Err(ProviderError::Unsupported(
                "OpenAI chat-completions adapter requires tool-result messages to contain a single tool_result block"
                    .into(),
            ));
        }

        if let provider::ContentBlock::ToolResult {
            call_id, output, ..
        } = &message.content[0]
        {
            return Ok(ChatCompletionMessage {
                role: ChatCompletionRole::Tool,
                content: Some(ChatCompletionMessageContent::Text(json_value_to_string(
                    output,
                ))),
                name: None,
                tool_calls: Vec::new(),
                tool_call_id: Some(call_id.clone()),
                extra: Default::default(),
            });
        }
    }

    let mut tool_calls = Vec::new();
    let mut content_parts = Vec::new();

    for block in &message.content {
        match block {
            provider::ContentBlock::Text { text }
            | provider::ContentBlock::Reasoning { text }
            | provider::ContentBlock::Refusal { text } => {
                content_parts.push(ChatCompletionContentPart::Text { text: text.clone() });
            }
            provider::ContentBlock::ImageUrl { url } => {
                content_parts.push(ChatCompletionContentPart::ImageUrl {
                    image_url: ChatCompletionImageUrlPart { url: url.clone() },
                });
            }
            provider::ContentBlock::ToolCall { id, name, input } => {
                tool_calls.push(ChatCompletionToolCall {
                    id: id.clone(),
                    call_type: "function".into(),
                    function: ChatCompletionFunctionCall {
                        name: name.clone(),
                        arguments: serde_json::to_string(input)?,
                    },
                });
            }
            provider::ContentBlock::ToolResult { .. } => {
                return Err(ProviderError::Unsupported(
                    "OpenAI chat-completions adapter could not mix tool_result blocks with standard message content".into(),
                ));
            }
        }
    }

    if !tool_calls.is_empty() && message.role != MessageRole::Assistant {
        return Err(ProviderError::Unsupported(
            "OpenAI chat-completions adapter requires tool_call blocks to appear on assistant messages".into(),
        ));
    }

    Ok(ChatCompletionMessage {
        role: map_chat_role(message.role),
        content: map_chat_content(content_parts),
        name: None,
        tool_calls,
        tool_call_id: None,
        extra: Default::default(),
    })
}

fn map_chat_content(
    content_parts: Vec<ChatCompletionContentPart>,
) -> Option<ChatCompletionMessageContent> {
    if content_parts.is_empty() {
        return None;
    }

    if content_parts.len() == 1
        && let ChatCompletionContentPart::Text { text } = &content_parts[0]
    {
        return Some(ChatCompletionMessageContent::Text(text.clone()));
    }

    Some(ChatCompletionMessageContent::Parts(content_parts))
}

fn map_role(role: MessageRole) -> ResponseInputRole {
    match role {
        MessageRole::System => ResponseInputRole::System,
        MessageRole::Developer => ResponseInputRole::Developer,
        MessageRole::User => ResponseInputRole::User,
        MessageRole::Assistant => ResponseInputRole::Assistant,
    }
}

fn map_chat_role(role: MessageRole) -> ChatCompletionRole {
    match role {
        MessageRole::System => ChatCompletionRole::System,
        MessageRole::Developer => ChatCompletionRole::Developer,
        MessageRole::User => ChatCompletionRole::User,
        MessageRole::Assistant => ChatCompletionRole::Assistant,
    }
}

fn map_tool_choice_value(choice: &ToolChoice) -> serde_json::Value {
    match choice {
        ToolChoice::Auto => serde_json::json!("auto"),
        ToolChoice::Required => serde_json::json!("required"),
        ToolChoice::Tool { name } => {
            serde_json::json!({ "type": "function", "name": name })
        }
        ToolChoice::None => serde_json::json!("none"),
    }
}

fn json_value_to_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(text) => text.clone(),
        _ => value.to_string(),
    }
}

fn map_openai_usage(usage: Option<&crate::TokenUsage>) -> Option<Usage> {
    usage.map(|usage| Usage {
        input_tokens: Some(usage.prompt),
        output_tokens: Some(usage.completion),
        total_tokens: Some(usage.total),
        cache_read_tokens: usage.cache_read,
        cache_write_tokens: usage.cache_write,
        reasoning_tokens: usage.reasoning,
    })
}

fn map_openai_finish_reason(reason: String) -> FinishReason {
    match reason.as_str() {
        "length" => FinishReason::MaxTokens,
        "max_output_tokens" | "max_tokens" => FinishReason::MaxTokens,
        "tool_calls" | "tool_call" => FinishReason::ToolCall,
        "stop_sequence" => FinishReason::StopSequence,
        "content_filter" | "content_filtered" => FinishReason::Refusal,
        "refusal" => FinishReason::Refusal,
        "pause_turn" => FinishReason::Pause,
        "completed" | "stop" => FinishReason::Stop,
        other => FinishReason::Unknown(other.to_owned()),
    }
}

fn map_openai_output_item_start(output_index: u32, item: &ResponseOutputItem) -> Option<Block> {
    match item {
        ResponseOutputItem::ToolCall(item) => Some(Block {
            id: openai_tool_call_block_id(item.id.as_deref().unwrap_or("tool_call")),
            output_index,
            kind: BlockKind::ToolCall {
                name: item.name.clone(),
                call_id: item.call_id.clone(),
            },
            item_id: item.id.clone(),
        }),
        ResponseOutputItem::Unknown(item) => Some(Block {
            id: format!("openai-unknown-{output_index}"),
            output_index,
            kind: BlockKind::Unknown {
                kind: "unknown".into(),
            },
            item_id: item
                .raw
                .get("id")
                .and_then(|id| id.as_str())
                .map(ToOwned::to_owned),
        }),
        ResponseOutputItem::Reasoning(_) => None,
        ResponseOutputItem::Message(_) => None,
    }
}

fn map_openai_item_done_delta(
    item: &ResponseOutputItem,
    already_streamed: bool,
) -> Option<BlockDelta> {
    match item {
        ResponseOutputItem::ToolCall(ResponseToolCallItem {
            arguments: Some(arguments),
            ..
        }) if !already_streamed && !arguments.is_empty() => Some(BlockDelta::Json {
            partial_json: arguments.clone(),
        }),
        ResponseOutputItem::Reasoning(item) if !already_streamed => item
            .summary
            .first()
            .and_then(map_openai_reasoning_part_delta),
        _ => None,
    }
}

fn map_openai_content_part_start(
    item_id: &str,
    output_index: u32,
    content_index: u32,
    part: &ResponseOutputContentPart,
) -> Block {
    let kind = match part {
        ResponseOutputContentPart::OutputText { .. } => BlockKind::Text,
        ResponseOutputContentPart::Refusal { .. } => BlockKind::Refusal,
        ResponseOutputContentPart::OutputAudio { .. } => BlockKind::Unknown {
            kind: "output_audio".into(),
        },
        ResponseOutputContentPart::Unknown { .. } => BlockKind::Unknown {
            kind: "unknown".into(),
        },
    };

    Block {
        id: openai_content_block_id(item_id, content_index, part),
        output_index,
        kind,
        item_id: Some(item_id.to_owned()),
    }
}

fn map_openai_content_done_delta(
    part: &ResponseOutputContentPart,
    already_streamed: bool,
) -> Option<BlockDelta> {
    if already_streamed {
        return None;
    }

    match part {
        ResponseOutputContentPart::OutputText { text, .. } if !text.is_empty() => {
            Some(BlockDelta::Text { text: text.clone() })
        }
        ResponseOutputContentPart::Refusal { refusal } if !refusal.is_empty() => {
            Some(BlockDelta::Refusal {
                text: refusal.clone(),
            })
        }
        ResponseOutputContentPart::OutputAudio { raw }
        | ResponseOutputContentPart::Unknown { raw } => {
            Some(BlockDelta::Unknown { raw: raw.clone() })
        }
        _ => None,
    }
}

fn map_openai_reasoning_part_delta(part: &ResponseReasoningSummaryPart) -> Option<BlockDelta> {
    match part {
        ResponseReasoningSummaryPart::SummaryText { text } if !text.is_empty() => {
            Some(BlockDelta::Reasoning { text: text.clone() })
        }
        ResponseReasoningSummaryPart::Unknown { raw } => {
            Some(BlockDelta::Unknown { raw: raw.clone() })
        }
        _ => None,
    }
}

fn remember_block(started: &mut HashMap<String, Block>, block: Block) {
    started.insert(block.id.clone(), block);
}

fn start_block_if_needed(started: &mut HashMap<String, Block>, block: Block) -> Option<Event> {
    if started.contains_key(&block.id) {
        None
    } else {
        remember_block(started, block.clone());
        Some(Event::BlockStart { block })
    }
}

fn openai_tool_call_block_id(item_id: &str) -> String {
    format!("openai-tool-call:{item_id}")
}

fn openai_text_block_id(item_id: &str, content_index: u32) -> String {
    format!("openai-text:{item_id}:{content_index}")
}

fn openai_refusal_block_id(item_id: &str, content_index: u32) -> String {
    format!("openai-refusal:{item_id}:{content_index}")
}

fn openai_reasoning_block_id(item_id: &str, summary_index: u32) -> String {
    format!("openai-reasoning:{item_id}:{summary_index}")
}

fn openai_chat_text_block_id(choice_index: u32) -> String {
    format!("openai-chat-text:{choice_index}")
}

fn openai_chat_tool_call_block_id(choice_index: u32, tool_index: u32) -> String {
    format!("openai-chat-tool-call:{choice_index}:{tool_index}")
}

fn openai_chat_choice_block_ids(
    started: &HashMap<String, Block>,
    choice_index: u32,
) -> Vec<String> {
    let mut ids = started
        .values()
        .filter(|block| block.output_index == choice_index)
        .map(|block| block.id.clone())
        .collect::<Vec<_>>();
    ids.sort();
    ids
}

fn openai_chat_all_block_ids(started: &HashMap<String, Block>) -> Vec<String> {
    let mut ids = started.keys().cloned().collect::<Vec<_>>();
    ids.sort();
    ids
}

fn capabilities_for_surface(surface: OpenAiApiSurface) -> ProviderCapabilities {
    match surface {
        OpenAiApiSurface::Responses => ProviderCapabilities {
            system_messages: true,
            developer_messages: true,
            input_text: true,
            input_image_urls: true,
            tool_calls: true,
            tool_results: true,
            reasoning_blocks: true,
            refusal_blocks: true,
            tool_call_argument_deltas: true,
            parallel_tool_calls: true,
            stream_granularity: StreamGranularity::Block,
        },
        OpenAiApiSurface::ChatCompletions => ProviderCapabilities {
            system_messages: true,
            developer_messages: true,
            input_text: true,
            input_image_urls: true,
            tool_calls: true,
            tool_results: true,
            reasoning_blocks: false,
            refusal_blocks: false,
            tool_call_argument_deltas: true,
            parallel_tool_calls: true,
            stream_granularity: StreamGranularity::Block,
        },
    }
}

fn openai_content_block_id(
    item_id: &str,
    content_index: u32,
    part: &ResponseOutputContentPart,
) -> String {
    match part {
        ResponseOutputContentPart::OutputText { .. } => {
            openai_text_block_id(item_id, content_index)
        }
        ResponseOutputContentPart::Refusal { .. } => {
            openai_refusal_block_id(item_id, content_index)
        }
        ResponseOutputContentPart::OutputAudio { .. } => {
            format!("openai-audio:{item_id}:{content_index}")
        }
        ResponseOutputContentPart::Unknown { .. } => {
            format!("openai-unknown:{item_id}:{content_index}")
        }
    }
}

#[derive(Debug, Clone, Default)]
struct ChatToolCallState {
    name: Option<String>,
    call_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_tool_result_messages_to_function_call_output() {
        let message = Message::new(
            MessageRole::User,
            vec![provider::ContentBlock::tool_result(
                "call-1",
                serde_json::json!({ "ok": true }),
            )],
        );

        let mapped = map_responses_message(&message).unwrap();
        assert!(matches!(
            mapped[0],
            ResponseInputItem::FunctionCallOutput { .. }
        ));
    }

    #[test]
    fn maps_chat_completions_assistant_tool_calls_with_text() {
        let message = Message::new(
            MessageRole::Assistant,
            vec![
                provider::ContentBlock::text("calling tool"),
                provider::ContentBlock::tool_call(
                    "call-1",
                    "lookup_weather",
                    serde_json::json!({ "city": "Paris" }),
                ),
            ],
        );

        let mapped = map_chat_completions_message(&message).unwrap();
        assert_eq!(mapped.role, ChatCompletionRole::Assistant);
        assert_eq!(mapped.tool_calls.len(), 1);
        assert!(matches!(
            mapped.content,
            Some(ChatCompletionMessageContent::Text(_))
        ));
    }

    #[test]
    fn maps_chat_completions_tool_result_messages() {
        let message = Message::new(
            MessageRole::User,
            vec![provider::ContentBlock::tool_result(
                "call-1",
                serde_json::json!({ "ok": true }),
            )],
        );

        let mapped = map_chat_completions_message(&message).unwrap();
        assert_eq!(mapped.role, ChatCompletionRole::Tool);
        assert_eq!(mapped.tool_call_id.as_deref(), Some("call-1"));
    }

    #[test]
    fn maps_text_and_image_message_to_openai_message_item() {
        let message = Message::new(
            MessageRole::User,
            vec![
                provider::ContentBlock::text("look"),
                provider::ContentBlock::image_url("https://example.com/cat.png"),
            ],
        );

        let mapped = map_responses_message(&message).unwrap();
        match &mapped[0] {
            ResponseInputItem::Message { content, .. } => {
                assert_eq!(content.len(), 2);
            }
            _ => panic!("expected message item"),
        }
    }

    #[test]
    fn reports_surface_specific_capabilities() {
        let responses = OpenAiProvider::new(
            Config::new("test").with_api_surface_mode(crate::OpenAiApiMode::Responses),
        );
        let chat = OpenAiProvider::new(
            crate::OpenAiConfigPreset::GEMINI
                .into_config("test")
                .with_api_surface_mode(crate::OpenAiApiMode::ChatCompletions),
        );

        assert!(responses.info().capabilities.reasoning_blocks);
        assert!(!chat.info().capabilities.reasoning_blocks);
        assert!(chat.info().capabilities.tool_call_argument_deltas);
    }

    #[test]
    fn info_falls_back_to_first_supported_surface_when_mode_is_unsupported() {
        let provider = OpenAiProvider::new(
            crate::OpenAiConfigPreset::GEMINI
                .into_config("test")
                .with_api_surface_mode(crate::OpenAiApiMode::Responses),
        );

        let info = provider.info();

        assert!(!info.capabilities.reasoning_blocks);
        assert!(info.capabilities.tool_call_argument_deltas);
    }

    #[test]
    fn info_reports_model_catalog_and_default_model() {
        let provider = OpenAiProvider::new(crate::OpenAiConfigPreset::OPENAI.into_config("test"));
        let info = provider.info();

        assert_eq!(info.name, "openai");
        assert_eq!(info.default_model_id.as_deref(), Some("gpt-4o-mini"));
        assert_eq!(
            info.default_model().map(|model| model.id.as_ref()),
            Some("gpt-4o-mini")
        );

        let gpt_54 = info
            .models
            .iter()
            .find(|model| model.id == "gpt-5.4")
            .expect("gpt-5.4 should be listed");
        assert_eq!(gpt_54.name.as_ref(), "GPT-5.4");
        assert!(gpt_54.supports_reasoning());
    }

    #[test]
    fn maps_chat_finish_reason_length_to_max_tokens() {
        assert_eq!(
            map_openai_finish_reason("length".into()),
            FinishReason::MaxTokens
        );
    }
}
