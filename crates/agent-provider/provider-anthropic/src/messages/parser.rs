//! Parsers for Anthropic Messages objects and SSE events.

use async_stream::stream;
use eventsource_stream::Eventsource;
use futures::StreamExt;
use serde_json::Value;

use crate::Error;
use crate::messages::client::MessageStream;
use crate::messages::types::{
    ContentBlock, ContentBlockDelta, MessageDelta, MessageDeltaUsage, MessageErrorInfo,
    MessageObject, MessageStreamEvent, MessageUsage, ServerToolUsage, StopReason,
};

/// Builds a typed SSE stream from an HTTP response.
pub(crate) fn sse_stream_from_response(response: reqwest::Response) -> MessageStream {
    let event_source = response.bytes_stream().eventsource();

    let s = stream! {
        let mut event_stream = Box::pin(event_source);
        let mut saw_terminal = false;

        while let Some(result) = event_stream.next().await {
            let event = match result {
                Ok(ev) => ev,
                Err(err) => {
                    yield Err(Error::Inference(err.to_string()));
                    break;
                }
            };

            if event.event == "ping" {
                yield Ok(MessageStreamEvent::Ping);
                continue;
            }

            if event.data.trim().is_empty() {
                continue;
            }

            let raw: Value = match serde_json::from_str(&event.data) {
                Ok(value) => value,
                Err(err) => {
                    yield Err(Error::Json(err));
                    break;
                }
            };

            match parse_message_stream_event(raw, Some(event.event.as_str())) {
                Ok(mapped) => {
                    if matches!(
                        mapped,
                        MessageStreamEvent::MessageStop | MessageStreamEvent::Error { .. }
                    ) {
                        saw_terminal = true;
                    }
                    yield Ok(mapped);
                }
                Err(err) => {
                    yield Err(err);
                    break;
                }
            }
        }

        if !saw_terminal {
            yield Err(Error::Inference("stream closed before terminal message event".into()));
        }
    };

    Box::pin(s)
}

/// Parses a non-streaming message object from raw JSON.
pub(crate) fn parse_message_object_value(raw: Value) -> Result<MessageObject, Error> {
    Ok(MessageObject {
        id: optional_string_field(&raw, "id"),
        object: optional_string_field(&raw, "type"),
        role: optional_string_field(&raw, "role"),
        content: raw
            .get("content")
            .and_then(Value::as_array)
            .map(|items| items.iter().map(parse_content_block).collect())
            .unwrap_or_default(),
        model: optional_string_field(&raw, "model"),
        stop_reason: raw.get("stop_reason").and_then(parse_stop_reason),
        stop_sequence: optional_string_field(&raw, "stop_sequence"),
        usage: raw.get("usage").map(parse_usage),
        raw,
    })
}

/// Parses a single Anthropic Messages SSE payload.
pub(crate) fn parse_message_stream_event(
    raw: Value,
    event_name: Option<&str>,
) -> Result<MessageStreamEvent, Error> {
    let event_type = raw
        .get("type")
        .and_then(Value::as_str)
        .map(|value| value.to_owned())
        .or_else(|| event_name.map(|value| value.to_owned()))
        .ok_or_else(|| Error::Inference("messages event missing type".into()))?;

    let event = match event_type.as_str() {
        "message_start" => MessageStreamEvent::MessageStart {
            message: parse_message_object_value(
                raw.get("message")
                    .cloned()
                    .unwrap_or_else(|| Value::Object(Default::default())),
            )?,
        },
        "message_delta" => MessageStreamEvent::MessageDelta {
            delta: MessageDelta {
                stop_reason: raw
                    .get("delta")
                    .and_then(|delta| delta.get("stop_reason"))
                    .and_then(parse_stop_reason),
                stop_sequence: raw
                    .get("delta")
                    .and_then(|delta| delta.get("stop_sequence"))
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                container: raw
                    .get("delta")
                    .and_then(|delta| delta.get("container"))
                    .cloned(),
            },
            usage: raw.get("usage").map(parse_message_delta_usage),
        },
        "message_stop" => MessageStreamEvent::MessageStop,
        "content_block_start" => MessageStreamEvent::ContentBlockStart {
            index: u32_field(&raw, "index"),
            content_block: parse_content_block(
                raw.get("content_block")
                    .unwrap_or(&Value::Object(Default::default())),
            ),
        },
        "content_block_delta" => MessageStreamEvent::ContentBlockDelta {
            index: u32_field(&raw, "index"),
            delta: parse_content_block_delta(
                raw.get("delta")
                    .unwrap_or(&Value::Object(Default::default())),
            ),
        },
        "content_block_stop" => MessageStreamEvent::ContentBlockStop {
            index: u32_field(&raw, "index"),
        },
        "ping" => MessageStreamEvent::Ping,
        "error" => MessageStreamEvent::Error {
            error: MessageErrorInfo {
                error_type: raw
                    .get("error")
                    .and_then(|value| value.get("type"))
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                message: raw
                    .get("error")
                    .and_then(|value| value.get("message"))
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            },
            raw,
        },
        _ => MessageStreamEvent::Unknown { event_type, raw },
    };

    Ok(event)
}

