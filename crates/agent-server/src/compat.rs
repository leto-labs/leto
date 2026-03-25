use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use agent_core::AgentCore;
use agent_store::StoredMessage;
use axum::extract::{Path, State};
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use axum::response::{IntoResponse, Json, Response};
use axum::{Router, routing};
use futures::StreamExt;
use futures::stream;
use serde_json::{Value, json};
use ulid::Ulid;

use crate::http::{invalid_request, parse_session_id, stub_response};
use crate::server::AgentServer;
use crate::types::{ErrorResponse, HealthResponse};

type AppState = Arc<AgentServer>;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/global/health", routing::get(global_health))
        .route("/global/event", routing::get(global_event))
        .route("/project", routing::get(projects))
        .route("/project/current", routing::get(project_current))
        .route("/config", routing::get(config_get))
        .route("/config/providers", routing::get(config_providers))
        .route("/provider", routing::get(providers))
        .route("/session", routing::get(sessions).post(session_create))
        .route("/session/status", routing::get(session_status))
        .route(
            "/session/{id}",
            routing::get(session_get)
                .patch(session_update)
                .delete(session_delete),
        )
        .route("/session/{id}/message", routing::get(session_messages))
        .route(
            "/session/{id}/message/{message_id}",
            routing::get(session_message),
        )
        .route("/session/{id}/prompt", routing::post(session_prompt))
        .route(
            "/session/{id}/prompt_async",
            routing::post(session_prompt_async),
        )
        .route(
            "/session/{id}/permissions/{permission_id}",
            routing::post(session_permission),
        )
        .route("/doc", routing::get(doc))
}

async fn global_health() -> impl IntoResponse {
    Json(HealthResponse::current())
}

async fn global_event(
    State(server): State<AppState>,
) -> Sse<impl futures::Stream<Item = Result<SseEvent, Infallible>>> {
    let connected = stream::once(async {
        Ok::<_, Infallible>(
            SseEvent::default()
                .event("server.connected")
                .data(r#"{"connected":true,"stub":true}"#),
        )
    });
    let core = server.core();
    let events = core.subscribe().map(|event| {
        let data = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_owned());
        Ok::<_, Infallible>(SseEvent::default().event("agent.core_event").data(data))
    });
    Sse::new(connected.chain(events)).keep_alive(KeepAlive::new().interval(Duration::from_secs(10)))
}

