use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use axum::response::{IntoResponse, Json, Response};
use axum::{routing, Router};
use futures::stream::Stream;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt as _;
use ulid::Ulid;

use brain_types::{BrainError, BrainErrorCode, CredentialEntry, Project, ProjectConfig, ProjectId, ProjectUpdate, SessionUpdate};

use crate::api::BrainApi;
use crate::server::BrainServer;
use crate::types::{CreateProjectRequest, SendMessageRequest};

type AppState = Arc<BrainServer>;

pub fn build_router(server: Arc<BrainServer>) -> Router {
    Router::new()
        // Project CRUD
        .route("/projects", routing::post(create_project).get(list_projects))
        .route("/projects/{id}", routing::get(get_project).delete(delete_project).patch(update_project))
        // Sessions scoped under project (create, list)
        .route("/projects/{project_id}/sessions", routing::post(create_session).get(list_sessions))
        // Session by id (flat, since session ids are globally unique)
        .route("/sessions/{id}", routing::get(get_session).delete(delete_session).patch(update_session))
        // Messages
        .route("/sessions/{id}/messages", routing::get(list_messages))
        // Turn management
        .route("/sessions/{id}/message", routing::post(send_message))
        .route("/sessions/{id}/message/stream", routing::post(send_message_stream))
        .route("/sessions/{id}/cancel", routing::post(cancel_turn))
        // Providers & credentials
        .route("/providers", routing::get(list_providers))
        .route("/credentials", routing::get(list_credentials))
        .route("/credentials/{name}", routing::get(get_credentials).put(save_credential))
        .route("/credentials/{name}/{credential_id}", routing::delete(delete_credential))
        // Server-wide
        .route("/events", routing::get(sse_handler))
        .route("/status", routing::get(status))
        .with_state(server)
}

pub async fn serve(server: Arc<BrainServer>, addr: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let router = build_router(server);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("brain-server listening on {}", addr);
    axum::serve(listener, router).await?;
    Ok(())
}

// -- Project handlers --

async fn create_project(
    State(server): State<AppState>,
    Json(body): Json<CreateProjectRequest>,
) -> Response {
    let project = Project::new(body.name, body.root, ProjectConfig::default());
    match server.create_project(project).await {
        Ok(p) => (StatusCode::CREATED, Json(p)).into_response(),
        Err(e) => error_response(e),
    }
}

async fn list_projects(State(server): State<AppState>) -> Response {
    match server.list_projects().await {
        Ok(projects) => Json(projects).into_response(),
        Err(e) => error_response(e),
    }
}

async fn get_project(State(server): State<AppState>, Path(id): Path<String>) -> Response {
    let id = match parse_project_id(&id) {
        Ok(id) => id,
        Err(r) => return r,
    };
    match server.get_project(id).await {
        Ok(p) => Json(p).into_response(),
        Err(e) => error_response(e),
    }
}

async fn update_project(
    State(server): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<ProjectUpdate>,
) -> Response {
    let id = match parse_project_id(&id) {
        Ok(id) => id,
        Err(r) => return r,
    };
    match server.update_project(id, body).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => error_response(e),
    }
}

async fn delete_project(State(server): State<AppState>, Path(id): Path<String>) -> Response {
    let id = match parse_project_id(&id) {
        Ok(id) => id,
        Err(r) => return r,
    };
    match server.delete_project(id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => error_response(e),
    }
}

// -- Session handlers --

async fn create_session(
    State(server): State<AppState>,
    Path(project_id): Path<String>,
) -> Response {
    let project_id = match parse_project_id(&project_id) {
        Ok(id) => id,
        Err(r) => return r,
    };
    match server.create_session(project_id).await {
        Ok(session) => (StatusCode::CREATED, Json(session)).into_response(),
        Err(e) => error_response(e),
    }
}

async fn list_sessions(
    State(server): State<AppState>,
    Path(project_id): Path<String>,
) -> Response {
    let project_id = match parse_project_id(&project_id) {
        Ok(id) => id,
        Err(r) => return r,
    };
    match server.list_sessions(project_id).await {
        Ok(sessions) => Json(sessions).into_response(),
        Err(e) => error_response(e),
    }
}

async fn get_session(State(server): State<AppState>, Path(id): Path<String>) -> Response {
    let id = match parse_ulid(&id) {
        Ok(id) => id,
        Err(r) => return r,
    };
    match server.get_session(id).await {
        Ok(session) => Json(session).into_response(),
        Err(e) => error_response(e),
    }
}

async fn update_session(
    State(server): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<SessionUpdate>,
) -> Response {
    let id = match parse_ulid(&id) {
        Ok(id) => id,
        Err(r) => return r,
    };
    match server.update_session(id, body).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => error_response(e),
    }
}

