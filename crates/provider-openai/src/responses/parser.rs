use async_stream::stream;
use eventsource_stream::Eventsource;
use futures::StreamExt;
use serde_json::Value;

use crate::responses::client::ResponseStream;
use crate::responses::types::{
    ResponseCompaction, ResponseConversationRef, ResponseErrorInfo, ResponseEvent,
    ResponseIncompleteDetails, ResponseInputTokenCount, ResponseItemPage, ResponseMessageItem,
    ResponseObject, ResponseOutputContentPart, ResponseOutputItem, ResponsePageItem,
    ResponsePromptRef, ResponseReasoningItem, ResponseReasoningSummaryPart, ResponseStreamEvent,
    ResponseToolCallItem, ResponseToolKind, ResponseUnknownItem,
};
use crate::{Error, TokenUsage};

pub(crate) fn sse_stream_from_response(response: reqwest::Response) -> ResponseStream {
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

            if event.data == "[DONE]" {
                break;
            }

            let raw: Value = match serde_json::from_str(&event.data) {
                Ok(value) => value,
                Err(err) => {
                    yield Err(Error::Json(err));
                    break;
                }
            };

            match parse_response_stream_event(raw) {
                Ok(Some(mapped)) => {
                    if matches!(
                        mapped.event,
                        ResponseEvent::ResponseCompleted { .. }
                            | ResponseEvent::ResponseIncomplete { .. }
                            | ResponseEvent::ResponseFailed { .. }
                    ) {
                        saw_terminal = true;
                    }
                    yield Ok(mapped);
                }
                Ok(None) => {}
                Err(err) => {
                    yield Err(err);
                    break;
                }
            }
        }

        if !saw_terminal {
            yield Err(Error::Inference("stream closed before terminal response event".into()));
        }
    };

    Box::pin(s)
}

pub fn parse_response_object_value(raw: Value) -> ResponseObject {
    parse_response_object(Some(&raw.clone()), raw)
}

pub fn parse_response_compaction_value(raw: Value) -> ResponseCompaction {
    let output = raw
        .get("output")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|item| parse_output_item(Some(item)))
                .collect()
        })
        .unwrap_or_default();

    ResponseCompaction {
        id: optional_string_field(&raw, "id"),
        object: optional_string_field(&raw, "object"),
        created_at: raw.get("created_at").and_then(Value::as_i64),
        output,
        usage: parse_usage(raw.get("usage")),
        raw,
    }
}

pub fn parse_response_item_page_value(raw: Value) -> ResponseItemPage {
    ResponseItemPage {
        data: raw
            .get("data")
            .and_then(Value::as_array)
            .map(|items| items.iter().cloned().map(parse_response_page_item).collect())
            .unwrap_or_default(),
        first_id: optional_string_field(&raw, "first_id"),
        last_id: optional_string_field(&raw, "last_id"),
        has_more: raw
            .get("has_more")
            .and_then(Value::as_bool)
            .unwrap_or_default(),
        object: optional_string_field(&raw, "object"),
    }
}

pub fn parse_response_input_token_count_value(raw: Value) -> ResponseInputTokenCount {
    ResponseInputTokenCount {
        input_tokens: raw
            .get("input_tokens")
            .and_then(Value::as_u64)
            .unwrap_or_default() as u32,
        object: optional_string_field(&raw, "object"),
    }
}

