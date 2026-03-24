//! Parsers for Chat Completions responses and SSE chunks.

use async_stream::stream;
use eventsource_stream::Eventsource;
use futures::StreamExt;
use serde::Deserialize;
use serde_json::Value;

use crate::chat_completions::client::ChatCompletionStream;
use crate::chat_completions::types::{
    ChatCompletionChunk, ChatCompletionChunkChoice, ChatCompletionChunkDelta,
    ChatCompletionFunctionCall, ChatCompletionFunctionCallChunk, ChatCompletionMessage,
    ChatCompletionMessageContent, ChatCompletionObject, ChatCompletionRole, ChatCompletionToolCall,
    ChatCompletionToolCallChunk,
};
use crate::{Error, TokenUsage};

#[derive(Deserialize)]
struct WireChatCompletionObject {
    id: Option<String>,
    object: Option<String>,
    created: Option<i64>,
    model: Option<String>,
    #[serde(default)]
    choices: Vec<WireChatCompletionChoice>,
    usage: Option<WireChunkUsage>,
}

#[derive(Deserialize)]
struct WireChatCompletionChoice {
    index: u32,
    message: WireChatCompletionMessage,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct WireChatCompletionMessage {
    role: Option<String>,
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<WireToolCall>,
}

#[derive(Deserialize)]
struct WireChatCompletionChunk {
    id: Option<String>,
    object: Option<String>,
    created: Option<i64>,
    model: Option<String>,
    #[serde(default)]
    choices: Vec<WireChunkChoice>,
    usage: Option<WireChunkUsage>,
}

#[derive(Deserialize)]
struct WireChunkChoice {
    index: u32,
    delta: WireChunkDelta,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct WireChunkDelta {
    role: Option<String>,
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<WireToolCallChunk>,
}

#[derive(Deserialize)]
struct WireToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: WireFunctionCall,
}

#[derive(Deserialize)]
struct WireFunctionCall {
    name: String,
    arguments: String,
}

#[derive(Deserialize)]
struct WireToolCallChunk {
    #[serde(default)]
    index: Option<u32>,
    id: Option<String>,
    function: Option<WireFunctionCallChunk>,
}

#[derive(Deserialize)]
struct WireFunctionCallChunk {
    name: Option<String>,
    arguments: Option<String>,
}

#[derive(Deserialize)]
struct WireChunkUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
    #[serde(default)]
    prompt_tokens_details: Option<PromptTokenDetails>,
    #[serde(default)]
    completion_tokens_details: Option<CompletionTokenDetails>,
}

#[derive(Deserialize)]
struct PromptTokenDetails {
    #[serde(default)]
    cached_tokens: Option<u32>,
    #[serde(default)]
    cache_creation_tokens: Option<u32>,
}

#[derive(Deserialize)]
struct CompletionTokenDetails {
    #[serde(default)]
    reasoning_tokens: Option<u32>,
}

/// Parses a non-streaming chat completion from raw JSON.
pub(crate) fn parse_chat_completion_object_value(
    raw: Value,
) -> Result<ChatCompletionObject, Error> {
    let wire: WireChatCompletionObject = serde_json::from_value(raw.clone())?;
    Ok(ChatCompletionObject {
        id: wire.id,
        object: wire.object,
        created: wire.created,
        model: wire.model,
        choices: wire.choices.into_iter().map(map_choice).collect(),
        usage: wire.usage.map(map_usage),
        raw,
    })
}

/// Parses a streamed chat-completions chunk from raw JSON.
pub(crate) fn parse_chat_completion_chunk_value(raw: Value) -> Result<ChatCompletionChunk, Error> {
    let wire: WireChatCompletionChunk = serde_json::from_value(raw.clone())?;
    Ok(ChatCompletionChunk {
        id: wire.id,
        object: wire.object,
        created: wire.created,
        model: wire.model,
        choices: wire.choices.into_iter().map(map_chunk_choice).collect(),
        usage: wire.usage.map(map_usage),
        raw,
    })
}

