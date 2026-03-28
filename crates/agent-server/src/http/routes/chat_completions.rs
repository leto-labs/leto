use std::sync::Arc;

use agent_core::{AgentCore, CoreError};
use agent_runtime::RuntimeEvent;
use agent_store::{Project, ProjectId, SessionId, SessionUpdate};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use futures::StreamExt;
use provider::{ContentBlock, FinishReason, Message, MessageRole, Usage};
use provider_openai::{
    ChatCompletionChoice, ChatCompletionFunctionCall, ChatCompletionMessage,
    ChatCompletionMessageContent, ChatCompletionObject, ChatCompletionRequest, ChatCompletionRole,
    ChatCompletionToolCall, TokenUsage,
};
use serde_json::json;

use super::super::AppState;
use super::super::errors::{core_error_response, store_error_response};
use crate::types::ErrorResponse;

#[derive(Debug, Clone, Copy)]
struct ChatCompletionRequestError {
    code: &'static str,
    message: &'static str,
}

impl ChatCompletionRequestError {
    const fn new(code: &'static str, message: &'static str) -> Self {
        Self { code, message }
    }
}

impl IntoResponse for ChatCompletionRequestError {
    fn into_response(self) -> Response {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(self.code, self.message)),
        )
            .into_response()
    }
}

pub(in crate::http) async fn create_chat_completion(
    State(server): State<AppState>,
    Json(body): Json<ChatCompletionRequest>,
) -> Response {
    if body
        .extra
        .get("stream")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "stream_not_supported",
                "streaming chat completions are not supported",
            )),
        )
            .into_response();
    }

    let input = match chat_completion_messages_to_input(&body.messages) {
        Ok(messages) => messages,
        Err(response) => return response.into_response(),
    };

    let core = server.core();
    let project = match core
        .store()
        .projects()
        .create(Project::new(
            Some("chat-completions".into()),
            None,
            Default::default(),
        ))
        .await
    {
        Ok(project) => project,
        Err(error) => return store_error_response(error),
    };
    let session = match core.create_session(project.id).await {
        Ok(session) => session,
        Err(error) => {
            let _ = core.store().projects().delete(project.id).await;
            return core_error_response(error);
        }
    };

    if body.model.is_some() {
        let update = SessionUpdate {
            model: Some(body.model.clone()),
            ..SessionUpdate::default()
        };
        if let Err(error) = core.update_session(session.id, update).await {
            cleanup_chat_completion_session(&core, project.id, session.id).await;
            return core_error_response(error);
        }
    }

    let turn_result = core.turn(session.id, input).await;
    let response = match turn_result {
        Ok(mut stream) => {
            let mut finish_reason = None;
            let mut usage = None;

            while let Some(event) = stream.next().await {
                let agent_core::CoreEvent::Turn { event, .. } = event else {
                    continue;
                };
                match event {
                    RuntimeEvent::Usage { usage: current } => usage = Some(current),
                    RuntimeEvent::TurnFinished {
                        finish_reason: current,
                        ..
                    } => {
                        finish_reason = current;
                        break;
                    }
                    RuntimeEvent::Error { message, .. } => {
                        cleanup_chat_completion_session(&core, project.id, session.id).await;
                        return (
                            StatusCode::BAD_GATEWAY,
                            Json(ErrorResponse::new("runtime_error", message)),
                        )
                            .into_response();
                    }
                    _ => {}
                }
            }

            let completion = match build_chat_completion_response(
                &core,
                session.id,
                body.model.clone(),
                finish_reason,
                usage,
            )
            .await
            {
                Ok(completion) => completion,
                Err(error) => {
                    cleanup_chat_completion_session(&core, project.id, session.id).await;
                    return core_error_response(error);
                }
            };

            Json(completion).into_response()
        }
        Err(error) => core_error_response(error),
    };

    cleanup_chat_completion_session(&core, project.id, session.id).await;
    response
}

async fn cleanup_chat_completion_session(
    core: &Arc<dyn AgentCore>,
    project_id: ProjectId,
    session_id: SessionId,
) {
    let _ = core.delete_session(session_id).await;
    let _ = core.store().projects().delete(project_id).await;
}

fn chat_completion_messages_to_input(
    messages: &[ChatCompletionMessage],
) -> Result<Vec<Message>, ChatCompletionRequestError> {
    messages
        .iter()
        .map(chat_completion_message_to_input_message)
        .collect()
}

fn chat_completion_message_to_input_message(
    message: &ChatCompletionMessage,
) -> Result<Message, ChatCompletionRequestError> {
    let role = match message.role {
        ChatCompletionRole::System => MessageRole::System,
        ChatCompletionRole::Developer => MessageRole::Developer,
        ChatCompletionRole::User => MessageRole::User,
        ChatCompletionRole::Assistant => MessageRole::Assistant,
        ChatCompletionRole::Tool => MessageRole::Assistant,
    };

    let mut content = chat_completion_content_to_blocks(message.content.as_ref())?;
    for tool_call in &message.tool_calls {
        let input = serde_json::from_str(&tool_call.function.arguments).map_err(|_| {
            ChatCompletionRequestError::new(
                "invalid_tool_arguments",
                "tool call arguments must be valid JSON",
            )
        })?;
        content.push(ContentBlock::tool_call(
            tool_call.id.clone(),
            tool_call.function.name.clone(),
            input,
        ));
    }

    if matches!(message.role, ChatCompletionRole::Tool) {
        let call_id = message.tool_call_id.clone().ok_or_else(|| {
            ChatCompletionRequestError::new(
                "missing_tool_call_id",
                "tool messages must include tool_call_id",
            )
        })?;
        let output = serde_json::Value::String(chat_message_text_lossy(message));
        content.push(ContentBlock::tool_result(call_id, output));
    }

    Ok(Message::new(role, content))
}