pub fn parse_response_stream_event(raw: Value) -> Result<Option<ResponseStreamEvent>, Error> {
    let sequence_number = raw.get("sequence_number").and_then(Value::as_u64);
    let event_type = raw
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::Inference("responses event missing type".into()))?
        .to_owned();

    let event = match event_type.as_str() {
        "response.created" => ResponseEvent::ResponseCreated {
            response: parse_response_object(raw.get("response"), response_raw(&raw)),
        },
        "response.queued" => ResponseEvent::ResponseQueued {
            response: parse_response_object(raw.get("response"), response_raw(&raw)),
        },
        "response.in_progress" => ResponseEvent::ResponseInProgress {
            response: parse_response_object(raw.get("response"), response_raw(&raw)),
        },
        "response.completed" => ResponseEvent::ResponseCompleted {
            response: parse_response_object(raw.get("response"), response_raw(&raw)),
        },
        "response.incomplete" => ResponseEvent::ResponseIncomplete {
            response: parse_response_object(raw.get("response"), response_raw(&raw)),
        },
        "response.failed" => ResponseEvent::ResponseFailed {
            response: parse_response_object(raw.get("response"), response_raw(&raw)),
        },
        "error" => ResponseEvent::Error {
            error: ResponseErrorInfo {
                code: optional_string_field(&raw, "code"),
                message: optional_string_field(&raw, "message"),
                param: optional_string_field(&raw, "param"),
                event_id: optional_string_field(&raw, "event_id"),
            },
            raw,
        },
        "response.output_item.added" => ResponseEvent::OutputItemAdded {
            output_index: u32_field(&raw, "output_index"),
            item: parse_output_item(raw.get("item")),
        },
        "response.output_item.done" => ResponseEvent::OutputItemDone {
            output_index: u32_field(&raw, "output_index"),
            item: parse_output_item(raw.get("item")),
        },
        "response.content_part.added" => ResponseEvent::ContentPartAdded {
            item_id: string_field(&raw, "item_id"),
            output_index: u32_field(&raw, "output_index"),
            content_index: u32_field(&raw, "content_index"),
            part: parse_output_content_part(raw.get("part")),
        },
        "response.content_part.done" => ResponseEvent::ContentPartDone {
            item_id: string_field(&raw, "item_id"),
            output_index: u32_field(&raw, "output_index"),
            content_index: u32_field(&raw, "content_index"),
            part: parse_output_content_part(raw.get("part")),
        },
        "response.output_text.delta" => ResponseEvent::OutputTextDelta {
            item_id: string_field(&raw, "item_id"),
            output_index: u32_field(&raw, "output_index"),
            content_index: u32_field(&raw, "content_index"),
            delta: string_field(&raw, "delta"),
        },
        "response.output_text.done" => ResponseEvent::OutputTextDone {
            item_id: string_field(&raw, "item_id"),
            output_index: u32_field(&raw, "output_index"),
            content_index: u32_field(&raw, "content_index"),
            text: string_field(&raw, "text"),
        },
        "response.audio.delta" => ResponseEvent::OutputAudioDelta {
            delta: string_field(&raw, "delta"),
        },
        "response.audio.done" => ResponseEvent::OutputAudioDone,
        "response.audio.transcript.delta" => ResponseEvent::OutputAudioTranscriptDelta {
            delta: string_field(&raw, "delta"),
        },
        "response.audio.transcript.done" => ResponseEvent::OutputAudioTranscriptDone,
        "response.refusal.delta" => ResponseEvent::RefusalDelta {
            item_id: string_field(&raw, "item_id"),
            output_index: u32_field(&raw, "output_index"),
            content_index: u32_field(&raw, "content_index"),
            delta: string_field(&raw, "delta"),
        },
        "response.refusal.done" => ResponseEvent::RefusalDone {
            item_id: string_field(&raw, "item_id"),
            output_index: u32_field(&raw, "output_index"),
            content_index: u32_field(&raw, "content_index"),
            refusal: string_field(&raw, "refusal"),
        },
        "response.function_call_arguments.delta" => ResponseEvent::FunctionCallArgumentsDelta {
            item_id: string_field(&raw, "item_id"),
            output_index: u32_field(&raw, "output_index"),
            delta: string_field(&raw, "delta"),
        },
        "response.function_call_arguments.done" => ResponseEvent::FunctionCallArgumentsDone {
            item_id: string_field(&raw, "item_id"),
            output_index: u32_field(&raw, "output_index"),
            arguments: string_field(&raw, "arguments"),
        },
        "response.reasoning_summary_text.delta" => ResponseEvent::ReasoningSummaryTextDelta {
            item_id: string_field(&raw, "item_id"),
            output_index: u32_field(&raw, "output_index"),
            summary_index: u32_field(&raw, "summary_index"),
            delta: string_field(&raw, "delta"),
        },
        "response.reasoning_summary_text.done" => ResponseEvent::ReasoningSummaryTextDone {
            item_id: string_field(&raw, "item_id"),
            output_index: u32_field(&raw, "output_index"),
            summary_index: u32_field(&raw, "summary_index"),
            text: string_field(&raw, "text"),
        },
        "response.reasoning_summary_part.added" => ResponseEvent::ReasoningSummaryPartAdded {
            item_id: string_field(&raw, "item_id"),
            output_index: u32_field(&raw, "output_index"),
            summary_index: u32_field(&raw, "summary_index"),
            part: parse_reasoning_summary_part(raw.get("part")),
        },
        "response.reasoning_summary_part.done" => ResponseEvent::ReasoningSummaryPartDone {
            item_id: string_field(&raw, "item_id"),
            output_index: u32_field(&raw, "output_index"),
            summary_index: u32_field(&raw, "summary_index"),
            part: parse_reasoning_summary_part(raw.get("part")),
        },
        "response.web_search_call.searching" => ResponseEvent::WebSearchCallSearching {
            item_id: string_field(&raw, "item_id"),
            output_index: u32_field(&raw, "output_index"),
        },
        "response.code_interpreter_call.code.delta" => ResponseEvent::CodeInterpreterCodeDelta {
            item_id: string_field(&raw, "item_id"),
            output_index: u32_field(&raw, "output_index"),
            delta: string_field(&raw, "delta"),
        },
        "response.code_interpreter_call.code.done" => ResponseEvent::CodeInterpreterCodeDone {
            item_id: string_field(&raw, "item_id"),
            output_index: u32_field(&raw, "output_index"),
            code: string_field(&raw, "code"),
        },
        "response.file_search_call.searching" => ResponseEvent::FileSearchSearching {
            item_id: string_field(&raw, "item_id"),
            output_index: u32_field(&raw, "output_index"),
        },
        _ => ResponseEvent::Unknown { event_type, raw },
    };

    Ok(Some(ResponseStreamEvent {
        sequence_number,
        event,
    }))
}

