use aide::axum::{
    ApiRouter,
    routing::{delete_with, patch_with, post_with},
};
use serde_json::Value;

use super::super::super::state::CompatSessionMeta;
use super::super::super::types::common::CompatQuery;
use super::super::super::types::session::{
    RevertRequest, SessionCreateRequest, SessionDoc, SessionForkRequest, SessionIdPath,
    SessionSummarizeRequest, SessionUpdateRequest,
};
use super::super::super::*;
use super::docs::{
    bad_request, inline_json_request, json_response, not_found, op, session_parameters,
};

pub(super) fn session_create_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/session",
        post_with(session_create, |operation| {
            bad_request(
                op("session.create")(operation)
                    .summary("Create session")
                    .description("Create a new OpenCode session for interacting with AI assistants and managing conversations.")
                    .with(|op| inline_json_request::<SessionCreateRequest>(op, false))
                    .with(|op| {
                        json_response::<200, SessionDoc>(op, "Successfully created session")
                    }),
            )
        }),
    )
}

pub(super) fn session_update_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/session/{sessionID}",
        patch_with(session_update, |operation| {
            not_found(bad_request(
                op("session.update")(operation)
                    .summary("Update session")
                    .description("Update properties of an existing session, such as title or other metadata.")
                    .with(|op| session_parameters(op, &["directory", "workspace", "sessionID"]))
                    .with(|op| inline_json_request::<SessionUpdateRequest>(op, true))
                    .with(|op| {
                        json_response::<200, SessionDoc>(op, "Successfully updated session")
                    }),
            ))
        }),
    )
}

pub(super) fn session_delete_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/session/{sessionID}",
        delete_with(session_delete, |operation| {
            not_found(bad_request(
                op("session.delete")(operation)
                    .summary("Delete session")
                    .description("Delete a session and permanently remove all associated data, including messages and history.")
                    .with(|op| session_parameters(op, &["directory", "workspace", "sessionID"]))
                    .with(|op| json_response::<200, bool>(op, "Successfully deleted session")),
            ))
        }),
    )
}

pub(super) fn session_fork_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/session/{sessionID}/fork",
        post_with(session_fork, |operation| {
            op("session.fork")(operation)
                .summary("Fork session")
                .description("Create a new session by forking an existing session at a specific message point.")
                .with(|op| session_parameters(op, &["directory", "workspace", "sessionID"]))
                .with(|op| inline_json_request::<SessionForkRequest>(op, false))
                .with(|op| json_response::<200, SessionDoc>(op, "200"))
        }),
    )
}

pub(super) fn session_abort_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/session/{sessionID}/abort",
        post_with(session_abort, |operation| {
            not_found(bad_request(
                op("session.abort")(operation)
                    .summary("Abort session")
                    .description("Abort an active session and stop any ongoing AI processing or command execution.")
                    .with(|op| session_parameters(op, &["directory", "workspace", "sessionID"]))
                    .with(|op| json_response::<200, bool>(op, "Aborted session")),
            ))
        }),
    )
}

pub(super) fn session_share_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/session/{sessionID}/share",
        post_with(session_share, |operation| {
            not_found(bad_request(
                op("session.share")(operation)
                    .summary("Share session")
                    .description("Create a shareable link for a session, allowing others to view the conversation.")
                    .with(|op| session_parameters(op, &["directory", "workspace", "sessionID"]))
                    .with(|op| {
                        json_response::<200, SessionDoc>(op, "Successfully shared session")
                    }),
            ))
        }),
    )
}

pub(super) fn session_unshare_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/session/{sessionID}/share",
        delete_with(session_unshare, |operation| {
            not_found(bad_request(
                op("session.unshare")(operation)
                    .summary("Unshare session")
                    .description(
                        "Remove the shareable link for a session, making it private again.",
                    )
                    .with(|op| session_parameters(op, &["directory", "workspace", "sessionID"]))
                    .with(|op| {
                        json_response::<200, SessionDoc>(op, "Successfully unshared session")
                    }),
            ))
        }),
    )
}

pub(super) fn session_summarize_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/session/{sessionID}/summarize",
        post_with(session_summarize, |operation| {
            not_found(bad_request(
                op("session.summarize")(operation)
                    .summary("Summarize session")
                    .description("Generate a concise summary of the session using AI compaction to preserve key information.")
                    .with(|op| session_parameters(op, &["directory", "workspace", "sessionID"]))
                    .with(|op| inline_json_request::<SessionSummarizeRequest>(op, true))
                    .with(|op| json_response::<200, bool>(op, "Summarized session")),
            ))
        }),
    )
}