fn chat_completion_content_to_blocks(
    content: Option<&ChatCompletionMessageContent>,
) -> Result<Vec<ContentBlock>, ChatCompletionRequestError> {
    let Some(content) = content else {
        return Ok(Vec::new());
    };

    match content {
        ChatCompletionMessageContent::Text(text) => Ok(vec![ContentBlock::text(text.clone())]),
        ChatCompletionMessageContent::Parts(parts) => parts
            .iter()
            .map(|part| match part {
                provider_openai::ChatCompletionContentPart::Text { text } => {
                    Ok(ContentBlock::text(text.clone()))
                }
                provider_openai::ChatCompletionContentPart::ImageUrl { image_url } => {
                    Ok(ContentBlock::image_url(image_url.url.clone()))
                }
            })
            .collect(),
    }
}

async fn build_chat_completion_response(
    core: &Arc<dyn AgentCore>,
    session_id: SessionId,
    requested_model: Option<String>,
    finish_reason: Option<FinishReason>,
    usage: Option<Usage>,
) -> Result<ChatCompletionObject, CoreError> {
    let messages = core.messages(session_id).await?;
    let Some(message) = messages
        .iter()
        .rev()
        .find(|stored| stored.message.role == MessageRole::Assistant)
    else {
        return Err(CoreError::Internal(
            "chat completion turn produced no assistant message".into(),
        ));
    };

    let chat_message = output_message_to_chat_completion(&message.message);
    let finish_reason = finish_reason.map(chat_finish_reason);
    let usage = usage.map(chat_completion_usage);
    let model = match requested_model {
        Some(model) => Some(model),
        None => core.current_model_id_for_session(session_id).await?,
    };
    let response_id = format!("chatcmpl-{session_id}");
    let created = chrono::Utc::now().timestamp();
    let raw = json!({
        "id": response_id,
        "object": "chat.completion",
        "created": created,
        "model": model,
        "choices": [
            {
                "index": 0,
                "message": chat_message,
                "finish_reason": finish_reason,
            }
        ],
        "usage": usage,
    });

    Ok(ChatCompletionObject {
        id: Some(format!("chatcmpl-{session_id}")),
        object: Some("chat.completion".into()),
        created: Some(created),
        model,
        choices: vec![ChatCompletionChoice {
            index: 0,
            message: chat_message,
            finish_reason,
        }],
        usage,
        raw,
    })
}

fn output_message_to_chat_completion(message: &Message) -> ChatCompletionMessage {
    let text = message.plain_text_lossy();
    let content = (!text.is_empty()).then_some(ChatCompletionMessageContent::Text(text));
    let tool_calls = message
        .content
        .iter()
        .filter_map(|block| match block {
            ContentBlock::ToolCall { id, name, input } => Some(ChatCompletionToolCall {
                id: id.clone(),
                call_type: "function".to_owned(),
                function: ChatCompletionFunctionCall {
                    name: name.clone(),
                    arguments: input.to_string(),
                },
            }),
            _ => None,
        })
        .collect::<Vec<_>>();

    ChatCompletionMessage {
        role: ChatCompletionRole::Assistant,
        content,
        name: None,
        tool_calls,
        tool_call_id: None,
        extra: Default::default(),
    }
}

fn chat_finish_reason(reason: FinishReason) -> String {
    match reason {
        FinishReason::Stop => "stop".into(),
        FinishReason::MaxTokens => "length".into(),
        FinishReason::StopSequence => "stop".into(),
        FinishReason::ToolCall => "tool_calls".into(),
        FinishReason::Pause => "pause".into(),
        FinishReason::Refusal => "content_filter".into(),
        FinishReason::Incomplete => "incomplete".into(),
        FinishReason::Error => "error".into(),
        FinishReason::Unknown(reason) => reason,
    }
}

fn chat_completion_usage(usage: Usage) -> TokenUsage {
    TokenUsage {
        prompt: usage.input_tokens.unwrap_or_default(),
        completion: usage.output_tokens.unwrap_or_default(),
        total: usage.total_tokens.unwrap_or_default(),
        cache_read: usage.cache_read_tokens,
        cache_write: usage.cache_write_tokens,
        reasoning: usage.reasoning_tokens,
    }
}

fn chat_message_text_lossy(message: &ChatCompletionMessage) -> String {
    match message.content.as_ref() {
        Some(ChatCompletionMessageContent::Text(text)) => text.clone(),
        Some(ChatCompletionMessageContent::Parts(parts)) => parts
            .iter()
            .filter_map(|part| match part {
                provider_openai::ChatCompletionContentPart::Text { text } => Some(text.as_str()),
                provider_openai::ChatCompletionContentPart::ImageUrl { .. } => None,
            })
            .collect::<Vec<_>>()
            .join("\n"),
        None => String::new(),
    }
}
