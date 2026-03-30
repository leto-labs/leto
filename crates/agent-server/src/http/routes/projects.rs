use agent_store::{Project, ProjectUpdate, SessionUpdate};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use super::super::errors::{core_error_response, store_error_response};
use super::super::{AppState, parse_project_id};
use crate::types::{CreateProjectRequest, CreateSessionRequest, ProjectRootRequest};

pub(in crate::http) async fn list_projects(State(server): State<AppState>) -> Response {
    let core = server.core();
    match core.store().projects().list().await {
        Ok(projects) => Json(projects).into_response(),
        Err(error) => store_error_response(error),
    }
}

pub(in crate::http) async fn create_project(
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

pub(in crate::http) async fn create_project_record(
    State(server): State<AppState>,
    Json(project): Json<Project>,
) -> Response {
    let core = server.core();
    match core.store().projects().create(project).await {
        Ok(project) => (StatusCode::CREATED, Json(project)).into_response(),
        Err(error) => store_error_response(error),
    }
}

pub(in crate::http) async fn find_project_by_root(
    State(server): State<AppState>,
    Json(body): Json<ProjectRootRequest>,
) -> Response {
    let core = server.core();
    match core.store().projects().find_by_root(&body.root).await {
        Ok(project) => Json(project).into_response(),
        Err(error) => store_error_response(error),
    }
}

pub(in crate::http) async fn resolve_project(
    State(server): State<AppState>,
    Json(body): Json<ProjectRootRequest>,
) -> Response {
    let core = server.core();
    match core.resolve_or_create_project(body.root).await {
        Ok(project) => Json(project).into_response(),
        Err(error) => core_error_response(error),
    }
}

pub(in crate::http) async fn get_project(
    State(server): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    let project_id = match parse_project_id(&id) {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    let core = server.core();
    match core.project(project_id).await {
        Ok(project) => Json(project).into_response(),
        Err(error) => core_error_response(error),
    }
}

pub(in crate::http) async fn replace_project(
    State(server): State<AppState>,
    Path(id): Path<String>,
    Json(project): Json<Project>,
) -> Response {
    let project_id = match parse_project_id(&id) {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    let core = server.core();
    match core.store().projects().update(project_id, project).await {
        Ok(project) => Json(project).into_response(),
        Err(error) => store_error_response(error),
    }
}

pub(in crate::http) async fn update_project(
    State(server): State<AppState>,
    Path(id): Path<String>,
    Json(update): Json<ProjectUpdate>,
) -> Response {
    let project_id = match parse_project_id(&id) {
        Ok(value) => value,
        Err(response) => return response.into_response(),
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

pub(in crate::http) async fn delete_project(
    State(server): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    let project_id = match parse_project_id(&id) {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    let core = server.core();
    match core.store().projects().delete(project_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => store_error_response(error),
    }
}

pub(in crate::http) async fn list_project_sessions(
    State(server): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    let project_id = match parse_project_id(&id) {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };
    let core = server.core();
    match core.sessions_for_project(project_id).await {
        Ok(sessions) => Json(sessions).into_response(),
        Err(error) => core_error_response(error),
    }
}

pub(in crate::http) async fn create_session(
    State(server): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<CreateSessionRequest>,
) -> Response {
    let project_id = match parse_project_id(&id) {
        Ok(value) => value,
        Err(response) => return response.into_response(),
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