async fn projects(State(server): State<AppState>) -> Response {
    let core = server.core();
    match core.store().projects().list().await {
        Ok(projects) => Json(
            projects
                .into_iter()
                .map(|project| {
                    json!({
                        "id": project.id,
                        "name": project.name,
                        "root": project.root,
                        "createdAt": project.created_at,
                        "updatedAt": project.updated_at,
                    })
                })
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(error) => compat_error("store_error", error.to_string()),
    }
}

async fn project_current() -> Response {
    stub_response("project.current is not implemented for agent-server yet")
}

async fn config_get() -> Response {
    stub_response("config.get is not implemented for agent-server yet")
}

async fn config_providers(State(server): State<AppState>) -> Response {
    let core = server.core();
    let providers = grouped_provider_models(&core);
    Json(json!({ "providers": providers, "default": {} })).into_response()
}

async fn providers(State(server): State<AppState>) -> Response {
    let core = server.core();
    Json(grouped_provider_models(&core)).into_response()
}

async fn sessions(State(server): State<AppState>) -> Response {
    let core = server.core();
    match core.store().sessions().list().await {
        Ok(sessions) => {
            Json(sessions.into_iter().map(compat_session).collect::<Vec<_>>()).into_response()
        }
        Err(error) => compat_error("store_error", error.to_string()),
    }
}

async fn session_create() -> Response {
    stub_response("session.create is not implemented for agent-server yet")
}

async fn session_status(State(server): State<AppState>) -> Response {
    let core = server.core();
    match core.store().sessions().list().await {
        Ok(sessions) => Json(
            sessions
                .into_iter()
                .map(|session| {
                    json!({
                        "sessionID": session.id,
                        "running": false,
                        "stub": true,
                    })
                })
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(error) => compat_error("store_error", error.to_string()),
    }
}

async fn session_get(State(server): State<AppState>, Path(id): Path<String>) -> Response {
    let session_id = match parse_session_id(&id) {
        Ok(session_id) => session_id,
        Err(response) => return response,
    };
    let core = server.core();
    match core.session(session_id).await {
        Ok(session) => Json(compat_session(session)).into_response(),
        Err(error) => compat_error("core_error", error.to_string()),
    }
}

async fn session_update() -> Response {
    stub_response("session.update is not implemented for agent-server yet")
}

async fn session_delete(State(server): State<AppState>, Path(id): Path<String>) -> Response {
    let session_id = match parse_session_id(&id) {
        Ok(session_id) => session_id,
        Err(response) => return response,
    };
    let core = server.core();
    match core.delete_session(session_id).await {
        Ok(()) => Json(true).into_response(),
        Err(error) => compat_error("core_error", error.to_string()),
    }
}

async fn session_messages(State(server): State<AppState>, Path(id): Path<String>) -> Response {
    let session_id = match parse_session_id(&id) {
        Ok(session_id) => session_id,
        Err(response) => return response,
    };
    let core = server.core();
    match core.messages(session_id).await {
        Ok(messages) => {
            Json(messages.into_iter().map(compat_message).collect::<Vec<_>>()).into_response()
        }
        Err(error) => compat_error("core_error", error.to_string()),
    }
}

async fn session_message(
    State(server): State<AppState>,
    Path((id, message_id)): Path<(String, String)>,
) -> Response {
    let session_id = match parse_session_id(&id) {
        Ok(session_id) => session_id,
        Err(response) => return response,
    };
    let message_id = match message_id.parse::<Ulid>() {
        Ok(message_id) => message_id,
        Err(_) => return invalid_request("invalid_message_id"),
    };
    let core = server.core();
    match core.messages(session_id).await {
        Ok(messages) => match messages
            .into_iter()
            .find(|message| message.id == message_id)
        {
            Some(message) => Json(compat_message(message)).into_response(),
            None => compat_error("not_found", format!("message not found: {message_id}")),
        },
        Err(error) => compat_error("core_error", error.to_string()),
    }
}

async fn session_prompt() -> Response {
    stub_response("session.prompt is not implemented for agent-server yet")
}

async fn session_prompt_async() -> Response {
    stub_response("session.prompt_async is not implemented for agent-server yet")
}

async fn session_permission() -> Response {
    stub_response("session permission responses are not implemented for agent-server yet")
}

async fn doc() -> Response {
    Json(openapi_stub()).into_response()
}

fn compat_session(session: agent_store::Session) -> Value {
    json!({
        "id": session.id,
        "projectID": session.project_id,
        "title": session.title,
        "providerID": session.provider,
        "modelID": session.model,
        "loopID": session.loop_name,
        "createdAt": session.created_at,
        "updatedAt": session.updated_at,
    })
}

fn compat_message(message: StoredMessage) -> Value {
    json!({
        "info": {
            "id": message.id,
            "sessionID": message.session_id,
            "createdAt": message.created_at,
            "role": message.message.role,
        },
        "parts": compat_parts(&message.message),
    })
}

fn compat_parts(message: &provider::Message) -> Vec<Value> {
    message
        .content
        .iter()
        .map(|block| match block {
            provider::ContentBlock::Text { text } => json!({ "type": "text", "text": text }),
            provider::ContentBlock::ImageUrl { url } => json!({ "type": "image_url", "url": url }),
            provider::ContentBlock::ToolCall { id, name, input } => {
                json!({ "type": "tool_call", "id": id, "name": name, "input": input })
            }
            provider::ContentBlock::ToolResult {
                call_id,
                output,
                is_error,
            } => json!({
                "type": "tool_result",
                "callID": call_id,
                "output": output,
                "isError": is_error,
            }),
            provider::ContentBlock::Reasoning { text } => {
                json!({ "type": "reasoning", "text": text })
            }
            provider::ContentBlock::Refusal { text } => {
                json!({ "type": "refusal", "text": text })
            }
        })
        .collect()
}

fn grouped_provider_models(core: &Arc<dyn AgentCore>) -> Vec<Value> {
    let mut grouped = std::collections::BTreeMap::<String, Vec<String>>::new();
    for model in core.list_models() {
        grouped
            .entry(model.provider_name)
            .or_default()
            .push(model.model.id.to_string());
    }
    grouped
        .into_iter()
        .map(|(id, models)| json!({ "id": id, "models": models }))
        .collect()
}

fn compat_error(code: impl Into<String>, message: impl Into<String>) -> Response {
    (
        axum::http::StatusCode::BAD_REQUEST,
        Json(ErrorResponse::new(code.into(), message.into())),
    )
        .into_response()
}

fn openapi_stub() -> Value {
    json!({
        "openapi": "3.1.0",
        "info": {
            "title": "agent-server OpenCode compatibility stub",
            "version": env!("CARGO_PKG_VERSION"),
        },
        "paths": {
            "/v1/compat/opencode/global/health": {},
            "/v1/compat/opencode/global/event": {},
            "/v1/compat/opencode/project": {},
            "/v1/compat/opencode/project/current": {},
            "/v1/compat/opencode/config": {},
            "/v1/compat/opencode/config/providers": {},
            "/v1/compat/opencode/provider": {},
            "/v1/compat/opencode/session": {},
            "/v1/compat/opencode/session/status": {},
            "/v1/compat/opencode/session/{id}": {},
            "/v1/compat/opencode/session/{id}/message": {},
            "/v1/compat/opencode/session/{id}/message/{message_id}": {},
            "/v1/compat/opencode/session/{id}/prompt": {},
            "/v1/compat/opencode/session/{id}/prompt_async": {},
            "/v1/compat/opencode/session/{id}/permissions/{permission_id}": {},
            "/v1/compat/opencode/doc": {},
        }
    })
}