pub(super) fn session_revert_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/session/{sessionID}/revert",
        post_with(session_revert, |operation| {
            not_found(bad_request(
                op("session.revert")(operation)
                    .summary("Revert message")
                    .description("Revert a specific message in a session, undoing its effects and restoring the previous state.")
                    .with(|op| session_parameters(op, &["directory", "workspace", "sessionID"]))
                    .with(|op| inline_json_request::<RevertRequest>(op, true))
                    .with(|op| json_response::<200, SessionDoc>(op, "Updated session")),
            ))
        }),
    )
}

pub(super) fn session_unrevert_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/session/{sessionID}/unrevert",
        post_with(session_unrevert, |operation| {
            not_found(bad_request(
                op("session.unrevert")(operation)
                    .summary("Restore reverted messages")
                    .description("Restore all previously reverted messages in a session.")
                    .with(|op| session_parameters(op, &["directory", "workspace", "sessionID"]))
                    .with(|op| json_response::<200, SessionDoc>(op, "Updated session")),
            ))
        }),
    )
}

async fn session_create(
    State(server): State<AppState>,
    Query(params): Query<CompatQuery>,
    Json(body): Json<SessionCreateRequest>,
) -> Response {
    let project = match resolve_current_project(&server, &params).await {
        Ok(project) => project,
        Err(response) => return response.into_response(),
    };
    let core = server.core();
    let mut session = match core.create_session(project.id).await {
        Ok(session) => session,
        Err(error) => return compat_error("core_error", error.to_string()),
    };
    if let Some(title) = body.title.clone() {
        session = match core
            .update_session(
                session.id,
                SessionUpdate {
                    title: Some(title),
                    ..SessionUpdate::default()
                },
            )
            .await
        {
            Ok(session) => session,
            Err(error) => return compat_error("core_error", error.to_string()),
        };
    }
    let compat_id = compat_session_id(session.id);
    server.compat().session_meta.write().await.insert(
        compat_id,
        CompatSessionMeta {
            parent_id: body.parent_id,
            workspace_id: body.workspace_id,
            archived_at: None,
            share_url: None,
            permission: serde_json::to_value(body.permission).unwrap_or(Value::Null),
        },
    );
    let meta = session_meta(&server, session.id).await;
    Json(compat_session(&session, Some(&project), &meta)).into_response()
}

async fn session_update(
    State(server): State<AppState>,
    Path(SessionIdPath { session_id }): Path<SessionIdPath>,
    Query(_query): Query<CompatQuery>,
    Json(body): Json<SessionUpdateRequest>,
) -> Response {
    let session_id = match parse_compat_session_id(&session_id) {
        Ok(id) => id,
        Err(response) => return response.into_response(),
    };
    let update = SessionUpdate {
        title: body.title.clone(),
        ..SessionUpdate::default()
    };
    let core = server.core();
    let session = match core.update_session(session_id, update).await {
        Ok(session) => session,
        Err(error) => return compat_error("core_error", error.to_string()),
    };
    let key = compat_session_id(session.id);
    {
        let compat = server.compat();
        let mut meta = compat.session_meta.write().await;
        let entry = meta.entry(key).or_default();
        entry.archived_at = body
            .time
            .and_then(|time| time.archived)
            .map(|archived| archived as i64)
            .or(entry.archived_at);
    }
    let project = core.project(session.project_id).await.ok();
    let meta = session_meta(&server, session.id).await;
    Json(compat_session(&session, project.as_ref(), &meta)).into_response()
}

async fn session_delete(
    State(server): State<AppState>,
    Path(SessionIdPath { session_id }): Path<SessionIdPath>,
    Query(_query): Query<CompatQuery>,
) -> Response {
    let session_id = match parse_compat_session_id(&session_id) {
        Ok(id) => id,
        Err(response) => return response.into_response(),
    };
    match server.core().delete_session(session_id).await {
        Ok(()) => {
            server
                .compat()
                .session_meta
                .write()
                .await
                .remove(&compat_session_id(session_id));
            Json(true).into_response()
        }
        Err(error) => compat_error("core_error", error.to_string()),
    }
}

