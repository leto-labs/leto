use std::collections::BTreeMap;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use agent_core::{AgentCore, CoreError};
use agent_store::{
    CredentialEntry, CredentialStoreKey, Project, ProjectId, ProjectUpdate, Session, SessionId,
    SessionUpdate, StoreError, StoredMessage,
};
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use axum::response::{IntoResponse, Json, Response};
use axum::{Router, routing};
use futures::StreamExt;
use serde_json::json;
use tower_http::cors::{Any, CorsLayer};
use ulid::Ulid;

use crate::compat;
use crate::server::AgentServer;
use crate::types::{
    CreateProjectRequest, CreateSessionRequest, CredentialRecord, ErrorResponse, HealthResponse,
    ProjectRootRequest, ProviderCatalogEntry, ProviderModelRecord, SessionRuntimeView,
    TrajectoryRecord, TurnRequest, UpdateCredentialHealthRequest,
};

type AppState = Arc<AgentServer>;

/// Builds the canonical and compatibility HTTP router.
pub fn build_router(server: Arc<AgentServer>) -> Router {
    Router::new()
        .nest("/v1", canonical_router())
        .nest("/v1/compat/opencode", compat::opencode::router())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers([AUTHORIZATION, CONTENT_TYPE]),
        )
        .with_state(server)
}

/// Serves the router on the provided address.
pub async fn serve(
    server: Arc<AgentServer>,
    addr: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let router = build_router(server);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("agent-server listening on {addr}");
    axum::serve(listener, router).await?;
    Ok(())
}

fn canonical_router() -> Router<AppState> {
    Router::new()
        .route("/health", routing::get(health))
        .route("/status", routing::get(status))
        .route("/events", routing::get(events))
        .route(
            "/projects",
            routing::get(list_projects).post(create_project),
        )
        .route("/projects/raw", routing::post(create_project_record))
        .route(
            "/projects/find-by-root",
            routing::post(find_project_by_root),
        )
        .route("/projects/resolve", routing::post(resolve_project))
        .route(
            "/projects/{id}",
            routing::get(get_project)
                .put(replace_project)
                .patch(update_project)
                .delete(delete_project),
        )
        .route(
            "/projects/{id}/sessions",
            routing::get(list_project_sessions).post(create_session),
        )
        .route("/providers", routing::get(list_providers))
        .route("/models", routing::get(list_models))
        .route(
            "/sessions",
            routing::get(list_sessions).post(create_session_record),
        )
        .route(
            "/sessions/{id}",
            routing::get(get_session)
                .put(replace_session)
                .patch(update_session)
                .delete(delete_session),
        )
        .route(
            "/sessions/{id}/messages",
            routing::get(list_messages)
                .put(replace_messages)
                .delete(delete_messages),
        )
        .route(
            "/sessions/{id}/trajectory",
            routing::get(get_trajectory)
                .put(upsert_trajectory)
                .delete(delete_trajectory),
        )
        .route("/trajectories", routing::get(list_trajectories))
        .route("/sessions/{id}/runtime", routing::get(get_runtime_view))
        .route("/sessions/{id}/turns", routing::post(start_turn))
        .route("/sessions/{id}/cancel", routing::post(cancel_turn))
        .route("/credentials", routing::get(list_credentials))
        .route(
            "/credentials/{provider}",
            routing::get(list_provider_credentials),
        )
        .route(
            "/credentials/{provider}/{id}",
            routing::get(get_credential)
                .post(create_credential)
                .put(update_credential)
                .delete(delete_credential),
        )
        .route(
            "/credentials/{provider}/{id}/health",
            routing::patch(update_credential_health),
        )
}

async fn health() -> impl IntoResponse {
    Json(HealthResponse::current())
}

async fn status(State(server): State<AppState>) -> Response {
    match server.status().await {
        Ok(status) => Json(status).into_response(),
        Err(error) => store_error_response(error),
    }
}

async fn events(
    State(server): State<AppState>,
) -> Sse<impl futures::Stream<Item = Result<SseEvent, Infallible>>> {
    let core = server.core();
    let stream = core.subscribe().map(|event| {
        let data = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_owned());
        Ok::<_, Infallible>(SseEvent::default().data(data))
    });
    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(10)))
}

async fn list_projects(State(server): State<AppState>) -> Response {
    let core = server.core();
    match core.store().projects().list().await {
        Ok(projects) => Json(projects).into_response(),
        Err(error) => store_error_response(error),
    }
}

async fn create_project(
    State(server): State<AppState>,
    Json(body): Json<CreateProjectRequest>,
) -> Response {
    let core = server.core();
    let project = Project::new(body.name, body.root, body.config.unwrap_or_default());
    match core.store().projects().create(project).await {
        Ok(project) => (StatusCode::CREATED, Json(project)).into_response(),
        Err(error) => store_error_response(error),
    }
}

