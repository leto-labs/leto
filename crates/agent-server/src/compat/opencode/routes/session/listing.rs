use std::collections::BTreeMap;

use aide::axum::{
    ApiRouter,
    routing::{get_with, post_with},
};

use super::super::super::types::common::{CompatQuery, ExperimentalSessionListQueryDoc};
use super::super::super::types::session::{
    SessionDoc, SessionIdPath, SessionIdleStatusDoc, SessionIdleStatusKindDoc, SessionInitRequest,
    SessionListQuery, SessionStatusDoc, TodoDoc,
};
use super::super::super::*;
use super::docs::{
    bad_request, inline_json_request, json_response, not_found, op, op_with_query,
    session_parameters,
};

pub(super) fn session_list_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/session",
        get_with(sessions, |operation| {
            op_with_query(
                "session.list",
                "List sessions",
                "Get a list of all OpenCode sessions, sorted by most recently updated.",
            )(operation)
            .parameter_untyped("directory", |param| {
                param.description("Filter sessions by project directory")
            })
            .parameter_untyped("roots", |param| {
                param.description("Only return root sessions (no parentID)")
            })
            .parameter_untyped("start", |param| {
                param.description(
                    "Filter sessions updated on or after this timestamp (milliseconds since epoch)",
                )
            })
            .parameter_untyped("search", |param| {
                param.description("Filter sessions by title (case-insensitive)")
            })
            .parameter_untyped("limit", |param| {
                param.description("Maximum number of sessions to return")
            })
            .with(|op| {
                session_parameters(
                    op,
                    &[
                        "directory",
                        "workspace",
                        "roots",
                        "start",
                        "search",
                        "limit",
                    ],
                )
            })
            .with(|op| json_response::<200, Vec<SessionDoc>>(op, "List of sessions"))
        }),
    )
}

pub(super) fn session_status_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/session/status",
        get_with(session_status, |operation| {
            bad_request(
                op("session.status")(operation)
                    .summary("Get session status")
                    .description("Retrieve the current status of all sessions, including active, idle, and completed states.")
                    .with(|op| session_parameters(op, &["directory", "workspace"]))
                    .with(|op| {
                        json_response::<200, BTreeMap<String, SessionStatusDoc>>(
                            op,
                            "Get session status",
                        )
                    }),
            )
        }),
    )
}

pub(super) fn session_get_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/session/{sessionID}",
        get_with(session_get, |operation| {
            not_found(bad_request(
                op("session.get")(operation)
                    .tag("Session")
                    .summary("Get session")
                    .description("Retrieve detailed information about a specific OpenCode session.")
                    .with(|op| session_parameters(op, &["directory", "workspace", "sessionID"]))
                    .with(|op| json_response::<200, SessionDoc>(op, "Get session")),
            ))
        }),
    )
}

pub(super) fn session_children_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/session/{sessionID}/children",
        get_with(session_children, |operation| {
            not_found(bad_request(
                op("session.children")(operation)
                    .tag("Session")
                    .summary("Get session children")
                    .description("Retrieve all child sessions that were forked from the specified parent session.")
                    .with(|op| session_parameters(op, &["directory", "workspace", "sessionID"]))
                    .with(|op| json_response::<200, Vec<SessionDoc>>(op, "List of children")),
            ))
        }),
    )
}

pub(super) fn session_todo_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/session/{sessionID}/todo",
        get_with(session_todo, |operation| {
            not_found(bad_request(
                op("session.todo")(operation)
                    .summary("Get session todos")
                    .description("Retrieve the todo list associated with a specific session, showing tasks and action items.")
                    .with(|op| session_parameters(op, &["directory", "workspace", "sessionID"]))
                    .with(|op| json_response::<200, Vec<TodoDoc>>(op, "Todo list")),
            ))
        }),
    )
}

pub(super) fn session_init_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/session/{sessionID}/init",
        post_with(session_init, |operation| {
            not_found(bad_request(
                op("session.init")(operation)
                    .summary("Initialize session")
                    .description("Analyze the current application and create an AGENTS.md file with project-specific agent configurations.")
                    .with(|op| session_parameters(op, &["directory", "workspace", "sessionID"]))
                    .with(|op| inline_json_request::<SessionInitRequest>(op, true))
                    .with(|op| json_response::<200, bool>(op, "200")),
            ))
        }),
    )
}

async fn sessions(
    State(server): State<AppState>,
    Query(params): Query<SessionListQuery>,
) -> Response {
    let filters = ExperimentalSessionListQueryDoc {
        directory: params.directory,
        workspace: params.workspace,
        roots: params.roots,
        start: params.start,
        cursor: None,
        search: params.search,
        limit: params.limit,
        archived: None,
    };
    match filtered_sessions(&server, &filters).await {
        Ok(sessions) => Json(
            sessions
                .iter()
                .map(|(session, project, meta)| compat_session(session, project.as_ref(), meta))
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(response) => response,
    }
}

async fn session_status(
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
) -> Response {
    match server.core().store().sessions().list().await {
        Ok(sessions) => {
            let payload = sessions
                .into_iter()
                .map(|session| {
                    (
                        compat_session_id(session.id),
                        SessionStatusDoc::Idle(SessionIdleStatusDoc {
                            status_type: SessionIdleStatusKindDoc::Idle,
                        }),
                    )
                })
                .collect::<BTreeMap<String, SessionStatusDoc>>();
            Json(payload).into_response()
        }
        Err(error) => compat_error("store_error", error.to_string()),
    }
}

pub(super) async fn session_get(
    State(server): State<AppState>,
    Path(SessionIdPath { session_id }): Path<SessionIdPath>,
    Query(_query): Query<CompatQuery>,
) -> Response {
    let session_id = match parse_compat_session_id(&session_id) {
        Ok(id) => id,
        Err(response) => return response,
    };
    let core = server.core();
    let session = match core.session(session_id).await {
        Ok(session) => session,
        Err(error) => return compat_error("core_error", error.to_string()),
    };
    let project = match core.project(session.project_id).await {
        Ok(project) => Some(project),
        Err(_) => None,
    };
    let meta = session_meta(&server, session.id).await;
    Json(compat_session(&session, project.as_ref(), &meta)).into_response()
}

async fn session_children(
    State(server): State<AppState>,
    Path(SessionIdPath { session_id }): Path<SessionIdPath>,
    Query(_query): Query<CompatQuery>,
) -> Response {
    let sessions = match server.core().store().sessions().list().await {
        Ok(sessions) => sessions,
        Err(error) => return compat_error("store_error", error.to_string()),
    };
    let compat = server.compat();
    let metas = compat.session_meta.read().await;
    let children = sessions
        .into_iter()
        .filter_map(|session| {
            let key = compat_session_id(session.id);
            let meta = metas.get(&key)?;
            (meta.parent_id.as_deref() == Some(session_id.as_str())).then_some(session)
        })
        .collect::<Vec<_>>();
    drop(metas);
    let mut result = Vec::new();
    for session in children {
        let project = server.core().project(session.project_id).await.ok();
        let meta = session_meta(&server, session.id).await;
        result.push(compat_session(&session, project.as_ref(), &meta));
    }
    Json(result).into_response()
}

async fn session_todo(
    Path(_path): Path<SessionIdPath>,
    Query(_query): Query<CompatQuery>,
) -> Response {
    Json(Vec::<TodoDoc>::new()).into_response()
}

async fn session_init(
    Path(_path): Path<SessionIdPath>,
    Query(_query): Query<CompatQuery>,
    Json(_body): Json<SessionInitRequest>,
) -> Response {
    Json(true).into_response()
}