fn response_raw(raw: &Value) -> Value {
    raw.get("response").cloned().unwrap_or(Value::Null)
}

fn parse_response_object(raw_ref: Option<&Value>, raw_owned: Value) -> ResponseObject {
    let raw = raw_ref.unwrap_or(&raw_owned);
    let output = raw
        .get("output")
        .and_then(Value::as_array)
        .map(|items| items.iter().map(|item| parse_output_item(Some(item))).collect())
        .unwrap_or_default();

    ResponseObject {
        id: optional_string_field(raw, "id"),
        object: optional_string_field(raw, "object"),
        created_at: raw.get("created_at").and_then(Value::as_i64),
        status: optional_string_field(raw, "status"),
        error: raw
            .get("error")
            .and_then(Value::as_object)
            .map(|_| ResponseErrorInfo {
                code: raw
                    .get("error")
                    .and_then(|err| err.get("code"))
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                message: raw
                    .get("error")
                    .and_then(|err| err.get("message"))
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                param: raw
                    .get("error")
                    .and_then(|err| err.get("param"))
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                event_id: None,
            }),
        incomplete_details: raw
            .get("incomplete_details")
            .map(|details| ResponseIncompleteDetails {
                reason: details
                    .get("reason")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            }),
        instructions: optional_string_field(raw, "instructions"),
        output_text: optional_string_field(raw, "output_text"),
        output,
        usage: parse_usage(raw.get("usage")),
        parallel_tool_calls: raw.get("parallel_tool_calls").and_then(Value::as_bool),
        prompt: raw.get("prompt").and_then(|prompt| {
            prompt.as_object().map(|_| ResponsePromptRef {
                id: prompt
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                version: prompt
                    .get("version")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                variables: prompt
                    .get("variables")
                    .and_then(Value::as_object)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .collect(),
            })
        }),
        conversation: raw.get("conversation").and_then(|conversation| {
            conversation.as_object().map(|_| ResponseConversationRef {
                id: conversation
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
            })
        }),
        service_tier: optional_string_field(raw, "service_tier"),
        raw: raw_owned,
    }
}

fn parse_output_item(raw: Option<&Value>) -> ResponseOutputItem {
    let raw = raw.cloned().unwrap_or(Value::Null);
    let type_name = raw.get("type").and_then(Value::as_str).unwrap_or_default();

    match type_name {
        "message" => ResponseOutputItem::Message(ResponseMessageItem {
            id: optional_string_field(&raw, "id"),
            status: optional_string_field(&raw, "status"),
            role: optional_string_field(&raw, "role"),
            content: raw
                .get("content")
                .and_then(Value::as_array)
                .map(|parts| parts.iter().map(|part| parse_output_content_part(Some(part))).collect())
                .unwrap_or_default(),
            raw,
        }),
        "reasoning" => ResponseOutputItem::Reasoning(ResponseReasoningItem {
            id: optional_string_field(&raw, "id"),
            summary: raw
                .get("summary")
                .and_then(Value::as_array)
                .map(|parts| parts.iter().map(|part| parse_reasoning_summary_part(Some(part))).collect())
                .unwrap_or_default(),
            raw,
        }),
        "function_call" | "web_search_call" | "file_search_call" | "code_interpreter_call" => {
            ResponseOutputItem::ToolCall(ResponseToolCallItem {
                id: optional_string_field(&raw, "id"),
                status: optional_string_field(&raw, "status"),
                tool_kind: parse_tool_kind(type_name),
                name: optional_string_field(&raw, "name"),
                call_id: optional_string_field(&raw, "call_id"),
                arguments: optional_string_field(&raw, "arguments"),
                raw,
            })
        }
        _ => ResponseOutputItem::Unknown(ResponseUnknownItem { raw }),
    }
}

