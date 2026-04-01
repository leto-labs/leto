//! Shared-provider adapter built on top of Anthropic Messages.
//!
//! Official references:
//! - Messages create: <https://platform.claude.com/docs/en/api/messages/create>
//! - Messages streaming: <https://platform.claude.com/docs/en/build-with-claude/streaming>

use std::collections::HashMap;

use async_stream::stream;
use futures::{StreamExt, future::BoxFuture};
use provider::{
    Block, BlockDelta, BlockKind, Error as ProviderError, Event, EventStream, FinishReason,
    Message, MessageRole, Provider, ProviderCapabilities, ProviderInfo, Request, StreamGranularity,
    ToolChoice, Usage,
};

use crate::messages::{
    ContentBlock, ContentBlockDelta, ContentBlockParam, ImageSource, MessageContentParam,
    MessageDeltaUsage, MessageParam, MessageRequest, MessageRole as AnthropicRole,
    MessageStreamEvent, StopReason, SystemPrompt, ThinkingConfig,
    ToolChoice as AnthropicToolChoice, ToolDefinition,
};
use crate::{Client, Config, Error};

/// Shared [`provider::Provider`] adapter backed by Anthropic Messages.
#[derive(Debug, Clone)]
pub struct AnthropicProvider {
    client: Client,
}

impl AnthropicProvider {
    /// Creates an adapter from an Anthropic configuration.
    pub fn new(config: Config) -> Self {
        Self::from_client(Client::new(config))
    }

    /// Wraps an existing protocol-native [`Client`].
    pub fn from_client(client: Client) -> Self {
        Self { client }
    }

    fn map_request(request: &Request) -> Result<MessageRequest, ProviderError> {
        let mut system_blocks = Vec::new();
        let mut messages = Vec::new();

        for message in &request.messages {
            match message.role {
                MessageRole::System | MessageRole::Developer => {
                    for block in &message.content {
                        match block {
                            provider::ContentBlock::Text { text }
                            | provider::ContentBlock::Reasoning { text }
                            | provider::ContentBlock::Refusal { text } => {
                                system_blocks
                                    .push(crate::messages::TextBlockParam::text(text.clone()));
                            }
                            provider::ContentBlock::ImageUrl { .. }
                            | provider::ContentBlock::ToolCall { .. }
                            | provider::ContentBlock::ToolResult { .. } => {
                                return Err(ProviderError::Unsupported(
                                    "Anthropic adapter only supports text-like system/developer blocks"
                                        .into(),
                                ));
                            }
                        }
                    }
                }
                MessageRole::User | MessageRole::Assistant => {
                    messages.push(map_message(message)?);
                }
            }
        }

        let system = if system_blocks.is_empty() {
            None
        } else {
            Some(SystemPrompt::Blocks(system_blocks))
        };

        let tool_choice = map_tool_choice(
            request.options.tool_choice.as_ref(),
            request.options.parallel_tool_calls,
            !request.tools.is_empty(),
        );

        let thinking = request.options.reasoning.as_ref().and_then(|reasoning| {
            reasoning
                .budget_tokens
                .map(|budget_tokens| ThinkingConfig::Enabled {
                    budget_tokens,
                    display: reasoning.summary.clone(),
                })
        });

        Ok(MessageRequest {
            model: request.model.clone(),
            max_tokens: request.options.max_output_tokens.unwrap_or(1024),
            messages,
            system,
            tools: request
                .tools
                .iter()
                .map(|tool| {
                    ToolDefinition::custom(
                        tool.name.clone(),
                        tool.description.clone().unwrap_or_default(),
                        tool.input_schema.clone(),
                    )
                })
                .collect(),
            tool_choice,
            thinking,
            output_config: request.options.reasoning.as_ref().map(|reasoning| {
                crate::messages::MessageOutputConfig {
                    effort: reasoning.effort.clone(),
                    format: None,
                }
            }),
            stop_sequences: request.options.stop_sequences.clone(),
            temperature: request.options.temperature,
            top_k: request.options.top_k,
            top_p: request.options.top_p,
            metadata: (!request.options.metadata.is_empty()).then(|| {
                serde_json::Value::Object(request.options.metadata.clone().into_iter().collect())
            }),
            ..MessageRequest::default()
        })
    }
}

