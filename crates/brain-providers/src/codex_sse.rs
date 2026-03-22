use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use tracing::warn;

use brain_types::{ChatChunk, ChatStream, InferenceConfig, Message, Role, TokenUsage};

const DEFAULT_INSTRUCTIONS: &str = "You are a concise and helpful coding assistant.";

#[derive(Debug, Serialize)]
pub(crate) struct ResponsesRequest {
    pub model: String,
    pub input: Vec<ResponsesInputItem>,
    pub instructions: String,
    pub store: bool,
    pub stream: bool,
    pub text: TextOptions,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<ReasoningOptions>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub include: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ResponsesTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub(crate) enum ResponsesInputItem {
    #[serde(rename = "message")]
    Message {
        role: String,
        content: Vec<ResponsesInputContent>,
    },
    #[serde(rename = "function_call")]
    FunctionCall {
        id: String,
        call_id: String,
        name: String,
        arguments: String,
    },
    #[serde(rename = "function_call_output")]
    FunctionCallOutput { call_id: String, output: String },
}

#[derive(Debug, Serialize)]
pub(crate) struct ResponsesInputContent {
    #[serde(rename = "type")]
    pub kind: String,
    pub text: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct TextOptions {
    pub verbosity: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct ReasoningOptions {
    pub effort: String,
    pub summary: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct ResponsesTool {
    #[serde(rename = "type")]
    pub kind: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct SseEvent {
    #[serde(rename = "type")]
    event_type: Option<String>,
    #[serde(default)]
    delta: Option<String>,
    #[serde(default)]
    item: Option<ResponseOutputItem>,
    #[serde(default)]
    response: Option<CompletedResponse>,
}

#[derive(Debug, Deserialize)]
struct ResponseOutputItem {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CompletedResponse {
    #[serde(default)]
    usage: Option<UsageInfo>,
}

#[derive(Debug, Deserialize)]
struct UsageInfo {
    #[serde(default)]
    input_tokens: u32,
    #[serde(default)]
    input_tokens_details: Option<InputTokenDetails>,
    #[serde(default)]
    output_tokens: u32,
    #[serde(default)]
    output_tokens_details: Option<OutputTokenDetails>,
    #[serde(default)]
    total_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct InputTokenDetails {
    #[serde(default)]
    cached_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct OutputTokenDetails {
    #[serde(default)]
    reasoning_tokens: Option<u32>,
}

/// Convert our internal `Message` slice into Responses API (`instructions`, `input`).
pub(crate) fn build_responses_input(messages: &[Message]) -> (String, Vec<ResponsesInputItem>) {
    let mut system_parts: Vec<&str> = Vec::new();
    let mut input: Vec<ResponsesInputItem> = Vec::new();

    for msg in messages {
        match msg.role {
            Role::System => system_parts.push(&msg.content),
            Role::User => {
                input.push(ResponsesInputItem::Message {
                    role: "user".to_owned(),
                    content: vec![ResponsesInputContent {
                        kind: "input_text".to_owned(),
                        text: msg.content.clone(),
                    }],
                });
            }
            Role::Assistant => {
                if !msg.content.is_empty() {
                    input.push(ResponsesInputItem::Message {
                        role: "assistant".to_owned(),
                        content: vec![ResponsesInputContent {
                            kind: "output_text".to_owned(),
                            text: msg.content.clone(),
                        }],
                    });
                }
                for tool_call in &msg.tool_calls {
                    input.push(ResponsesInputItem::FunctionCall {
                        id: tool_call.id.clone(),
                        call_id: tool_call.id.clone(),
                        name: tool_call.name.clone(),
                        arguments: tool_call.arguments.to_string(),
                    });
                }
            }
            Role::Tool => {
                if let Some(call_id) = &msg.tool_call_id {
                    input.push(ResponsesInputItem::FunctionCallOutput {
                        call_id: call_id.clone(),
                        output: msg.content.clone(),
                    });
                }
            }
        }
    }

    let instructions = if system_parts.is_empty() {
        DEFAULT_INSTRUCTIONS.to_owned()
    } else {
        system_parts.join("\n\n")
    };

    (instructions, input)
}

pub(crate) fn build_responses_request(
    model: String,
    messages: &[Message],
    tools: &[brain_types::ToolDef],
    config: &InferenceConfig,
) -> ResponsesRequest {
    let (instructions, input) = build_responses_input(messages);
    let response_tools = to_responses_tools(tools);

    ResponsesRequest {
        model,
        input,
        instructions,
        store: false,
        stream: true,
        text: TextOptions {
            verbosity: "medium".to_owned(),
        },
        reasoning: config
            .reasoning
            .as_deref()
            .map(|reasoning| ReasoningOptions {
                effort: normalize_reasoning_effort(Some(reasoning)),
                summary: "auto".to_owned(),
            }),
        include: Vec::new(),
        tools: response_tools,
        tool_choice: if tools.is_empty() {
            None
        } else {
            Some("auto".to_owned())
        },
        parallel_tool_calls: if tools.is_empty() { None } else { Some(true) },
    }
}

fn normalize_reasoning_effort(reasoning: Option<&str>) -> String {
    match reasoning.unwrap_or("medium") {
        "minimal" => "low".to_owned(),
        "low" | "medium" | "high" => reasoning.unwrap_or("medium").to_owned(),
        _ => "medium".to_owned(),
    }
}

/// Convert `ToolDef` slice to Responses API tool format.
pub(crate) fn to_responses_tools(tools: &[brain_types::ToolDef]) -> Vec<ResponsesTool> {
    tools
        .iter()
        .map(|t| ResponsesTool {
            kind: "function".to_owned(),
            name: t.name.clone(),
            description: Some(t.description.clone()),
            parameters: Some(t.parameters.clone()),
        })
        .collect()
}

/// Parse the Responses API SSE stream from a `reqwest::Response` into a `ChatStream`.
pub(crate) fn stream_from_response(response: reqwest::Response) -> ChatStream {
    let byte_stream = response.bytes_stream();
    let mut buffer = String::new();

    let event_stream = byte_stream.flat_map(move |result| {
        let bytes = match result {
            Ok(b) => b,
            Err(e) => {
                warn!(error = %e, "SSE stream read error");
                return stream::iter(vec![]);
            }
        };

        buffer.push_str(&String::from_utf8_lossy(&bytes));

        let mut chunks: Vec<Result<ChatChunk, brain_types::BrainError>> = Vec::new();

        while let Some(idx) = buffer.find("\n\n") {
            let raw_event = buffer[..idx].to_string();
            buffer = buffer[idx + 2..].to_string();

            let data = extract_sse_data(&raw_event);
            if data.is_empty() || data == "[DONE]" {
                continue;
            }

            match serde_json::from_str::<SseEvent>(&data) {
                Ok(event) => {
                    if let Some(chunk) = event_to_chunk(&event) {
                        chunks.push(Ok(chunk));
                    }
                }
                Err(_) => {}
            }
        }

        stream::iter(chunks)
    });

    Box::pin(event_stream)
}

fn extract_sse_data(raw: &str) -> String {
    let mut parts = Vec::new();
    for line in raw.lines() {
        if let Some(data) = line.strip_prefix("data:") {
            parts.push(data.trim());
        } else if let Some(data) = line.strip_prefix("data: ") {
            parts.push(data.trim());
        }
    }
    parts.join("\n")
}

fn event_to_chunk(event: &SseEvent) -> Option<ChatChunk> {
    let event_type = event.event_type.as_deref()?;

    match event_type {
        "response.output_text.delta" => {
            let text = event.delta.as_deref().unwrap_or("");
            if text.is_empty() {
                return None;
            }
            Some(ChatChunk::Delta {
                content: text.to_owned(),
            })
        }
        "response.output_item.done" => {
            let item = event.item.as_ref()?;
            if item.kind != "function_call" {
                return None;
            }

            let name = item.name.as_ref()?;
            let args_str = item.arguments.as_deref().unwrap_or("{}");
            let args: serde_json::Value = serde_json::from_str(args_str)
                .unwrap_or(serde_json::Value::Object(Default::default()));

            Some(ChatChunk::ToolCall {
                id: item.id.clone(),
                name: name.clone(),
                arguments: args,
            })
        }
        "response.completed" => {
            let usage = event.response.as_ref().and_then(|r| {
                r.usage.as_ref().map(|u| TokenUsage {
                    prompt: u.input_tokens,
                    completion: u.output_tokens,
                    total: u.total_tokens,
                    cache_read: u
                        .input_tokens_details
                        .as_ref()
                        .and_then(|details| details.cached_tokens),
                    cache_write: None,
                    reasoning: u
                        .output_tokens_details
                        .as_ref()
                        .and_then(|details| details.reasoning_tokens),
                })
            });
            Some(ChatChunk::Done { usage })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_input_extracts_system_as_instructions() {
        let msgs = vec![
            Message::system("Be helpful."),
            Message::user("Hi"),
            Message::assistant("Hello!"),
            Message::user("Thanks"),
        ];
        let (instructions, input) = build_responses_input(&msgs);
        assert_eq!(instructions, "Be helpful.");
        assert_eq!(input.len(), 3);
        match &input[0] {
            ResponsesInputItem::Message { role, content } => {
                assert_eq!(role, "user");
                assert_eq!(content[0].kind, "input_text");
            }
            other => panic!("expected user message, got {other:?}"),
        }
        match &input[1] {
            ResponsesInputItem::Message { role, content } => {
                assert_eq!(role, "assistant");
                assert_eq!(content[0].kind, "output_text");
            }
            other => panic!("expected assistant message, got {other:?}"),
        }
        match &input[2] {
            ResponsesInputItem::Message { role, .. } => assert_eq!(role, "user"),
            other => panic!("expected user message, got {other:?}"),
        }
    }

    #[test]
    fn build_input_uses_default_instructions_when_no_system() {
        let msgs = vec![Message::user("Hello")];
        let (instructions, input) = build_responses_input(&msgs);
        assert_eq!(instructions, DEFAULT_INSTRUCTIONS);
        assert_eq!(input.len(), 1);
    }

    #[test]
    fn build_input_preserves_tool_history() {
        let mut assistant = Message::assistant("");
        assistant.tool_calls.push(brain_types::ToolCall {
            id: "call_123".into(),
            name: "file_read".into(),
            arguments: serde_json::json!({ "path": "/tmp/test.txt" }),
        });
        let tool = Message::tool_result("call_123", "hello");

        let (_instructions, input) = build_responses_input(&[assistant, tool]);
        assert_eq!(input.len(), 2);

        match &input[0] {
            ResponsesInputItem::FunctionCall {
                call_id,
                name,
                arguments,
                ..
            } => {
                assert_eq!(call_id, "call_123");
                assert_eq!(name, "file_read");
                assert!(arguments.contains("/tmp/test.txt"));
            }
            other => panic!("expected function_call, got {other:?}"),
        }

        match &input[1] {
            ResponsesInputItem::FunctionCallOutput { call_id, output } => {
                assert_eq!(call_id, "call_123");
                assert_eq!(output, "hello");
            }
            other => panic!("expected function_call_output, got {other:?}"),
        }
    }

    #[test]
    fn extract_sse_data_parses_multiline() {
        let raw = "event: delta\ndata: {\"type\":\"response.output_text.delta\",\"delta\":\"hi\"}";
        let data = extract_sse_data(raw);
        assert!(data.contains("response.output_text.delta"));
    }

    #[test]
    fn event_to_chunk_handles_text_delta() {
        let event = SseEvent {
            event_type: Some("response.output_text.delta".into()),
            delta: Some("hello".into()),
            item: None,
            response: None,
        };
        match event_to_chunk(&event) {
            Some(ChatChunk::Delta { content }) => assert_eq!(content, "hello"),
            other => panic!("expected Delta, got {other:?}"),
        }
    }

    #[test]
    fn event_to_chunk_handles_completed() {
        let event = SseEvent {
            event_type: Some("response.completed".into()),
            delta: None,
            item: None,
            response: Some(CompletedResponse {
                usage: Some(UsageInfo {
                    input_tokens: 10,
                    input_tokens_details: Some(InputTokenDetails {
                        cached_tokens: Some(3),
                    }),
                    output_tokens: 20,
                    output_tokens_details: Some(OutputTokenDetails {
                        reasoning_tokens: Some(4),
                    }),
                    total_tokens: 30,
                }),
            }),
        };
        match event_to_chunk(&event) {
            Some(ChatChunk::Done { usage: Some(u) }) => {
                assert_eq!(u.prompt, 10);
                assert_eq!(u.completion, 20);
                assert_eq!(u.total, 30);
                assert_eq!(u.cache_read, Some(3));
                assert_eq!(u.reasoning, Some(4));
            }
            other => panic!("expected Done, got {other:?}"),
        }
    }

    #[test]
    fn event_to_chunk_handles_function_call_output_item() {
        let event = SseEvent {
            event_type: Some("response.output_item.done".into()),
            delta: None,
            item: Some(ResponseOutputItem {
                id: "fc_123".into(),
                kind: "function_call".into(),
                name: Some("file_write".into()),
                arguments: Some(r#"{"path":"hello.txt"}"#.into()),
            }),
            response: None,
        };

        match event_to_chunk(&event) {
            Some(ChatChunk::ToolCall {
                id,
                name,
                arguments,
            }) => {
                assert_eq!(id, "fc_123");
                assert_eq!(name, "file_write");
                assert_eq!(arguments["path"], "hello.txt");
            }
            other => panic!("expected ToolCall, got {other:?}"),
        }
    }
}
