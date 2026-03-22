use async_stream::stream;
use eventsource_stream::Eventsource;
use futures::StreamExt;
use serde::{Deserialize, Serialize};

use brain_types::*;

// ── Request types ──

#[derive(Serialize)]
pub(crate) struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<OaiMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<OaiTool>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<StreamOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
}

#[derive(Serialize)]
pub(crate) struct OaiMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<OaiToolCallOut>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct OaiToolCallOut {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: OaiFunctionCallOut,
}

#[derive(Serialize)]
pub(crate) struct OaiFunctionCallOut {
    pub name: String,
    pub arguments: String,
}

#[derive(Serialize)]
pub(crate) struct OaiTool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: OaiFunctionDef,
}

#[derive(Serialize)]
pub(crate) struct OaiFunctionDef {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Serialize)]
pub(crate) struct StreamOptions {
    pub include_usage: bool,
}

// ── Response types ──

#[derive(Deserialize)]
pub(crate) struct ChatCompletionChunk {
    pub choices: Vec<ChunkChoice>,
    pub usage: Option<ChunkUsage>,
}

#[derive(Deserialize)]
pub(crate) struct ChunkChoice {
    pub delta: ChunkDelta,
    pub finish_reason: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct ChunkDelta {
    pub content: Option<String>,
    pub tool_calls: Option<Vec<ToolCallChunk>>,
}

#[derive(Deserialize)]
pub(crate) struct ToolCallChunk {
    pub index: u32,
    pub id: Option<String>,
    pub function: Option<FunctionCallChunk>,
}

#[derive(Deserialize)]
pub(crate) struct FunctionCallChunk {
    pub name: Option<String>,
    pub arguments: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct ChunkUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    #[serde(default)]
    pub prompt_tokens_details: Option<PromptTokenDetails>,
    #[serde(default)]
    pub completion_tokens_details: Option<CompletionTokenDetails>,
}

#[derive(Deserialize)]
pub(crate) struct PromptTokenDetails {
    #[serde(default)]
    pub cached_tokens: Option<u32>,
    #[serde(default)]
    pub cache_creation_tokens: Option<u32>,
}

#[derive(Deserialize)]
pub(crate) struct CompletionTokenDetails {
    #[serde(default)]
    pub reasoning_tokens: Option<u32>,
}

#[derive(Default)]
pub(crate) struct AccumulatedToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

// ── Conversion helpers ──

pub(crate) fn to_oai_message(msg: &Message) -> OaiMessage {
    let role = match msg.role {
        Role::System => "system",
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::Tool => "tool",
    };

    let content = if msg.content.is_empty() && !msg.tool_calls.is_empty() {
        None
    } else {
        Some(msg.content.clone())
    };

    OaiMessage {
        role: role.into(),
        content,
        tool_calls: msg
            .tool_calls
            .iter()
            .map(|tc| OaiToolCallOut {
                id: tc.id.clone(),
                call_type: "function".into(),
                function: OaiFunctionCallOut {
                    name: tc.name.clone(),
                    arguments: tc.arguments.to_string(),
                },
            })
            .collect(),
        tool_call_id: msg.tool_call_id.clone(),
    }
}

pub(crate) fn to_oai_tool(def: &ToolDef) -> OaiTool {
    OaiTool {
        tool_type: "function".into(),
        function: OaiFunctionDef {
            name: def.name.clone(),
            description: def.description.clone(),
            parameters: def.parameters.clone(),
        },
    }
}

/// Convert an SSE byte stream from an OpenAI-compatible chat completions
/// endpoint into a `ChatStream` of `ChatChunk` items.
pub(crate) fn stream_from_response(response: reqwest::Response) -> ChatStream {
    let event_source = response.bytes_stream().eventsource();

    let s = stream! {
        let mut tool_calls: Vec<AccumulatedToolCall> = Vec::new();
        let mut got_done = false;
        let mut event_stream = Box::pin(event_source);

        while let Some(result) = event_stream.next().await {
            let event = match result {
                Ok(ev) => ev,
                Err(e) => {
                    yield Err(BrainError::Inference(e.to_string()));
                    break;
                }
            };

            if event.data == "[DONE]" {
                if !got_done {
                    yield Ok(ChatChunk::Done { usage: None });
                }
                break;
            }

            let chunk: ChatCompletionChunk = match serde_json::from_str(&event.data) {
                Ok(c) => c,
                Err(e) => {
                    yield Err(BrainError::Json(e));
                    break;
                }
            };

            if chunk.choices.is_empty() {
                if let Some(usage) = chunk.usage {
                    for tc in tool_calls.drain(..) {
                        if !tc.id.is_empty() {
                            let args = serde_json::from_str(&tc.arguments)
                                .unwrap_or(serde_json::Value::Object(Default::default()));
                            yield Ok(ChatChunk::ToolCall {
                                id: tc.id,
                                name: tc.name,
                                arguments: args,
                            });
                        }
                    }
                    yield Ok(ChatChunk::Done {
                        usage: Some(TokenUsage {
                            prompt: usage.prompt_tokens,
                            completion: usage.completion_tokens,
                            total: usage.total_tokens,
                            cache_read: usage
                                .prompt_tokens_details
                                .as_ref()
                                .and_then(|details| details.cached_tokens),
                            cache_write: usage
                                .prompt_tokens_details
                                .as_ref()
                                .and_then(|details| details.cache_creation_tokens),
                            reasoning: usage
                                .completion_tokens_details
                                .as_ref()
                                .and_then(|details| details.reasoning_tokens),
                        }),
                    });
                    got_done = true;
                }
                continue;
            }

            let choice = &chunk.choices[0];

            if let Some(ref content) = choice.delta.content {
                if !content.is_empty() {
                    yield Ok(ChatChunk::Delta { content: content.clone() });
                }
            }

            if let Some(ref tc_chunks) = choice.delta.tool_calls {
                for tc in tc_chunks {
                    let idx = tc.index as usize;
                    while tool_calls.len() <= idx {
                        tool_calls.push(AccumulatedToolCall::default());
                    }
                    let acc = &mut tool_calls[idx];
                    if let Some(ref id) = tc.id {
                        acc.id.clone_from(id);
                    }
                    if let Some(ref func) = tc.function {
                        if let Some(ref name) = func.name {
                            acc.name.clone_from(name);
                        }
                        if let Some(ref args) = func.arguments {
                            acc.arguments.push_str(args);
                        }
                    }
                }
            }

            if choice.finish_reason.is_some() {
                for tc in tool_calls.drain(..) {
                    if !tc.id.is_empty() {
                        let args = serde_json::from_str(&tc.arguments)
                            .unwrap_or(serde_json::Value::Object(Default::default()));
                        yield Ok(ChatChunk::ToolCall {
                            id: tc.id,
                            name: tc.name,
                            arguments: args,
                        });
                    }
                }
            }
        }
    };

    Box::pin(s) as ChatStream
}

#[cfg(test)]
mod tests {
    use super::ChunkUsage;

    #[test]
    fn chunk_usage_deserializes_detailed_token_fields() {
        let usage: ChunkUsage = serde_json::from_str(
            r#"{
                "prompt_tokens": 100,
                "completion_tokens": 60,
                "total_tokens": 160,
                "prompt_tokens_details": {
                    "cached_tokens": 25,
                    "cache_creation_tokens": 5
                },
                "completion_tokens_details": {
                    "reasoning_tokens": 12
                }
            }"#,
        )
        .unwrap();

        assert_eq!(usage.prompt_tokens, 100);
        assert_eq!(usage.completion_tokens, 60);
        assert_eq!(
            usage
                .prompt_tokens_details
                .as_ref()
                .and_then(|details| details.cached_tokens),
            Some(25)
        );
        assert_eq!(
            usage
                .prompt_tokens_details
                .as_ref()
                .and_then(|details| details.cache_creation_tokens),
            Some(5)
        );
        assert_eq!(
            usage
                .completion_tokens_details
                .as_ref()
                .and_then(|details| details.reasoning_tokens),
            Some(12)
        );
    }
}