fn parse_content_block(raw: &Value) -> ContentBlock {
    let block_type = raw
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_owned();

    match block_type.as_str() {
        "text" => ContentBlock::Text {
            text: optional_string_field(raw, "text").unwrap_or_default(),
            citations: raw.get("citations").cloned(),
        },
        "tool_use" => ContentBlock::ToolUse {
            id: optional_string_field(raw, "id").unwrap_or_default(),
            name: optional_string_field(raw, "name").unwrap_or_default(),
            input: raw.get("input").cloned().unwrap_or(Value::Null),
            caller: raw.get("caller").cloned(),
        },
        "thinking" => ContentBlock::Thinking {
            thinking: optional_string_field(raw, "thinking").unwrap_or_default(),
            signature: optional_string_field(raw, "signature"),
        },
        "redacted_thinking" => ContentBlock::RedactedThinking {
            data: optional_string_field(raw, "data").unwrap_or_default(),
        },
        _ => ContentBlock::Unknown {
            block_type,
            raw: raw.clone(),
        },
    }
}

fn parse_content_block_delta(raw: &Value) -> ContentBlockDelta {
    let delta_type = raw
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_owned();

    match delta_type.as_str() {
        "text_delta" => ContentBlockDelta::TextDelta {
            text: optional_string_field(raw, "text").unwrap_or_default(),
        },
        "input_json_delta" => ContentBlockDelta::InputJsonDelta {
            partial_json: optional_string_field(raw, "partial_json").unwrap_or_default(),
        },
        "thinking_delta" => ContentBlockDelta::ThinkingDelta {
            thinking: optional_string_field(raw, "thinking").unwrap_or_default(),
        },
        "signature_delta" => ContentBlockDelta::SignatureDelta {
            signature: optional_string_field(raw, "signature").unwrap_or_default(),
        },
        _ => ContentBlockDelta::Unknown {
            delta_type,
            raw: raw.clone(),
        },
    }
}

fn parse_usage(raw: &Value) -> MessageUsage {
    MessageUsage {
        input_tokens: u32_field(raw, "input_tokens"),
        output_tokens: u32_field(raw, "output_tokens"),
        cache_creation_input_tokens: optional_u32_field(raw, "cache_creation_input_tokens"),
        cache_read_input_tokens: optional_u32_field(raw, "cache_read_input_tokens"),
        inference_geo: optional_string_field(raw, "inference_geo"),
        service_tier: optional_string_field(raw, "service_tier"),
        server_tool_use: raw.get("server_tool_use").map(parse_server_tool_usage),
    }
}

fn parse_message_delta_usage(raw: &Value) -> MessageDeltaUsage {
    MessageDeltaUsage {
        input_tokens: optional_u32_field(raw, "input_tokens"),
        output_tokens: optional_u32_field(raw, "output_tokens"),
        cache_creation_input_tokens: optional_u32_field(raw, "cache_creation_input_tokens"),
        cache_read_input_tokens: optional_u32_field(raw, "cache_read_input_tokens"),
        server_tool_use: raw.get("server_tool_use").map(parse_server_tool_usage),
    }
}

fn parse_server_tool_usage(raw: &Value) -> ServerToolUsage {
    ServerToolUsage {
        web_fetch_requests: u32_field(raw, "web_fetch_requests"),
        web_search_requests: u32_field(raw, "web_search_requests"),
    }
}

fn parse_stop_reason(value: &Value) -> Option<StopReason> {
    Some(match value.as_str()? {
        "end_turn" => StopReason::EndTurn,
        "max_tokens" => StopReason::MaxTokens,
        "stop_sequence" => StopReason::StopSequence,
        "tool_use" => StopReason::ToolUse,
        "pause_turn" => StopReason::PauseTurn,
        "refusal" => StopReason::Refusal,
        _ => StopReason::Unknown,
    })
}

fn optional_string_field(raw: &Value, key: &str) -> Option<String> {
    raw.get(key).and_then(Value::as_str).map(str::to_owned)
}

fn u32_field(raw: &Value, key: &str) -> u32 {
    raw.get(key).and_then(Value::as_u64).unwrap_or_default() as u32
}

fn optional_u32_field(raw: &Value, key: &str) -> Option<u32> {
    raw.get(key)
        .and_then(Value::as_u64)
        .map(|value| value as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_non_streaming_message_object() {
        let raw = json!({
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "content": [
                {
                    "type": "text",
                    "text": "hello"
                }
            ],
            "model": "claude-haiku-4-5",
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 10,
                "output_tokens": 5
            }
        });

        let parsed = parse_message_object_value(raw).unwrap();
        assert_eq!(parsed.id.as_deref(), Some("msg_123"));
        assert_eq!(parsed.stop_reason, Some(StopReason::EndTurn));
        assert!(matches!(parsed.content[0], ContentBlock::Text { .. }));
    }

    #[test]
    fn parses_content_block_delta_event() {
        let raw = json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": {
                "type": "text_delta",
                "text": "hel"
            }
        });

        let parsed = parse_message_stream_event(raw, None).unwrap();
        match parsed {
            MessageStreamEvent::ContentBlockDelta { index, delta } => {
                assert_eq!(index, 0);
                assert_eq!(delta, ContentBlockDelta::TextDelta { text: "hel".into() });
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn parses_error_event() {
        let raw = json!({
            "type": "error",
            "error": {
                "type": "overloaded_error",
                "message": "busy"
            }
        });

        let parsed = parse_message_stream_event(raw, None).unwrap();
        match parsed {
            MessageStreamEvent::Error { error, .. } => {
                assert_eq!(error.error_type.as_deref(), Some("overloaded_error"));
                assert_eq!(error.message.as_deref(), Some("busy"));
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }
}
