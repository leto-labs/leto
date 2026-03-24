//! Deterministic mock implementation of the shared [`crate::Provider`] trait.

use std::time::Duration;

use async_stream::stream;
use futures::future::BoxFuture;

use crate::{
    Block, BlockKind, Error, Event, EventStream, FinishReason, ModelInfo, Provider,
    ProviderCapabilities, ProviderInfo, Request, Usage,
};

/// Test provider that echoes the last user message and can simulate a tool
/// call.
pub struct MockProvider {
    /// Optional per-token delay used to simulate streaming latency.
    pub delay_ms: u64,
}

impl MockProvider {
    /// Creates a mock provider with no artificial delay.
    pub fn new() -> Self {
        Self { delay_ms: 0 }
    }

    /// Sets an artificial delay in milliseconds between emitted text chunks.
    pub fn with_delay(mut self, ms: u64) -> Self {
        self.delay_ms = ms;
        self
    }
}

impl Default for MockProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl Provider for MockProvider {
    fn stream<'a>(&'a self, request: &'a Request) -> BoxFuture<'a, Result<EventStream<'a>, Error>> {
        Box::pin(async move {
            let last_user = request.last_user_text_lossy().unwrap_or_default();
            let delay = self.delay_ms;
            let first_tool_name = request.tools.first().map(|tool| tool.name.clone());

            let stream = stream! {
                yield Ok(Event::ResponseStart {
                    response_id: Some("mock-response-1".into()),
                    model: Some(request.model.clone().unwrap_or_else(|| "mock-echo".into())),
                });

                if last_user.starts_with("tool:") && first_tool_name.is_some() {
                    let block_id = "mock-tool-call-1".to_string();
                    yield Ok(Event::BlockStart {
                        block: Block {
                            id: block_id.clone(),
                            output_index: 0,
                            kind: BlockKind::ToolCall {
                                name: first_tool_name.clone(),
                                call_id: Some("mock-call-1".into()),
                            },
                            item_id: Some("mock-item-1".into()),
                        },
                    });
                    let message = last_user.strip_prefix("tool:").unwrap_or("").trim();
                    yield Ok(Event::BlockDelta {
                        id: block_id.clone(),
                        delta: crate::BlockDelta::Json {
                            partial_json: serde_json::json!({ "message": message }).to_string(),
                        },
                    });
                    yield Ok(Event::BlockStop { id: block_id });
                } else {
                    let block_id = "mock-text-1".to_string();
                    yield Ok(Event::BlockStart {
                        block: Block {
                            id: block_id.clone(),
                            output_index: 0,
                            kind: BlockKind::Text,
                            item_id: Some("mock-item-1".into()),
                        },
                    });

                    for word in last_user.split_whitespace() {
                        if delay > 0 {
                            tokio::time::sleep(Duration::from_millis(delay)).await;
                        }

                        yield Ok(Event::text_delta(block_id.clone(), format!("{word} ")));
                    }

                    yield Ok(Event::BlockStop { id: block_id });
                }

                let approx_tokens = (last_user.len() as u32) / 4;
                yield Ok(Event::Usage {
                    usage: Usage {
                        input_tokens: Some(approx_tokens),
                        output_tokens: Some(approx_tokens),
                        total_tokens: Some(approx_tokens.saturating_mul(2)),
                        cache_read_tokens: None,
                        cache_write_tokens: None,
                        reasoning_tokens: None,
                    },
                });
                yield Ok(Event::Completed {
                    response_id: Some("mock-response-1".into()),
                    finish_reason: Some(FinishReason::Stop),
                });
            };

            Ok(Box::pin(stream) as EventStream<'a>)
        })
    }

    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "mock".into(),
            default_model: Some("mock-echo".into()),
            capabilities: ProviderCapabilities {
                system_messages: true,
                developer_messages: true,
                input_text: true,
                input_image_urls: false,
                tool_calls: true,
                tool_results: true,
                reasoning_blocks: false,
                refusal_blocks: false,
                tool_call_argument_deltas: true,
                parallel_tool_calls: false,
                stream_granularity: crate::StreamGranularity::Block,
            },
            models: vec![ModelInfo {
                id: "mock-echo".into(),
                name: Some("Mock Echo".into()),
                capabilities: None,
            }],
        }
    }
}
