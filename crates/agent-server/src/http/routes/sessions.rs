use agent_store::{Session, SessionUpdate, StoredMessage};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use super::super::errors::{core_error_response, store_error_response};
use super::super::{AppState, parse_session_id};
use crate::types::{SessionRuntimeView, TrajectoryRecord};

pub(in crate::http) async fn list_sessions(State(server): State<AppState>) -> Response {
    let core = server.core();
    match core.store().sessions().list().await {
        Ok(sessions) => Json(sessions).into_response(),
        Err(error) => store_error_response(error),
    }
}

pub(in crate::http) async fn create_session_record(
    State(server): State<AppState>,
    Json(session): Json<Session>,
) -> Response {
    let core = server.core();
    match core.store().sessions().create(session).await {
        Ok(session) => (StatusCode::CREATED, Json(session)).into_response(),
        Err(error) => store_error_response(error),
    }
}

pub(in crate::http) async fn get_session(
    State(server): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    let session_id = match parse_session_id(&id) {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    let core = server.core();
    match core.session(session_id).await {
        Ok(session) => Json(session).into_response(),
        Err(error) => core_error_response(error),
    }
}

pub(in crate::http) async fn replace_session(
    State(server): State<AppState>,
    Path(id): Path<String>,
    Json(session): Json<Session>,
) -> Response {
    let session_id = match parse_session_id(&id) {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    let core = server.core();
    match core.store().sessions().update(session_id, session).await {
        Ok(session) => Json(session).into_response(),
        Err(error) => store_error_response(error),
    }
}

pub(in crate::http) async fn update_session(
    State(server): State<AppState>,
    Path(id): Path<String>,
    Json(update): Json<SessionUpdate>,
) -> Response {
    let session_id = match parse_session_id(&id) {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    let core = server.core();
    match core.update_session(session_id, update).await {
        Ok(session) => Json(session).into_response(),
        Err(error) => core_error_response(error),
    }
}

pub(in crate::http) async fn delete_session(
    State(server): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    let session_id = match parse_session_id(&id) {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    let core = server.core();
    match core.delete_session(session_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => core_error_response(error),
    }
}

pub(in crate::http) async fn list_messages(
    State(server): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    let session_id = match parse_session_id(&id) {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    let core = server.core();
    match core.messages(session_id).await {
        Ok(messages) => Json(messages).into_response(),
        Err(error) => core_error_response(error),
    }
}

pub(in crate::http) async fn replace_messages(
    State(server): State<AppState>,
    Path(id): Path<String>,
    Json(messages): Json<Vec<StoredMessage>>,
) -> Response {
    let session_id = match parse_session_id(&id) {
        Ok(value) => value,
        Err(response) => return response.into_response(),
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

pub(in crate::http) async fn delete_messages(
    State(server): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    let session_id = match parse_session_id(&id) {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    let core = server.core();
    match core.store().messages().delete_for_session(session_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => store_error_response(error),
    }
}

pub(in crate::http) async fn get_trajectory(
    State(server): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    let session_id = match parse_session_id(&id) {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    let core = server.core();
    match core.trajectory(session_id).await {
        Ok(trajectory) => Json(trajectory).into_response(),
        Err(error) => core_error_response(error),
    }
}

pub(in crate::http) async fn upsert_trajectory(
    State(server): State<AppState>,
    Path(id): Path<String>,
    Json(trajectory): Json<atif::Trajectory>,
) -> Response {
    let session_id = match parse_session_id(&id) {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    let core = server.core();
    match core.upsert_trajectory(session_id, trajectory).await {
        Ok(trajectory) => Json(trajectory).into_response(),
        Err(error) => core_error_response(error),
    }
}

pub(in crate::http) async fn delete_trajectory(
    State(server): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    let session_id = match parse_session_id(&id) {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    let core = server.core();
    match core.store().trajectories().delete(session_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => store_error_response(error),
    }
}

pub(in crate::http) async fn list_trajectories(State(server): State<AppState>) -> Response {
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

pub(in crate::http) async fn get_runtime_view(
    State(server): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    let session_id = match parse_session_id(&id) {
        Ok(value) => value,
        Err(response) => return response.into_response(),
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