async fn delete_session(State(server): State<AppState>, Path(id): Path<String>) -> Response {
    let id = match parse_ulid(&id) {
        Ok(id) => id,
        Err(r) => return r,
    };
    match server.delete_session(id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => error_response(e),
    }
}

// -- Message history --

async fn list_messages(State(server): State<AppState>, Path(id): Path<String>) -> Response {
    let id = match parse_ulid(&id) {
        Ok(id) => id,
        Err(r) => return r,
    };
    match server.list_messages(id).await {
        Ok(messages) => Json(messages).into_response(),
        Err(e) => error_response(e),
    }
}

// -- Turn handlers --

async fn send_message(
    State(server): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<SendMessageRequest>,
) -> Response {
    let id = match parse_ulid(&id) {
        Ok(id) => id,
        Err(r) => return r,
    };
    match server.send_message(id, &body.content).await {
        Ok(()) => StatusCode::ACCEPTED.into_response(),
        Err(e) => error_response(e),
    }
}

/// Streaming endpoint: returns NDJSON (newline-delimited JSON) of Event objects.
async fn send_message_stream(
    State(server): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<SendMessageRequest>,
) -> Response {
    let id = match parse_ulid(&id) {
        Ok(id) => id,
        Err(r) => return r,
    };
    match server.send_message_stream(id, &body.content).await {
        Ok(event_stream) => {
            let ndjson = event_stream.map(|event| {
                let mut line = serde_json::to_string(&event).unwrap_or_default();
                line.push('\n');
                Ok::<_, Infallible>(line)
            });
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/x-ndjson")
                .header("Transfer-Encoding", "chunked")
                .body(Body::from_stream(ndjson))
                .unwrap()
        }
        Err(e) => error_response(e),
    }
}

async fn cancel_turn(State(server): State<AppState>, Path(id): Path<String>) -> Response {
    let id = match parse_ulid(&id) {
        Ok(id) => id,
        Err(r) => return r,
    };
    match server.cancel_turn(id).await {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => error_response(e),
    }
}

// -- Providers & credentials --

async fn list_providers(State(server): State<AppState>) -> Response {
    match server.list_providers().await {
        Ok(providers) => Json(providers).into_response(),
        Err(e) => error_response(e),
    }
}

async fn list_credentials(State(server): State<AppState>) -> Response {
    match server.list_credentials().await {
        Ok(creds) => {
            let entries: Vec<serde_json::Value> = creds
                .into_iter()
                .map(|(name, entry)| serde_json::json!({"provider": name, "entry": entry}))
                .collect();
            Json(entries).into_response()
        }
        Err(e) => error_response(e),
    }
}

async fn get_credentials(State(server): State<AppState>, Path(name): Path<String>) -> Response {
    match server.get_credentials(&name).await {
        Ok(entries) => Json(entries).into_response(),
        Err(e) => error_response(e),
    }
}

async fn save_credential(
    State(server): State<AppState>,
    Path(name): Path<String>,
    Json(entry): Json<CredentialEntry>,
) -> Response {
    match server.save_credential(&name, entry).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => error_response(e),
    }
}

async fn delete_credential(
    State(server): State<AppState>,
    Path((name, credential_id)): Path<(String, String)>,
) -> Response {
    match server.delete_credential(&name, &credential_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => error_response(e),
    }
}

// -- SSE & status --

async fn sse_handler(
    State(server): State<AppState>,
) -> Sse<impl Stream<Item = Result<SseEvent, Infallible>>> {
    let rx = server.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|result| match result {
        Ok(server_event) => {
            let data = serde_json::to_string(&server_event).unwrap_or_default();
            Some(Ok(SseEvent::default().data(data)))
        }
        Err(e) => {
            tracing::debug!("SSE broadcast recv error: {e}");
            None
        }
    });

    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(10)))
}

async fn status(State(server): State<AppState>) -> Response {
    match server.status().await {
        Ok(status) => Json(status).into_response(),
        Err(e) => error_response(e),
    }
}

// -- Helpers --

fn parse_ulid(s: &str) -> Result<Ulid, Response> {
    s.parse::<Ulid>().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "invalid id"})),
        )
            .into_response()
    })
}

fn parse_project_id(s: &str) -> Result<ProjectId, Response> {
    s.parse::<Ulid>().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "invalid project id"})),
        )
            .into_response()
    })
}

fn error_response(err: BrainError) -> Response {
    let status = match err.code() {
        BrainErrorCode::TurnActive => StatusCode::CONFLICT,
        BrainErrorCode::StorageFailed => {
            if err.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
        BrainErrorCode::Cancelled => StatusCode::OK,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };

    (
        status,
        Json(serde_json::json!({
            "error": err.to_string(),
            "code": err.code(),
        })),
    )
        .into_response()
}