impl Provider for AnthropicProvider {
    fn stream<'a>(
        &'a self,
        request: &'a Request,
    ) -> BoxFuture<'a, Result<EventStream<'a>, ProviderError>> {
        Box::pin(async move {
            let mapped_request = Self::map_request(request)?;
            let stream = self
                .client
                .messages()
                .stream(&mapped_request)
                .await
                .map_err(map_anthropic_error)?;

            let output_stream = stream! {
                let mut stream = stream;
                let mut started = HashMap::<u32, String>::new();
                let mut finish_reason = None;

                yield Ok(Event::ResponseStart {
                    response_id: None,
                    model: request.model.clone(),
                });

                while let Some(event) = stream.next().await {
                    let event = match event {
                        Ok(event) => event,
                        Err(err) => {
                            yield Err(map_anthropic_error(err));
                            break;
                        }
                    };

                    match event {
                        MessageStreamEvent::MessageStart { message } => {
                            if let Some(usage) = map_message_usage(message.usage.as_ref()) {
                                yield Ok(Event::Usage { usage });
                            }
                        }
                        MessageStreamEvent::MessageDelta { delta, usage } => {
                            if let Some(usage) = map_delta_usage(usage.as_ref()) {
                                yield Ok(Event::Usage { usage });
                            }
                            if let Some(reason) = delta.stop_reason {
                                finish_reason = Some(map_stop_reason(reason));
                            }
                        }
                        MessageStreamEvent::MessageStop => {
                            yield Ok(Event::Completed {
                                response_id: None,
                                finish_reason: finish_reason.clone(),
                            });
                        }
                        MessageStreamEvent::ContentBlockStart { index, content_block } => {
                            let block = map_content_block_start(index, content_block);
                            started.insert(index, block.id.clone());
                            yield Ok(Event::BlockStart { block });
                        }
                        MessageStreamEvent::ContentBlockDelta { index, delta } => {
                            let block_id = started
                                .get(&index)
                                .cloned()
                                .unwrap_or_else(|| format!("anthropic-block:{index}"));
                            if !started.contains_key(&index) {
                                let block = Block {
                                    id: block_id.clone(),
                                    output_index: index,
                                    kind: guess_block_kind_from_delta(&delta),
                                    item_id: None,
                                };
                                started.insert(index, block.id.clone());
                                yield Ok(Event::BlockStart { block });
                            }
                            yield Ok(Event::BlockDelta {
                                id: block_id,
                                delta: map_content_block_delta(delta),
                            });
                        }
                        MessageStreamEvent::ContentBlockStop { index } => {
                            if let Some(block_id) = started.remove(&index) {
                                yield Ok(Event::BlockStop { id: block_id });
                            }
                        }
                        MessageStreamEvent::Ping => {}
                        MessageStreamEvent::Error { error, .. } => {
                            let message = error.message.unwrap_or_else(|| "Anthropic stream error".into());
                            yield Err(ProviderError::Remote(message));
                            break;
                        }
                        MessageStreamEvent::Unknown { .. } => {}
                    }
                }
            };

            Ok(Box::pin(output_stream) as EventStream<'a>)
        })
    }

    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "anthropic".into(),
            default_model_id: Some(self.client.config().default_model.clone()),
            capabilities: ProviderCapabilities {
                system_messages: true,
                developer_messages: true,
                input_text: true,
                input_image_urls: true,
                tool_calls: true,
                tool_results: true,
                reasoning_blocks: true,
                refusal_blocks: false,
                tool_call_argument_deltas: true,
                parallel_tool_calls: true,
                stream_granularity: StreamGranularity::Block,
            },
            models: Vec::new(),
        }
    }
}

fn map_anthropic_error(err: Error) -> ProviderError {
    match err {
        Error::Inference(message) => ProviderError::Inference(message),
        Error::Internal(message) => ProviderError::Configuration(message),
        Error::Json(err) => ProviderError::Json(err),
    }
}