/// Builds a typed chunk stream from a streaming HTTP response.
pub(crate) fn sse_stream_from_response(response: reqwest::Response) -> ChatCompletionStream {
    let event_source = response.bytes_stream().eventsource();

    let s = stream! {
        let mut event_stream = Box::pin(event_source);
        let mut saw_done = false;

        while let Some(result) = event_stream.next().await {
            let event = match result {
                Ok(ev) => ev,
                Err(err) => {
                    yield Err(Error::Inference(err.to_string()));
                    break;
                }
            };

            if event.data == "[DONE]" {
                saw_done = true;
                break;
            }

            let raw: Value = match serde_json::from_str(&event.data) {
                Ok(value) => value,
                Err(err) => {
                    yield Err(Error::Json(err));
                    break;
                }
            };

            match parse_chat_completion_chunk_value(raw) {
                Ok(mapped) => yield Ok(mapped),
                Err(err) => {
                    yield Err(err);
                    break;
                }
            }
        }

        if !saw_done {
            yield Err(Error::Inference("stream closed before [DONE]".into()));
        }
    };

    Box::pin(s)
}

fn map_choice(wire: WireChatCompletionChoice) -> crate::chat_completions::ChatCompletionChoice {
    crate::chat_completions::ChatCompletionChoice {
        index: wire.index,
        message: ChatCompletionMessage {
            role: wire
                .message
                .role
                .as_deref()
                .map(map_role)
                .unwrap_or(ChatCompletionRole::Assistant),
            content: wire.message.content.map(ChatCompletionMessageContent::Text),
            name: None,
            tool_calls: wire
                .message
                .tool_calls
                .into_iter()
                .map(map_tool_call)
                .collect(),
            tool_call_id: None,
            extra: Default::default(),
        },
        finish_reason: wire.finish_reason,
    }
}

fn map_chunk_choice(wire: WireChunkChoice) -> ChatCompletionChunkChoice {
    ChatCompletionChunkChoice {
        index: wire.index,
        delta: ChatCompletionChunkDelta {
            role: wire.delta.role.as_deref().map(map_role),
            content: wire.delta.content,
            tool_calls: wire
                .delta
                .tool_calls
                .into_iter()
                .map(map_tool_call_chunk)
                .collect(),
        },
        finish_reason: wire.finish_reason,
    }
}

fn map_tool_call(wire: WireToolCall) -> ChatCompletionToolCall {
    ChatCompletionToolCall {
        id: wire.id,
        call_type: wire.call_type,
        function: ChatCompletionFunctionCall {
            name: wire.function.name,
            arguments: wire.function.arguments,
        },
    }
}

fn map_tool_call_chunk(wire: WireToolCallChunk) -> ChatCompletionToolCallChunk {
    ChatCompletionToolCallChunk {
        index: wire.index,
        id: wire.id,
        function: wire
            .function
            .map(|function| ChatCompletionFunctionCallChunk {
                name: function.name,
                arguments: function.arguments,
            }),
    }
}

fn map_role(role: &str) -> ChatCompletionRole {
    match role {
        "system" => ChatCompletionRole::System,
        "user" => ChatCompletionRole::User,
        "tool" => ChatCompletionRole::Tool,
        "developer" => ChatCompletionRole::Developer,
        _ => ChatCompletionRole::Assistant,
    }
}

fn map_usage(wire: WireChunkUsage) -> TokenUsage {
    TokenUsage {
        prompt: wire.prompt_tokens,
        completion: wire.completion_tokens,
        total: wire.total_tokens,
        cache_read: wire
            .prompt_tokens_details
            .as_ref()
            .and_then(|details| details.cached_tokens),
        cache_write: wire
            .prompt_tokens_details
            .as_ref()
            .and_then(|details| details.cache_creation_tokens),
        reasoning: wire
            .completion_tokens_details
            .as_ref()
            .and_then(|details| details.reasoning_tokens),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_chat_completion_chunk_delta() {
        let raw = serde_json::json!({
            "id": "chatcmpl-123",
            "object": "chat.completion.chunk",
            "choices": [
                {
                    "index": 0,
                    "delta": { "role": "assistant", "content": "hel" },
                    "finish_reason": null
                }
            ]
        });

        let parsed = parse_chat_completion_chunk_value(raw).unwrap();
        assert_eq!(parsed.id.as_deref(), Some("chatcmpl-123"));
        assert_eq!(parsed.choices[0].delta.content.as_deref(), Some("hel"));
        assert_eq!(
            parsed.choices[0].delta.role,
            Some(ChatCompletionRole::Assistant)
        );
    }
}