async fn create_project_record(
    State(server): State<AppState>,
    Json(project): Json<Project>,
) -> Response {
    let core = server.core();
    match core.store().projects().create(project).await {
        Ok(project) => (StatusCode::CREATED, Json(project)).into_response(),
        Err(error) => store_error_response(error),
    }
}

async fn find_project_by_root(
    State(server): State<AppState>,
    Json(body): Json<ProjectRootRequest>,
) -> Response {
    let core = server.core();
    match core.store().projects().find_by_root(&body.root).await {
        Ok(project) => Json(project).into_response(),
        Err(error) => store_error_response(error),
    }
}

async fn resolve_project(
    State(server): State<AppState>,
    Json(body): Json<ProjectRootRequest>,
) -> Response {
    let core = server.core();
    match core.resolve_or_create_project(body.root).await {
        Ok(project) => Json(project).into_response(),
        Err(error) => core_error_response(error),
    }
}

async fn get_project(State(server): State<AppState>, Path(id): Path<String>) -> Response {
    let project_id = match parse_project_id(&id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let core = server.core();
    match core.project(project_id).await {
        Ok(project) => Json(project).into_response(),
        Err(error) => core_error_response(error),
    }
}

async fn replace_project(
    State(server): State<AppState>,
    Path(id): Path<String>,
    Json(project): Json<Project>,
) -> Response {
    let project_id = match parse_project_id(&id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let core = server.core();
    match core.store().projects().update(project_id, project).await {
        Ok(project) => Json(project).into_response(),
        Err(error) => store_error_response(error),
    }
}

async fn update_project(
    State(server): State<AppState>,
    Path(id): Path<String>,
    Json(update): Json<ProjectUpdate>,
) -> Response {
    let project_id = match parse_project_id(&id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let core = server.core();
    let projects = core.store().projects();
    let mut project = match projects.get(project_id).await {
        Ok(project) => project,
        Err(error) => return store_error_response(error),
    };
    if let Some(name) = update.name {
        project.name = Some(name);
    }
    if let Some(config) = update.config {
        project.config = config;
    }
    project.updated_at = chrono::Utc::now();
    match projects.update(project_id, project).await {
        Ok(project) => Json(project).into_response(),
        Err(error) => store_error_response(error),
    }
}

async fn delete_project(State(server): State<AppState>, Path(id): Path<String>) -> Response {
    let project_id = match parse_project_id(&id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let core = server.core();
    match core.store().projects().delete(project_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => store_error_response(error),
    }
}

async fn list_project_sessions(State(server): State<AppState>, Path(id): Path<String>) -> Response {
    let project_id = match parse_project_id(&id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let core = server.core();
    match core.sessions_for_project(project_id).await {
        Ok(sessions) => Json(sessions).into_response(),
        Err(error) => core_error_response(error),
    }
}

async fn create_session(
    State(server): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<CreateSessionRequest>,
) -> Response {
    let project_id = match parse_project_id(&id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let core = server.core();
    let session = match core.create_session(project_id).await {
        Ok(session) => session,
        Err(error) => return core_error_response(error),
    };
    let session = match core
        .update_session(
            session.id,
            SessionUpdate {
                title: body.title,
                provider: body.provider.map(Some),
                model: body.model.map(Some),
                loop_name: body.loop_name.map(Some),
                request: None,
            },
        )
        .await
    {
        Ok(session) => session,
        Err(error) => return core_error_response(error),
    };
    (StatusCode::CREATED, Json(session)).into_response()
}

async fn list_providers(State(server): State<AppState>) -> Response {
    let core = server.core();
    Json(grouped_provider_models(&core)).into_response()
}

async fn list_models(State(server): State<AppState>) -> Response {
    let core = server.core();
    Json(
        core.list_models()
            .into_iter()
            .map(ProviderModelRecord::from)
            .collect::<Vec<_>>(),
    )
    .into_response()
}

async fn list_sessions(State(server): State<AppState>) -> Response {
    let core = server.core();
    match core.store().sessions().list().await {
        Ok(sessions) => Json(sessions).into_response(),
        Err(error) => store_error_response(error),
    }
}

async fn create_session_record(
    State(server): State<AppState>,
    Json(session): Json<Session>,
) -> Response {
    let core = server.core();
    match core.store().sessions().create(session).await {
        Ok(session) => (StatusCode::CREATED, Json(session)).into_response(),
        Err(error) => store_error_response(error),
    }
}

async fn get_session(State(server): State<AppState>, Path(id): Path<String>) -> Response {
    let session_id = match parse_session_id(&id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let core = server.core();
    match core.session(session_id).await {
        Ok(session) => Json(session).into_response(),
        Err(error) => core_error_response(error),
    }
}

async fn replace_session(
    State(server): State<AppState>,
    Path(id): Path<String>,
    Json(session): Json<Session>,
) -> Response {
    let session_id = match parse_session_id(&id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let core = server.core();
    match core.store().sessions().update(session_id, session).await {
        Ok(session) => Json(session).into_response(),
        Err(error) => store_error_response(error),
    }
}

async fn update_session(
    State(server): State<AppState>,
    Path(id): Path<String>,
    Json(update): Json<SessionUpdate>,
) -> Response {
    let session_id = match parse_session_id(&id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let core = server.core();
    match core.update_session(session_id, update).await {
        Ok(session) => Json(session).into_response(),
        Err(error) => core_error_response(error),
    }
}

async fn delete_session(State(server): State<AppState>, Path(id): Path<String>) -> Response {
    let session_id = match parse_session_id(&id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let core = server.core();
    match core.delete_session(session_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => core_error_response(error),
    }
}

async fn list_messages(State(server): State<AppState>, Path(id): Path<String>) -> Response {
    let session_id = match parse_session_id(&id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let core = server.core();
    match core.messages(session_id).await {
        Ok(messages) => Json(messages).into_response(),
        Err(error) => core_error_response(error),
    }
}

async fn replace_messages(
    State(server): State<AppState>,
    Path(id): Path<String>,
    Json(messages): Json<Vec<StoredMessage>>,
) -> Response {
    let session_id = match parse_session_id(&id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let core = server.core();
    match core
        .store()
        .messages()
        .replace_for_session(session_id, messages)
        .await
    {
        Ok(messages) => Json(messages).into_response(),
        Err(error) => store_error_response(error),
    }
}

async fn delete_messages(State(server): State<AppState>, Path(id): Path<String>) -> Response {
    let session_id = match parse_session_id(&id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let core = server.core();
    match core.store().messages().delete_for_session(session_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => store_error_response(error),
    }
}

async fn get_trajectory(State(server): State<AppState>, Path(id): Path<String>) -> Response {
    let session_id = match parse_session_id(&id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let core = server.core();
    match core.trajectory(session_id).await {
        Ok(trajectory) => Json(trajectory).into_response(),
        Err(error) => core_error_response(error),
    }
}

async fn upsert_trajectory(
    State(server): State<AppState>,
    Path(id): Path<String>,
    Json(trajectory): Json<atif::Trajectory>,
) -> Response {
    let session_id = match parse_session_id(&id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let core = server.core();
    match core.upsert_trajectory(session_id, trajectory).await {
        Ok(trajectory) => Json(trajectory).into_response(),
        Err(error) => core_error_response(error),
    }
}

async fn delete_trajectory(State(server): State<AppState>, Path(id): Path<String>) -> Response {
    let session_id = match parse_session_id(&id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let core = server.core();
    match core.store().trajectories().delete(session_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => store_error_response(error),
    }
}

async fn list_trajectories(State(server): State<AppState>) -> Response {
    let core = server.core();
    match core.store().trajectories().list().await {
        Ok(records) => Json(
            records
                .into_iter()
                .map(|(session_id, trajectory)| TrajectoryRecord {
                    session_id,
                    trajectory,
                })
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(error) => store_error_response(error),
    }
}

async fn get_runtime_view(State(server): State<AppState>, Path(id): Path<String>) -> Response {
    let session_id = match parse_session_id(&id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let core = server.core();
    let config = match core.effective_runtime_config(session_id).await {
        Ok(config) => config,
        Err(error) => return core_error_response(error),
    };
    let current_loop_name = match core.current_loop_name_for_session(session_id).await {
        Ok(loop_name) => loop_name,
        Err(error) => return core_error_response(error),
    };
    let current_model_id = match core.current_model_id_for_session(session_id).await {
        Ok(model_id) => model_id,
        Err(error) => return core_error_response(error),
    };
    Json(SessionRuntimeView {
        config,
        current_loop_name,
        current_model_id,
    })
    .into_response()
}

async fn start_turn(
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

async fn cancel_turn(State(server): State<AppState>, Path(id): Path<String>) -> Response {
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

async fn list_credentials(State(server): State<AppState>) -> Response {
    let core = server.core();
    match core.store().credentials().list().await {
        Ok(records) => Json(credential_records(records)).into_response(),
        Err(error) => store_error_response(error),
    }
}

async fn list_provider_credentials(
    State(server): State<AppState>,
    Path(provider): Path<String>,
) -> Response {
    let core = server.core();
    match core
        .store()
        .credentials()
        .list_for_provider(&provider)
        .await
    {
        Ok(records) => Json(credential_records(records)).into_response(),
        Err(error) => store_error_response(error),
    }
}

async fn get_credential(
    State(server): State<AppState>,
    Path((provider, id)): Path<(String, String)>,
) -> Response {
    let core = server.core();
    match core.store().credentials().get((provider, id)).await {
        Ok(entry) => Json(entry).into_response(),
        Err(error) => store_error_response(error),
    }
}

async fn create_credential(
    State(server): State<AppState>,
    Path((provider, id)): Path<(String, String)>,
    Json(credential): Json<CredentialEntry>,
) -> Response {
    let core = server.core();
    match core
        .store()
        .credentials()
        .create((provider, id), credential)
        .await
    {
        Ok(entry) => (StatusCode::CREATED, Json(entry)).into_response(),
        Err(error) => store_error_response(error),
    }
}

async fn update_credential(
    State(server): State<AppState>,
    Path((provider, id)): Path<(String, String)>,
    Json(credential): Json<CredentialEntry>,
) -> Response {
    let core = server.core();
    match core
        .store()
        .credentials()
        .update((provider, id), credential)
        .await
    {
        Ok(entry) => Json(entry).into_response(),
        Err(error) => store_error_response(error),
    }
}

async fn update_credential_health(
    State(server): State<AppState>,
    Path((provider, id)): Path<(String, String)>,
    Json(body): Json<UpdateCredentialHealthRequest>,
) -> Response {
    let core = server.core();
    match core
        .store()
        .credentials()
        .update_health(&provider, &id, &body.health)
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => store_error_response(error),
    }
}

async fn delete_credential(
    State(server): State<AppState>,
    Path((provider, id)): Path<(String, String)>,
) -> Response {
    let core = server.core();
    match core.store().credentials().delete((provider, id)).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => store_error_response(error),
    }
}

fn credential_records(
    records: Vec<(CredentialStoreKey, CredentialEntry)>,
) -> Vec<CredentialRecord> {
    records
        .into_iter()
        .map(
            |((provider_name, credential_id), credential)| CredentialRecord {
                provider_name,
                credential_id,
                credential,
            },
        )
        .collect()
}

pub(crate) fn parse_project_id(value: &str) -> Result<ProjectId, Response> {
    value
        .parse::<Ulid>()
        .map_err(|_| invalid_request("invalid_project_id"))
}

pub(crate) fn parse_session_id(value: &str) -> Result<SessionId, Response> {
    value
        .parse::<Ulid>()
        .map_err(|_| invalid_request("invalid_session_id"))
}

pub(crate) fn invalid_request(code: &str) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse::new(code, "invalid request")),
    )
        .into_response()
}

fn grouped_provider_models(core: &Arc<dyn AgentCore>) -> Vec<ProviderCatalogEntry> {
    let mut grouped = BTreeMap::<String, Vec<String>>::new();
    for model in core.list_models() {
        grouped
            .entry(model.provider_name)
            .or_default()
            .push(model.model.id.to_string());
    }
    grouped
        .into_iter()
        .map(|(name, mut model_ids)| {
            model_ids.sort();
            model_ids.dedup();
            ProviderCatalogEntry { name, model_ids }
        })
        .collect()
}

fn core_error_response(error: CoreError) -> Response {
    match error {
        CoreError::Store(error) => store_error_response(error),
        CoreError::ProviderNotRegistered(name) => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::with_details(
                "provider_not_registered",
                format!("provider not registered: {name}"),
                json!({ "name": name }),
            )),
        )
            .into_response(),
        CoreError::LoopNotRegistered(name) => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::with_details(
                "loop_not_registered",
                format!("loop not registered: {name}"),
                json!({ "name": name }),
            )),
        )
            .into_response(),
        CoreError::TurnActive(session_id) => (
            StatusCode::CONFLICT,
            Json(ErrorResponse::with_details(
                "turn_active",
                format!("a turn is already active for session {session_id}"),
                json!({ "session_id": session_id.to_string() }),
            )),
        )
            .into_response(),
        CoreError::NoProvidersRegistered => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "no_providers_registered",
                "no providers are registered",
            )),
        )
            .into_response(),
        CoreError::NoLoopsRegistered => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "no_loops_registered",
                "no loops are registered",
            )),
        )
            .into_response(),
        CoreError::Runtime(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("runtime_error", error.to_string())),
        )
            .into_response(),
    }
}

fn store_error_response(error: StoreError) -> Response {
    match error {
        StoreError::NotFound(message) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("store_not_found", message)),
        )
            .into_response(),
        StoreError::AlreadyExists(message) => (
            StatusCode::CONFLICT,
            Json(ErrorResponse::new("store_already_exists", message)),
        )
            .into_response(),
        StoreError::InvalidInput(message) => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("store_invalid_input", message)),
        )
            .into_response(),
        StoreError::Io(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("store_io", error.to_string())),
        )
            .into_response(),
        StoreError::Serde(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("store_serde", error.to_string())),
        )
            .into_response(),
    }
}
