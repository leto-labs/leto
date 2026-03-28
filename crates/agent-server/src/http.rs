mod errors;
mod routes;

use std::collections::BTreeMap;
use std::future::{Future, pending};
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

#[derive(Debug, Clone, Copy)]
pub(crate) struct InvalidRequestError(&'static str);

impl IntoResponse for InvalidRequestError {
    fn into_response(self) -> Response {
        invalid_request(self.0)
    }
}

/// Builds the canonical and compatibility HTTP router.
pub fn build_router(server: Arc<AgentServer>) -> Router {
    Router::new()
        .route("/health", routing::get(routes::system::health))
        .route("/metrics", routing::get(routes::system::metrics))
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
    serve_with_shutdown(server, addr, pending()).await
}

/// Serves the router on the provided address until the shutdown signal resolves.
///
/// Once shutdown begins, the server stops accepting new connections and allows
/// in-flight requests to complete before returning.
pub async fn serve_with_shutdown<F>(
    server: Arc<AgentServer>,
    addr: &str,
    shutdown_signal: F,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    F: Future<Output = ()> + Send + 'static,
{
    let router = build_router(server);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("agent-server listening on {addr}");
    serve_listener_with_shutdown(listener, router, shutdown_signal).await?;
    Ok(())
}

async fn serve_listener_with_shutdown<F>(
    listener: tokio::net::TcpListener,
    router: Router,
    shutdown_signal: F,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    F: Future<Output = ()> + Send + 'static,
{
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal)
        .await?;
    Ok(())
}

fn canonical_router() -> Router<AppState> {
    Router::new()
        .route("/health", routing::get(routes::system::health))
        .route("/metrics", routing::get(routes::system::metrics))
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
            "/organization/audit_logs",
            routing::get(routes::audit_logs::list_audit_logs),
        )
        .route(
            "/embeddings",
            routing::post(routes::embeddings::create_embeddings),
        )
        .route(
            "/vector_stores",
            routing::post(routes::vector_stores::create_vector_store),
        )
        .route(
            "/images/generations",
            routing::post(routes::images::create_image_generation),
        )
        .route("/videos", routing::post(routes::videos::create_video))
        .route(
            "/moderations",
            routing::post(routes::moderations::create_moderation),
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

pub(crate) fn parse_project_id(value: &str) -> Result<ProjectId, InvalidRequestError> {
    value
        .parse::<Ulid>()
        .map_err(|_| InvalidRequestError("invalid_project_id"))
}

pub(crate) fn parse_session_id(value: &str) -> Result<SessionId, InvalidRequestError> {
    value
        .parse::<Ulid>()
        .map_err(|_| InvalidRequestError("invalid_session_id"))
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use agent_core::{AgentCore, AgentCoreNative};
    use agent_store::{Project, Session};
    use provider::{MockProvider, Provider};
    use tokio::sync::oneshot;

    use super::*;

    fn make_server_with_provider(provider: Arc<dyn Provider>) -> Arc<AgentServer> {
        let core: Arc<dyn AgentCore> = Arc::new(futures::executor::block_on(async {
            AgentCoreNative::builder(Arc::new(agent_store::InMemoryStore::new()))
                .with_provider("mock", provider)
                .build()
                .await
                .unwrap()
        }));
        Arc::new(AgentServer::new(core))
    }

    async fn create_project(client: &reqwest::Client, base: &str) -> Project {
        client
            .post(format!("{base}/v1/projects"))
            .json(&serde_json::json!({
                "name": "agent-server-shutdown-test",
            }))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .await
            .unwrap()
    }

    async fn create_session(client: &reqwest::Client, base: &str, project: &Project) -> Session {
        client
            .post(format!("{base}/v1/projects/{}/sessions", project.id))
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn graceful_shutdown_waits_for_in_flight_turns() {
        let server = make_server_with_provider(Arc::new(MockProvider::new().with_delay(500)));
        let router = build_router(server);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let base = format!("http://{addr}");
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let server_task = tokio::spawn(async move {
            serve_listener_with_shutdown(listener, router, async move {
                let _ = shutdown_rx.await;
            })
            .await
            .unwrap();
        });

        let client = reqwest::Client::new();
        let project = create_project(&client, &base).await;
        let session = create_session(&client, &base, &project).await;

        let turn_client = client.clone();
        let turn_base = base.clone();
        let turn_task = tokio::spawn(async move {
            turn_client
                .post(format!("{turn_base}/v1/sessions/{}/turns", session.id))
                .json(&serde_json::json!({
                    "input": [
                        {
                            "role": "user",
                            "content": [{"type": "text", "text": "finish before shutdown"}]
                        }
                    ]
                }))
                .send()
                .await
                .unwrap()
        });

        tokio::time::sleep(Duration::from_millis(100)).await;
        shutdown_tx.send(()).unwrap();

        let response = tokio::time::timeout(Duration::from_secs(5), turn_task)
            .await
            .unwrap()
            .unwrap();
        assert!(response.status().is_success());

        tokio::time::timeout(Duration::from_secs(5), server_task)
            .await
            .unwrap()
            .unwrap();

        let shutdown_result = reqwest::Client::new()
            .get(format!("{base}/v1/health"))
            .send()
            .await;
        assert!(shutdown_result.is_err());
    }
}
