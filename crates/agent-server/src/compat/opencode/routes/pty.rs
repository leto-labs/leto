use aide::axum::{
    ApiRouter,
    routing::{delete_with, get_with, post_with, put_with},
};
use axum::extract::Query;

use super::super::types::common::CompatQuery;
use super::super::types::errors::{BadRequestErrorDoc, NotFoundErrorDoc};
use super::super::types::pty::{PtyCreateRequest, PtyDoc, PtyIdPath, PtyUpdateRequest};
use super::super::*;

pub(super) fn routes() -> ApiRouter<AppState> {
    ApiRouter::new()
        .merge(pty_list_route())
        .merge(pty_create_route())
        .merge(pty_get_route())
        .merge(pty_update_route())
        .merge(pty_remove_route())
        .merge(pty_connect_route())
}

fn pty_list_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/pty",
        get_with(pty_list, |operation| {
            operation
                .id("pty.list")
                .summary("List PTY sessions")
                .description(
                    "Get a list of all active pseudo-terminal (PTY) sessions managed by OpenCode.",
                )
                .response_with::<200, Json<Vec<PtyDoc>>, _>(|res| {
                    res.description("List of sessions")
                })
        }),
    )
}

fn pty_create_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/pty",
        post_with(pty_create, |operation| {
            operation
                .id("pty.create")
                .summary("Create PTY session")
                .description("Create a new pseudo-terminal (PTY) session for running shell commands and processes.")
                .response_with::<200, Json<PtyDoc>, _>(|res| {
                    res.description("Created session")
                })
                .response_with::<400, Json<BadRequestErrorDoc>, _>(|res| {
                    res.description("Bad request")
                })
        }),
    )
}

fn pty_get_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/pty/{ptyID}",
        get_with(pty_get, |operation| {
            operation
                .id("pty.get")
                .summary("Get PTY session")
                .description(
                    "Retrieve detailed information about a specific pseudo-terminal (PTY) session.",
                )
                .response_with::<200, Json<PtyDoc>, _>(|res| res.description("Session info"))
                .response_with::<404, Json<NotFoundErrorDoc>, _>(|res| res.description("Not found"))
        }),
    )
}

fn pty_update_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/pty/{ptyID}",
        put_with(pty_update, |operation| {
            operation
                .id("pty.update")
                .summary("Update PTY session")
                .description("Update properties of an existing pseudo-terminal (PTY) session.")
                .response_with::<200, Json<PtyDoc>, _>(|res| res.description("Updated session"))
                .response_with::<400, Json<BadRequestErrorDoc>, _>(|res| {
                    res.description("Bad request")
                })
        }),
    )
}

fn pty_remove_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/pty/{ptyID}",
        delete_with(pty_remove, |operation| {
            operation
                .id("pty.remove")
                .summary("Remove PTY session")
                .description("Remove and terminate a specific pseudo-terminal (PTY) session.")
                .response_with::<200, Json<bool>, _>(|res| res.description("Session removed"))
                .response_with::<404, Json<NotFoundErrorDoc>, _>(|res| res.description("Not found"))
        }),
    )
}

fn pty_connect_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/pty/{ptyID}/connect",
        get_with(pty_connect, |operation| {
            operation
                .id("pty.connect")
                .summary("Connect to PTY session")
                .description("Establish a WebSocket connection to interact with a pseudo-terminal (PTY) session in real-time.")
                .response_with::<200, Json<bool>, _>(|res| {
                    res.description("Connected session")
                })
                .response_with::<404, Json<NotFoundErrorDoc>, _>(|res| {
                    res.description("Not found")
                })
        }),
    )
}

async fn pty_list(State(server): State<AppState>, Query(_query): Query<CompatQuery>) -> Response {
    Json(
        server
            .compat()
            .ptys
            .read()
            .await
            .values()
            .map(CompatPty::as_doc)
            .collect::<Vec<_>>(),
    )
    .into_response()
}

async fn pty_create(
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
    Json(body): Json<PtyCreateRequest>,
) -> Response {
    let pty = CompatPty::new(&body);
    let payload = pty.as_doc();
    server
        .compat()
        .ptys
        .write()
        .await
        .insert(pty.id.clone(), pty);
    Json(payload).into_response()
}

async fn pty_get(
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
    Path(PtyIdPath { pty_id }): Path<PtyIdPath>,
) -> Response {
    match server.compat().ptys.read().await.get(&pty_id) {
        Some(pty) => Json(pty.as_doc()).into_response(),
        None => compat_error("not_found", "pty not found"),
    }
}

async fn pty_update(
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
    Path(PtyIdPath { pty_id }): Path<PtyIdPath>,
    Json(body): Json<PtyUpdateRequest>,
) -> Response {
    let compat = server.compat();
    let mut ptys = compat.ptys.write().await;
    let pty = match ptys.get_mut(&pty_id) {
        Some(pty) => pty,
        None => return compat_error("not_found", "pty not found"),
    };
    if let Some(title) = body.title {
        pty.title = title;
    }
    Json(pty.as_doc()).into_response()
}

async fn pty_remove(
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
    Path(PtyIdPath { pty_id }): Path<PtyIdPath>,
) -> Response {
    server.compat().ptys.write().await.remove(&pty_id);
    Json(true).into_response()
}

async fn pty_connect(
    Query(_query): Query<CompatQuery>,
    Path(PtyIdPath { pty_id: _ }): Path<PtyIdPath>,
) -> Response {
    Json(true).into_response()
}

#[cfg(test)]
mod tests {
    use crate::{
        compat::opencode::test_utils::{
            normalize_opencode_route_doc, opencode_openapi_options, pinned_opencode_openapi,
        },
        utils::openapi::{generate_from_router, subset_for_operations},
    };

    #[test]
    fn pty_list_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_opencode_route_doc(&generate_from_router(
                super::pty_list_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/pty", "get")],
            ))
        );
    }

    #[test]
    fn pty_create_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_opencode_route_doc(&generate_from_router(
                super::pty_create_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/pty", "post")],
            ))
        );
    }

    #[test]
    fn pty_get_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_opencode_route_doc(&generate_from_router(
                super::pty_get_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/pty/{ptyID}", "get")],
            ))
        );
    }

    #[test]
    fn pty_update_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_opencode_route_doc(&generate_from_router(
                super::pty_update_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/pty/{ptyID}", "put")],
            ))
        );
    }

    #[test]
    fn pty_remove_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_opencode_route_doc(&generate_from_router(
                super::pty_remove_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/pty/{ptyID}", "delete")],
            ))
        );
    }

    #[test]
    fn pty_connect_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_opencode_route_doc(&generate_from_router(
                super::pty_connect_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/pty/{ptyID}/connect", "get")],
            ))
        );
    }
}
