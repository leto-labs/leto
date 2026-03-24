//! Shared-provider adapter built on top of the OpenAI Responses API.
//!
//! Official references:
//! - Responses create: <https://developers.openai.com/api/reference/resources/responses/methods/create>
//! - Responses streaming events: <https://developers.openai.com/api/reference/resources/responses/streaming-events>

use std::collections::{HashMap, HashSet};

use async_stream::stream;
use futures::{StreamExt, future::BoxFuture};
use provider::{
    Block, BlockDelta, BlockKind, Error as ProviderError, Event, EventStream, FinishReason,
    Message, MessageRole, Provider, ProviderCapabilities, ProviderInfo, Request, StreamGranularity,
    ToolChoice, Usage,
};

use crate::responses::{
    ResponseEvent, ResponseInputContentPart, ResponseInputItem, ResponseInputRole,
    ResponseOutputContentPart, ResponseOutputItem, ResponseReasoningConfig,
    ResponseReasoningSummaryPart, ResponseRequest, ResponseStreamTransport, ResponseTextConfig,
    ResponseTool, ResponseToolCallItem,
};
use crate::{Client, Config, Error};

/// Shared [`provider::Provider`] adapter backed by the OpenAI Responses API.
#[derive(Debug, Clone)]
pub struct OpenAiProvider {
    client: Client,
    transport: ResponseStreamTransport,
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
        }
    }

    /// Selects the streaming transport used for Responses streaming.
    pub fn with_transport(mut self, transport: ResponseStreamTransport) -> Self {
        self.transport = transport;
        self
    }

    fn map_request(request: &Request) -> Result<ResponseRequest, ProviderError> {
        let mut input = Vec::new();
        for message in &request.messages {
            input.extend(map_message(message)?);
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

        Ok(ResponseRequest {
            model: request.model.clone(),
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
}

impl Provider for OpenAiProvider {
    fn stream<'a>(
        &'a self,
        request: &'a Request,
    ) -> BoxFuture<'a, Result<EventStream<'a>, ProviderError>> {
        Box::pin(async move {
            let mapped_request = Self::map_request(request)?;
            let response_stream = self
                .client
                .responses()
                .stream(&mapped_request, self.transport)
                .await
                .map_err(map_openai_error)?;

            let output_stream = stream! {
                let mut stream = response_stream;
                let mut started = HashMap::<String, Block>::new();
                let mut saw_delta = HashSet::<String>::new();

                yield Ok(Event::ResponseStart {
                    response_id: None,
                    model: request.model.clone(),
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
                            if let Some(block) = map_openai_output_item_start(output_index, &item) {
                                if let Some(start_event) = start_block_if_needed(&mut started, block) {
                                    yield Ok(start_event);
                                }
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
                            if let Some(delta) = map_openai_reasoning_part_delta(&part) {
                                if !saw_delta.contains(&block_id) {
                                    saw_delta.insert(block_id.clone());
                                    yield Ok(Event::BlockDelta { id: block_id.clone(), delta });
                                }
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
        })
    }

    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "openai".into(),
            default_model: Some(self.client.config().default_model.clone()),
            capabilities: ProviderCapabilities {
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
            models: Vec::new(),
        }
    }
}

fn map_openai_error(err: Error) -> ProviderError {
    match err {
        Error::Inference(message) => ProviderError::Inference(message),
        Error::Internal(message) => ProviderError::Configuration(message),
        Error::Json(err) => ProviderError::Json(err),
    }
}

fn map_message(message: &Message) -> Result<Vec<ResponseInputItem>, ProviderError> {
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

fn map_role(role: MessageRole) -> ResponseInputRole {
    match role {
        MessageRole::System => ResponseInputRole::System,
        MessageRole::Developer => ResponseInputRole::Developer,
        MessageRole::User => ResponseInputRole::User,
        MessageRole::Assistant => ResponseInputRole::Assistant,
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
        "max_output_tokens" | "max_tokens" => FinishReason::MaxTokens,
        "tool_calls" | "tool_call" => FinishReason::ToolCall,
        "stop_sequence" => FinishReason::StopSequence,
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

        let mapped = map_message(&message).unwrap();
        assert!(matches!(
            mapped[0],
            ResponseInputItem::FunctionCallOutput { .. }
        ));
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

        let mapped = map_message(&message).unwrap();
        match &mapped[0] {
            ResponseInputItem::Message { content, .. } => {
                assert_eq!(content.len(), 2);
            }
            _ => panic!("expected message item"),
        }
    }
}
