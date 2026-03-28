use std::convert::Infallible;
use std::time::Duration;

use agent_core::CoreEvent;
use axum::extract::{Path, State};
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use axum::response::{IntoResponse, Json, Response};
use futures::StreamExt;

use super::super::errors::store_error_response;
use super::super::{AppState, parse_session_id};
use crate::types::{AgentInfoRecord, HealthResponse};

pub(in crate::http) async fn health() -> impl IntoResponse {
    Json(HealthResponse::current())
}

pub(in crate::http) async fn status(State(server): State<AppState>) -> Response {
    match server.status().await {
        Ok(status) => Json(status).into_response(),
        Err(error) => store_error_response(error),
    }
}

pub(in crate::http) async fn list_agents(State(server): State<AppState>) -> Response {
    Json::<Vec<AgentInfoRecord>>(server.agent_info()).into_response()
}

pub(in crate::http) async fn list_mcp_servers(State(server): State<AppState>) -> Response {
    Json::<Vec<AgentInfoRecord>>(server.agent_info()).into_response()
}

pub(in crate::http) async fn events(
    State(server): State<AppState>,
) -> Sse<impl futures::Stream<Item = Result<SseEvent, Infallible>>> {
    let core = server.core();
    let stream = core.subscribe().map(|event| {
        let data = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_owned());
        Ok::<_, Infallible>(SseEvent::default().data(data))
    });
    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(10)))
}

pub(in crate::http) async fn session_events(
    State(server): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    let session_id = match parse_session_id(&id) {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    let core = server.core();
    let stream = core.subscribe().filter_map(move |event| {
        let include = matches!(
            &event,
            CoreEvent::Turn {
                session_id: event_session_id,
                ..
            }
            | CoreEvent::TurnCancelled {
                session_id: event_session_id,
            } if *event_session_id == session_id
        );
        async move {
            if !include {
                return None;
            }
            let data = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_owned());
            Some(Ok::<_, Infallible>(SseEvent::default().data(data)))
        }
    });
    Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(10)))
        .into_response()
}