async fn session_fork(
    State(server): State<AppState>,
    Path(SessionIdPath { session_id }): Path<SessionIdPath>,
    Query(_query): Query<CompatQuery>,
    Json(_body): Json<SessionForkRequest>,
) -> Response {
    let parent_id = match parse_compat_session_id(&session_id) {
        Ok(id) => id,
        Err(response) => return response.into_response(),
    };
    let parent = match server.core().session(parent_id).await {
        Ok(session) => session,
        Err(error) => return compat_error("core_error", error.to_string()),
    };
    let mut forked = match server.core().create_session(parent.project_id).await {
        Ok(session) => session,
        Err(error) => return compat_error("core_error", error.to_string()),
    };
    if parent.title.is_some() {
        forked = match server
            .core()
            .update_session(
                forked.id,
                SessionUpdate {
                    title: parent.title.clone(),
                    ..SessionUpdate::default()
                },
            )
            .await
        {
            Ok(session) => session,
            Err(error) => return compat_error("core_error", error.to_string()),
        };
    }
    server.compat().session_meta.write().await.insert(
        compat_session_id(forked.id),
        CompatSessionMeta {
            parent_id: Some(session_id),
            ..CompatSessionMeta::default()
        },
    );
    let project = server.core().project(forked.project_id).await.ok();
    let meta = session_meta(&server, forked.id).await;
    Json(compat_session(&forked, project.as_ref(), &meta)).into_response()
}

async fn session_abort(
    State(server): State<AppState>,
    Path(SessionIdPath { session_id }): Path<SessionIdPath>,
    Query(_query): Query<CompatQuery>,
) -> Response {
    let session_id = match parse_compat_session_id(&session_id) {
        Ok(id) => id,
        Err(response) => return response.into_response(),
    };
    let _ = server.core().cancel_turn(session_id).await;
    Json(true).into_response()
}

async fn session_share(
    State(server): State<AppState>,
    Path(SessionIdPath { session_id }): Path<SessionIdPath>,
    Query(_query): Query<CompatQuery>,
) -> Response {
    let internal = match parse_compat_session_id(&session_id) {
        Ok(id) => id,
        Err(response) => return response.into_response(),
    };
    {
        let compat = server.compat();
        let mut meta = compat.session_meta.write().await;
        meta.entry(compat_session_id(internal))
            .or_default()
            .share_url = Some(format!("https://example.invalid/s/{session_id}"));
    }
    crate::compat::opencode::routes::session::listing::session_get(
        State(server),
        Path(SessionIdPath { session_id }),
        Query(CompatQuery::default()),
    )
    .await
}

async fn session_unshare(
    State(server): State<AppState>,
    Path(SessionIdPath { session_id }): Path<SessionIdPath>,
    Query(_query): Query<CompatQuery>,
) -> Response {
    let internal = match parse_compat_session_id(&session_id) {
        Ok(id) => id,
        Err(response) => return response.into_response(),
    };
    {
        let compat = server.compat();
        let mut meta = compat.session_meta.write().await;
        meta.entry(compat_session_id(internal))
            .or_default()
            .share_url = None;
    }
    crate::compat::opencode::routes::session::listing::session_get(
        State(server),
        Path(SessionIdPath { session_id }),
        Query(CompatQuery::default()),
    )
    .await
}

async fn session_summarize(
    Path(_path): Path<SessionIdPath>,
    Query(_query): Query<CompatQuery>,
    Json(_body): Json<SessionSummarizeRequest>,
) -> Response {
    Json(true).into_response()
}

async fn session_revert(
    State(server): State<AppState>,
    Path(SessionIdPath { session_id }): Path<SessionIdPath>,
    Query(_query): Query<CompatQuery>,
    Json(_body): Json<RevertRequest>,
) -> Response {
    crate::compat::opencode::routes::session::listing::session_get(
        State(server),
        Path(SessionIdPath { session_id }),
        Query(CompatQuery::default()),
    )
    .await
}

async fn session_unrevert(
    State(server): State<AppState>,
    Path(SessionIdPath { session_id }): Path<SessionIdPath>,
    Query(_query): Query<CompatQuery>,
) -> Response {
    crate::compat::opencode::routes::session::listing::session_get(
        State(server),
        Path(SessionIdPath { session_id }),
        Query(CompatQuery::default()),
    )
    .await
}
