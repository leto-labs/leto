use std::convert::Infallible;
use std::time::Duration;

use agent_store::StoredMessage;
use axum::Json;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use futures::StreamExt;
use provider::{ContentBlock, Message, MessageRole};

use super::AppState;
use super::errors::{core_error_response, store_error_response};
use super::parse_session_id;
use crate::types::{BatchTurnRequest, ToolCallRequest, TurnRequest};

pub(super) async fn start_turn(
    State(server): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<TurnRequest>,
) -> Response {
    let session_id = match parse_session_id(&id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let core = server.core();
    match core.turn(session_id, body.input).await {
        Ok(stream) => {
            let ndjson = stream.map(|event| {
                let mut line = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_owned());
                line.push('\n');
                Ok::<_, Infallible>(line)
            });
            Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/x-ndjson")
                .body(Body::from_stream(ndjson))
                .unwrap()
        }
        Err(error) => core_error_response(error),
    }
}

pub(super) async fn start_turn_sse(
    State(server): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<TurnRequest>,
) -> Response {
    let session_id = match parse_session_id(&id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let core = server.core();
    match core.turn(session_id, body.input).await {
        Ok(stream) => {
            let sse = stream.map(|event| {
                let data = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_owned());
                Ok::<_, Infallible>(SseEvent::default().data(data))
            });
            Sse::new(sse)
                .keep_alive(KeepAlive::new().interval(Duration::from_secs(10)))
                .into_response()
        }
        Err(error) => core_error_response(error),
    }
}

pub(super) async fn start_batch_turns(
    State(server): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<BatchTurnRequest>,
) -> Response {
    let session_id = match parse_session_id(&id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let core = server.core();
    let mut lines = Vec::new();

    for turn in body.turns {
        let mut stream = match core.turn(session_id, turn.input).await {
            Ok(stream) => stream,
            Err(error) => return core_error_response(error),
        };
        while let Some(event) = stream.next().await {
            let mut line = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_owned());
            line.push('\n');
            lines.push(line);
        }
    }

    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/x-ndjson")
        .body(Body::from(lines.concat()))
        .unwrap()
}

pub(super) async fn append_tool_calls(
    State(server): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<ToolCallRequest>,
) -> Response {
    let session_id = match parse_session_id(&id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let core = server.core();
    let mut messages = match core.messages(session_id).await {
        Ok(messages) => messages,
        Err(error) => return core_error_response(error),
    };
    let starting_ordinal = messages.len() as u64;
    let appended = body
        .calls
        .into_iter()
        .enumerate()
        .flat_map(|(index, call)| {
            let ordinal = starting_ordinal + (index as u64 * 2);
            let tool_call_id = call.id.clone();
            let tool_call_input = call.input.clone();
            let tool_call_message = StoredMessage::new(
                session_id,
                ordinal,
                Message::new(
                    MessageRole::Assistant,
                    vec![ContentBlock::ToolCall {
                        id: tool_call_id.clone(),
                        name: call.name,
                        input: tool_call_input,
                    }],
                ),
            );
            let tool_result_message = StoredMessage::new(
                session_id,
                ordinal + 1,
                Message::new(
                    MessageRole::User,
                    vec![ContentBlock::ToolResult {
                        call_id: tool_call_id,
                        output: call.output,
                        is_error: Some(call.is_error),
                    }],
                ),
            );
            vec![tool_call_message, tool_result_message]
        })
        .collect::<Vec<_>>();

    messages.extend(appended.clone());
    match core
        .store()
        .messages()
        .replace_for_session(session_id, messages)
        .await
    {
        Ok(_) => Json(appended).into_response(),
        Err(error) => store_error_response(error),
    }
}

pub(super) async fn cancel_turn(
    State(server): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    let session_id = match parse_session_id(&id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let core = server.core();
    match core.cancel_turn(session_id).await {
        Ok(()) => StatusCode::OK.into_response(),
        Err(error) => core_error_response(error),
    }
}