fn map_message(message: &Message) -> Result<MessageParam, ProviderError> {
    let role = match message.role {
        MessageRole::User => AnthropicRole::User,
        MessageRole::Assistant => AnthropicRole::Assistant,
        MessageRole::System | MessageRole::Developer => {
            return Err(ProviderError::Unsupported(
                "Anthropic message mapping expects system/developer messages to be lifted into the top-level system prompt".into(),
            ));
        }
    };

    let mut blocks = Vec::new();
    for block in &message.content {
        blocks.push(match block {
            provider::ContentBlock::Text { text } => ContentBlockParam::Text { text: text.clone() },
            provider::ContentBlock::ImageUrl { url } => ContentBlockParam::Image {
                source: ImageSource::Url { url: url.clone() },
            },
            provider::ContentBlock::ToolCall {
                id, name, input, ..
            } => ContentBlockParam::ToolUse {
                id: id.clone(),
                name: name.clone(),
                input: input.clone(),
                caller: None,
            },
            provider::ContentBlock::ToolResult {
                call_id,
                output,
                is_error,
            } => ContentBlockParam::ToolResult {
                tool_use_id: call_id.clone(),
                content: Some(crate::messages::ToolResultContent::Text(
                    json_value_to_string(output),
                )),
                is_error: *is_error,
            },
            provider::ContentBlock::Reasoning { text } => ContentBlockParam::Thinking {
                thinking: text.clone(),
                signature: String::new(),
            },
            provider::ContentBlock::Refusal { text } => {
                ContentBlockParam::Text { text: text.clone() }
            }
        });
    }

    Ok(MessageParam {
        role,
        content: MessageContentParam::Blocks(blocks),
    })
}

fn map_tool_choice(
    choice: Option<&ToolChoice>,
    parallel_tool_calls: Option<bool>,
    has_tools: bool,
) -> Option<AnthropicToolChoice> {
    let disable_parallel_tool_use = parallel_tool_calls.map(|enabled| !enabled);
    match choice {
        Some(ToolChoice::Auto) => Some(AnthropicToolChoice::Auto {
            disable_parallel_tool_use,
        }),
        Some(ToolChoice::Required) => Some(AnthropicToolChoice::Any {
            disable_parallel_tool_use,
        }),
        Some(ToolChoice::Tool { name }) => Some(AnthropicToolChoice::Tool {
            name: name.clone(),
            disable_parallel_tool_use,
        }),
        Some(ToolChoice::None) => Some(AnthropicToolChoice::None),
        None if has_tools && disable_parallel_tool_use.is_some() => {
            Some(AnthropicToolChoice::Auto {
                disable_parallel_tool_use,
            })
        }
        None => None,
    }
}

fn json_value_to_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(text) => text.clone(),
        _ => value.to_string(),
    }
}

fn map_message_usage(usage: Option<&crate::messages::MessageUsage>) -> Option<Usage> {
    usage.map(|usage| {
        let mut usage_out = Usage::with_totals(Some(usage.input_tokens), Some(usage.output_tokens));
        usage_out.cache_read_tokens = usage.cache_read_input_tokens;
        usage_out.cache_write_tokens = usage.cache_creation_input_tokens;
        usage_out
    })
}

fn map_delta_usage(usage: Option<&MessageDeltaUsage>) -> Option<Usage> {
    usage.map(|usage| {
        let mut usage_out = Usage::with_totals(usage.input_tokens, usage.output_tokens);
        usage_out.cache_read_tokens = usage.cache_read_input_tokens;
        usage_out.cache_write_tokens = usage.cache_creation_input_tokens;
        usage_out
    })
}

fn map_stop_reason(reason: StopReason) -> FinishReason {
    match reason {
        StopReason::EndTurn => FinishReason::Stop,
        StopReason::MaxTokens => FinishReason::MaxTokens,
        StopReason::StopSequence => FinishReason::StopSequence,
        StopReason::ToolUse => FinishReason::ToolCall,
        StopReason::PauseTurn => FinishReason::Pause,
        StopReason::Refusal => FinishReason::Refusal,
        StopReason::Unknown => FinishReason::Unknown("unknown".into()),
    }
}