fn parse_output_content_part(raw: Option<&Value>) -> ResponseOutputContentPart {
    let raw = raw.cloned().unwrap_or(Value::Null);
    let part_type = raw.get("type").and_then(Value::as_str).unwrap_or_default();

    match part_type {
        "output_text" => ResponseOutputContentPart::OutputText {
            text: optional_string_field(&raw, "text").unwrap_or_default(),
            annotations: raw
                .get("annotations")
                .cloned()
                .unwrap_or_else(|| Value::Array(Vec::new())),
        },
        "refusal" => ResponseOutputContentPart::Refusal {
            refusal: optional_string_field(&raw, "refusal").unwrap_or_default(),
        },
        "output_audio" => ResponseOutputContentPart::OutputAudio { raw },
        _ => ResponseOutputContentPart::Unknown { raw },
    }
}

fn parse_reasoning_summary_part(raw: Option<&Value>) -> ResponseReasoningSummaryPart {
    let raw = raw.cloned().unwrap_or(Value::Null);
    let part_type = raw.get("type").and_then(Value::as_str).unwrap_or_default();

    match part_type {
        "summary_text" => ResponseReasoningSummaryPart::SummaryText {
            text: optional_string_field(&raw, "text").unwrap_or_default(),
        },
        _ => ResponseReasoningSummaryPart::Unknown { raw },
    }
}

fn parse_response_page_item(raw: Value) -> ResponsePageItem {
    ResponsePageItem {
        id: optional_string_field(&raw, "id"),
        item_type: optional_string_field(&raw, "type"),
        raw,
    }
}

fn parse_tool_kind(kind: &str) -> ResponseToolKind {
    match kind {
        "function_call" => ResponseToolKind::Function,
        "web_search_call" => ResponseToolKind::WebSearch,
        "file_search_call" => ResponseToolKind::FileSearch,
        "code_interpreter_call" => ResponseToolKind::CodeInterpreter,
        _ => ResponseToolKind::Unknown(kind.to_owned()),
    }
}

fn parse_usage(raw: Option<&Value>) -> Option<TokenUsage> {
    raw.map(|raw| TokenUsage {
        prompt: raw
            .get("input_tokens")
            .or_else(|| raw.get("prompt_tokens"))
            .and_then(Value::as_u64)
            .unwrap_or_default() as u32,
        completion: raw
            .get("output_tokens")
            .or_else(|| raw.get("completion_tokens"))
            .and_then(Value::as_u64)
            .unwrap_or_default() as u32,
        total: raw
            .get("total_tokens")
            .and_then(Value::as_u64)
            .unwrap_or_default() as u32,
        cache_read: raw
            .get("input_tokens_details")
            .and_then(|details| details.get("cached_tokens"))
            .and_then(Value::as_u64)
            .map(|n| n as u32)
            .or_else(|| {
                raw.get("prompt_tokens_details")
                    .and_then(|details| details.get("cached_tokens"))
                    .and_then(Value::as_u64)
                    .map(|n| n as u32)
            }),
        cache_write: raw
            .get("input_tokens_details")
            .and_then(|details| details.get("cache_creation_tokens"))
            .and_then(Value::as_u64)
            .map(|n| n as u32)
            .or_else(|| {
                raw.get("prompt_tokens_details")
                    .and_then(|details| details.get("cache_creation_tokens"))
                    .and_then(Value::as_u64)
                    .map(|n| n as u32)
            }),
        reasoning: raw
            .get("output_tokens_details")
            .and_then(|details| details.get("reasoning_tokens"))
            .and_then(Value::as_u64)
            .map(|n| n as u32)
            .or_else(|| {
                raw.get("completion_tokens_details")
                    .and_then(|details| details.get("reasoning_tokens"))
                    .and_then(Value::as_u64)
                    .map(|n| n as u32)
            }),
    })
}

fn optional_string_field(raw: &Value, field: &str) -> Option<String> {
    raw.get(field).and_then(Value::as_str).map(str::to_owned)
}

fn string_field(raw: &Value, field: &str) -> String {
    optional_string_field(raw, field).unwrap_or_default()
}

fn u32_field(raw: &Value, field: &str) -> u32 {
    raw.get(field).and_then(Value::as_u64).unwrap_or_default() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_output_text_delta_event() {
        let raw = serde_json::json!({
            "sequence_number": 5,
            "type": "response.output_text.delta",
            "item_id": "item_1",
            "output_index": 0,
            "content_index": 0,
            "delta": "hel"
        });

        let parsed = parse_response_stream_event(raw)
            .unwrap()
            .expect("event should parse");

        assert_eq!(parsed.sequence_number, Some(5));
        match parsed.event {
            ResponseEvent::OutputTextDelta { delta, .. } => assert_eq!(delta, "hel"),
            other => panic!("unexpected event: {other:?}"),
        }
    }
}
