use std::convert::Infallible;
use std::time::Duration;

use agent_core::CoreEvent;
use axum::extract::{Path, State};
use axum::http::header;
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use axum::response::{IntoResponse, Json, Response};
use futures::StreamExt;

use super::super::errors::store_error_response;
use super::super::{AppState, parse_session_id};
use crate::types::{AgentInfoRecord, AgentServerStatus, HealthResponse};

pub(in crate::http) async fn health() -> impl IntoResponse {
    Json(HealthResponse::current())
}

pub(in crate::http) async fn metrics(State(server): State<AppState>) -> Response {
    match server.status().await {
        Ok(status) => (
            [(
                header::CONTENT_TYPE,
                "text/plain; version=0.0.4; charset=utf-8",
            )],
            prometheus_metrics(&status),
        )
            .into_response(),
        Err(error) => store_error_response(error),
    }
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

fn prometheus_metrics(status: &AgentServerStatus) -> String {
    let version = prometheus_label_value(env!("CARGO_PKG_VERSION"));
    let default_provider = prometheus_label_value(&status.default_provider_name);
    let default_loop = prometheus_label_value(&status.default_loop_name);

    format!(
        concat!(
            "# HELP agent_server_info Static agent-server build information.\n",
            "# TYPE agent_server_info gauge\n",
            "agent_server_info{{version=\"{version}\",default_provider=\"{default_provider}\",default_loop=\"{default_loop}\"}} 1\n",
            "# HELP agent_server_provider_count Number of configured providers.\n",
            "# TYPE agent_server_provider_count gauge\n",
            "agent_server_provider_count {provider_count}\n",
            "# HELP agent_server_loop_count Number of registered loops.\n",
            "# TYPE agent_server_loop_count gauge\n",
            "agent_server_loop_count {loop_count}\n",
            "# HELP agent_server_project_count Number of stored projects.\n",
            "# TYPE agent_server_project_count gauge\n",
            "agent_server_project_count {project_count}\n",
            "# HELP agent_server_session_count Number of stored sessions.\n",
            "# TYPE agent_server_session_count gauge\n",
            "agent_server_session_count {session_count}\n"
        ),
        version = version,
        default_provider = default_provider,
        default_loop = default_loop,
        provider_count = status.provider_names.len(),
        loop_count = status.loop_names.len(),
        project_count = status.project_count,
        session_count = status.session_count,
    )
}

fn prometheus_label_value(value: &str) -> String {
    value.replace('\\', r"\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::prometheus_metrics;
    use crate::types::AgentServerStatus;

    #[test]
    fn prometheus_metrics_escapes_labels_and_reports_counts() {
        let status = AgentServerStatus {
            provider_names: vec!["mock".to_owned(), "backup".to_owned()],
            loop_names: vec!["simple".to_owned(), "robust".to_owned()],
            default_provider_name: r#"mock"primary\east"#.to_owned(),
            default_loop_name: r#"simple"steady\west"#.to_owned(),
            project_count: 3,
            session_count: 5,
        };

        let metrics = prometheus_metrics(&status);

        assert_eq!(
            metrics,
            format!(
                concat!(
                    "# HELP agent_server_info Static agent-server build information.\n",
                    "# TYPE agent_server_info gauge\n",
                    "agent_server_info{{version=\"{}\",default_provider=\"mock\\\"primary\\\\east\",default_loop=\"simple\\\"steady\\\\west\"}} 1\n",
                    "# HELP agent_server_provider_count Number of configured providers.\n",
                    "# TYPE agent_server_provider_count gauge\n",
                    "agent_server_provider_count 2\n",
                    "# HELP agent_server_loop_count Number of registered loops.\n",
                    "# TYPE agent_server_loop_count gauge\n",
                    "agent_server_loop_count 2\n",
                    "# HELP agent_server_project_count Number of stored projects.\n",
                    "# TYPE agent_server_project_count gauge\n",
                    "agent_server_project_count 3\n",
                    "# HELP agent_server_session_count Number of stored sessions.\n",
                    "# TYPE agent_server_session_count gauge\n",
                    "agent_server_session_count 5\n"
                ),
                env!("CARGO_PKG_VERSION")
            )
        );
    }
}
