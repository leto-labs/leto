use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use tracing::warn;

use brain_types::{ChatChunk, ChatStream, Message, Role, TokenUsage};

const DEFAULT_INSTRUCTIONS: &str = "You are a concise and helpful coding assistant.";

#[derive(Debug, Serialize)]
pub(crate) struct ResponsesRequest {
    pub model: String,
    pub input: Vec<ResponsesInput>,
    pub instructions: String,
    pub store: bool,
    pub stream: bool,
    pub text: TextOptions,
    pub reasoning: ReasoningOptions,
    pub include: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ResponsesTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ResponsesInput {
    pub role: String,
    pub content: Vec<ResponsesInputContent>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ResponsesInputContent {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
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
    name: Option<String>,
    #[serde(default)]
    call_id: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
    #[serde(default)]
    response: Option<CompletedResponse>,
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
    output_tokens: u32,
    #[serde(default)]
    total_tokens: u32,
}

/// Convert our internal `Message` slice into Responses API (`instructions`, `input`).
pub(crate) fn build_responses_input(messages: &[Message]) -> (String, Vec<ResponsesInput>) {
    let mut system_parts: Vec<&str> = Vec::new();
    let mut input: Vec<ResponsesInput> = Vec::new();

    for msg in messages {
        match msg.role {
            Role::System => system_parts.push(&msg.content),
            Role::User => {
                input.push(ResponsesInput {
                    role: "user".to_owned(),
                    content: vec![ResponsesInputContent {
                        kind: "input_text".to_owned(),
                        text: Some(msg.content.clone()),
                    }],
                });
            }
            Role::Assistant => {
                input.push(ResponsesInput {
                    role: "assistant".to_owned(),
                    content: vec![ResponsesInputContent {
                        kind: "output_text".to_owned(),
                        text: Some(msg.content.clone()),
                    }],
                });
            }
            Role::Tool => {
                // Tool results aren't directly supported in the Responses API input format.
                // Skip for now — the agent loop handles tool orchestration at a higher level.
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
        "response.function_call_arguments.done" => {
            let name = event.name.clone().unwrap_or_default();
            let call_id = event.call_id.clone().unwrap_or_default();
            let args_str = event.arguments.as_deref().unwrap_or("{}");
            let args: serde_json::Value =
                serde_json::from_str(args_str).unwrap_or(serde_json::Value::Object(Default::default()));
            Some(ChatChunk::ToolCall {
                id: call_id,
                name,
                arguments: args,
            })
        }
        "response.completed" => {
            let usage = event.response.as_ref().and_then(|r| {
                r.usage.as_ref().map(|u| TokenUsage {
                    prompt: u.input_tokens,
                    completion: u.output_tokens,
                    total: u.total_tokens,
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
        assert_eq!(input[0].role, "user");
        assert_eq!(input[0].content[0].kind, "input_text");
        assert_eq!(input[1].role, "assistant");
        assert_eq!(input[1].content[0].kind, "output_text");
        assert_eq!(input[2].role, "user");
    }

    #[test]
    fn build_input_uses_default_instructions_when_no_system() {
        let msgs = vec![Message::user("Hello")];
        let (instructions, input) = build_responses_input(&msgs);
        assert_eq!(instructions, DEFAULT_INSTRUCTIONS);
        assert_eq!(input.len(), 1);
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
            name: None,
            call_id: None,
            arguments: None,
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
            name: None,
            call_id: None,
            arguments: None,
            response: Some(CompletedResponse {
                usage: Some(UsageInfo {
                    input_tokens: 10,
                    output_tokens: 20,
                    total_tokens: 30,
                }),
            }),
        };
        match event_to_chunk(&event) {
            Some(ChatChunk::Done { usage: Some(u) }) => {
                assert_eq!(u.prompt, 10);
                assert_eq!(u.completion, 20);
                assert_eq!(u.total, 30);
            }
            other => panic!("expected Done, got {other:?}"),
        }
    }
}