fn map_content_block_start(index: u32, content_block: ContentBlock) -> Block {
    let (id, kind) = match content_block {
        ContentBlock::Text { .. } => (format!("anthropic-text:{index}"), BlockKind::Text),
        ContentBlock::ToolUse { id, name, .. } => (
            format!("anthropic-tool:{id}"),
            BlockKind::ToolCall {
                name: Some(name),
                call_id: Some(id),
            },
        ),
        ContentBlock::Thinking { .. } => {
            (format!("anthropic-thinking:{index}"), BlockKind::Reasoning)
        }
        ContentBlock::RedactedThinking { .. } => (
            format!("anthropic-redacted-thinking:{index}"),
            BlockKind::Unknown {
                kind: "redacted_thinking".into(),
            },
        ),
        ContentBlock::Unknown { block_type, .. } => (
            format!("anthropic-unknown:{index}"),
            BlockKind::Unknown { kind: block_type },
        ),
    };

    Block {
        id,
        output_index: index,
        kind,
        item_id: None,
    }
}

fn guess_block_kind_from_delta(delta: &ContentBlockDelta) -> BlockKind {
    match delta {
        ContentBlockDelta::TextDelta { .. } => BlockKind::Text,
        ContentBlockDelta::InputJsonDelta { .. } => BlockKind::ToolCall {
            name: None,
            call_id: None,
        },
        ContentBlockDelta::ThinkingDelta { .. } | ContentBlockDelta::SignatureDelta { .. } => {
            BlockKind::Reasoning
        }
        ContentBlockDelta::Unknown { delta_type, .. } => BlockKind::Unknown {
            kind: delta_type.clone(),
        },
    }
}

fn map_content_block_delta(delta: ContentBlockDelta) -> BlockDelta {
    match delta {
        ContentBlockDelta::TextDelta { text } => BlockDelta::Text { text },
        ContentBlockDelta::InputJsonDelta { partial_json } => BlockDelta::Json { partial_json },
        ContentBlockDelta::ThinkingDelta { thinking } => BlockDelta::Reasoning { text: thinking },
        ContentBlockDelta::SignatureDelta { signature } => BlockDelta::Signature { signature },
        ContentBlockDelta::Unknown { raw, .. } => BlockDelta::Unknown { raw },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_system_and_developer_blocks_into_top_level_system_prompt() {
        let request = Request {
            messages: vec![
                Message::new(
                    MessageRole::System,
                    vec![
                        provider::ContentBlock::text("system instruction"),
                        provider::ContentBlock::Reasoning {
                            text: "internal guidance".into(),
                        },
                    ],
                ),
                Message::new(
                    MessageRole::Developer,
                    vec![provider::ContentBlock::Refusal {
                        text: "developer refusal guidance".into(),
                    }],
                ),
                Message::user_text("hello"),
            ],
            ..Request::default()
        };

        let mapped = AnthropicProvider::map_request(&request).unwrap();

        assert_eq!(
            mapped.system,
            Some(crate::messages::SystemPrompt::Blocks(vec![
                crate::messages::TextBlockParam::text("system instruction"),
                crate::messages::TextBlockParam::text("internal guidance"),
                crate::messages::TextBlockParam::text("developer refusal guidance"),
            ]))
        );
        assert_eq!(mapped.messages.len(), 1);
        assert_eq!(mapped.messages[0].role, AnthropicRole::User);
        assert_eq!(
            mapped.messages[0].content,
            MessageContentParam::Blocks(vec![ContentBlockParam::Text {
                text: "hello".into(),
            }])
        );
    }

    #[test]
    fn maps_tool_result_message_to_anthropic_tool_result_block() {
        let message = Message::new(
            MessageRole::User,
            vec![provider::ContentBlock::tool_result(
                "call-1",
                serde_json::json!({ "ok": true }),
            )],
        );

        let mapped = map_message(&message).unwrap();
        match mapped.content {
            MessageContentParam::Blocks(blocks) => {
                assert!(matches!(blocks[0], ContentBlockParam::ToolResult { .. }));
            }
            MessageContentParam::Text(_) => panic!("expected block content"),
        }
    }

    #[test]
    fn maps_parallel_tool_setting_into_tool_choice() {
        let choice = map_tool_choice(Some(&ToolChoice::Auto), Some(false), true).unwrap();
        match choice {
            AnthropicToolChoice::Auto {
                disable_parallel_tool_use,
            } => assert_eq!(disable_parallel_tool_use, Some(true)),
            _ => panic!("expected auto tool choice"),
        }
    }
}
