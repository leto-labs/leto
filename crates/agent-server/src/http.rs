mod errors;
mod routes;

use std::collections::BTreeMap;
use std::sync::Arc;

use agent_core::AgentCore;
use agent_store::{ProjectId, SessionId};
use axum::http::StatusCode;
use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use axum::response::{IntoResponse, Json, Response};
use axum::{Router, routing};
use tower_http::cors::{Any, CorsLayer};
use ulid::Ulid;

use crate::compat;
use crate::server::AgentServer;
use crate::types::{ErrorResponse, ProviderCatalogEntry};

type AppState = Arc<AgentServer>;

/// Builds the canonical and compatibility HTTP router.
pub fn build_router(server: Arc<AgentServer>) -> Router {
    Router::new()
        .route("/health", routing::get(routes::system::health))
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
        .route("/health", routing::get(routes::system::health))
        .route("/status", routing::get(routes::system::status))
        .route("/agents", routing::get(routes::system::list_agents))
        .route(
            "/mcp/servers",
            routing::get(routes::system::list_mcp_servers),
        )
        .route("/mcp/tools", routing::get(routes::system::list_mcp_servers))
        .route("/events", routing::get(routes::system::events))
        .route(
            "/projects",
            routing::get(routes::projects::list_projects).post(routes::projects::create_project),
        )
        .route(
            "/projects/raw",
            routing::post(routes::projects::create_project_record),
        )
        .route(
            "/projects/find-by-root",
            routing::post(routes::projects::find_project_by_root),
        )
        .route(
            "/projects/resolve",
            routing::post(routes::projects::resolve_project),
        )
        .route(
            "/projects/{id}",
            routing::get(routes::projects::get_project)
                .put(routes::projects::replace_project)
                .patch(routes::projects::update_project)
                .delete(routes::projects::delete_project),
        )
        .route(
            "/projects/{id}/sessions",
            routing::get(routes::projects::list_project_sessions)
                .post(routes::projects::create_session),
        )
        .route(
            "/providers",
            routing::get(routes::providers::list_providers),
        )
        .route("/models", routing::get(routes::providers::list_models))
        .route("/models/{id}", routing::get(routes::providers::get_model))
        .route(
            "/chat/completions",
            routing::post(routes::chat_completions::create_chat_completion),
        )
        .route(
            "/sessions",
            routing::get(routes::sessions::list_sessions)
                .post(routes::sessions::create_session_record),
        )
        .route(
            "/sessions/{id}",
            routing::get(routes::sessions::get_session)
                .put(routes::sessions::replace_session)
                .patch(routes::sessions::update_session)
                .delete(routes::sessions::delete_session),
        )
        .route(
            "/sessions/{id}/messages",
            routing::get(routes::sessions::list_messages)
                .put(routes::sessions::replace_messages)
                .delete(routes::sessions::delete_messages),
        )
        .route(
            "/sessions/{id}/trajectory",
            routing::get(routes::sessions::get_trajectory)
                .put(routes::sessions::upsert_trajectory)
                .delete(routes::sessions::delete_trajectory),
        )
        .route(
            "/trajectories",
            routing::get(routes::sessions::list_trajectories),
        )
        .route(
            "/sessions/{id}/runtime",
            routing::get(routes::sessions::get_runtime_view),
        )
        .route(
            "/sessions/{id}/events",
            routing::get(routes::system::session_events),
        )
        .route(
            "/sessions/{id}/turns",
            routing::post(routes::turns::start_turn),
        )
        .route(
            "/sessions/{id}/stream-turns",
            routing::post(routes::turns::start_turn_sse),
        )
        .route(
            "/sessions/{id}/batch-turns",
            routing::post(routes::turns::start_batch_turns),
        )
        .route(
            "/sessions/{id}/tool-calls",
            routing::post(routes::turns::append_tool_calls),
        )
        .route(
            "/sessions/{id}/cancel",
            routing::post(routes::turns::cancel_turn),
        )
        .route(
            "/credentials",
            routing::get(routes::credentials::list_credentials),
        )
        .route(
            "/credentials/health",
            routing::get(routes::credentials::list_credential_health),
        )
        .route(
            "/credentials/{provider}",
            routing::get(routes::credentials::list_provider_credentials),
        )
        .route(
            "/credentials/{provider}/{id}",
            routing::get(routes::credentials::get_credential)
                .post(routes::credentials::create_credential)
                .put(routes::credentials::update_credential)
                .delete(routes::credentials::delete_credential),
        )
        .route(
            "/credentials/{provider}/{id}/health",
            routing::patch(routes::credentials::update_credential_health),
        )
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

pub(crate) fn grouped_provider_models(core: &Arc<dyn AgentCore>) -> Vec<ProviderCatalogEntry> {
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
