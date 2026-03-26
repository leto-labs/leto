use aide::axum::{
    ApiRouter,
    routing::{get_with, post_with},
};
use axum::extract::Query;

use super::super::types::common::CompatQuery;
use super::super::types::errors::{BadRequestErrorDoc, NotFoundErrorDoc};
use super::super::types::permission::{
    PermissionReplyRequest, PermissionRequestDoc, PermissionRequestIdPath,
};
use super::super::*;

// This route family is intentionally small and is a good place to study how the
// compat server combines:
// - Axum runtime handlers (`permission_list`, `permission_reply`)
// - aide route registration (`ApiRouter`, `get_with`, `post_with`)
// - route-local OpenAPI metadata attached directly in this file
//
// The important pattern to notice is that the runtime handler and the OpenAPI
// operation metadata are declared together. `aide` uses the route tree built
// here as input when the compat router generates `/doc`.
pub(super) fn routes() -> ApiRouter<AppState> {
    ApiRouter::new()
        .merge(permission_list_route())
        .merge(permission_reply_route())
}

fn permission_list_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/permission",
        get_with(permission_list, |operation| {
            // `get_with` comes from `aide::axum`. It registers a GET route
            // and also gives us an OpenAPI operation-transform closure for
            // that route. The closure below only affects documentation.
            operation
                .id("permission.list")
                .summary("List pending permissions")
                .description("Get all pending permission requests across all sessions.")
                .response_with::<200, Json<Vec<PermissionRequestDoc>>, _>(|res| {
                    res.description("List of pending permissions")
                })
        }),
    )
}

fn permission_reply_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/permission/{requestID}/reply",
        post_with(permission_reply, |operation| {
            // The route path contains `{requestID}`. Axum will deserialize
            // that path parameter into `PermissionRequestIdPath` because
            // the handler takes `Path<PermissionRequestIdPath>`.
            //
            // The JSON request body comes from the handler's
            // `Json<PermissionReplyRequest>` extractor. For this experiment we
            // keep the route docs as close to stock `axum + aide` as possible
            // and only add the explicit response metadata that cannot be
            // inferred from the opaque `Response` return type.
            operation
                .id("permission.reply")
                .summary("Respond to permission request")
                .description("Approve or deny a permission request from the AI assistant.")
                .response_with::<200, Json<bool>, _>(|res| {
                    res.description("Permission processed successfully")
                })
                .response_with::<400, Json<BadRequestErrorDoc>, _>(|res| {
                    res.description("Bad request")
                })
                .response_with::<404, Json<NotFoundErrorDoc>, _>(|res| res.description("Not found"))
        }),
    )
}

async fn permission_list(
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
) -> Response {
    // `State<AppState>` and `Query<CompatQuery>` are Axum extractors. Axum
    // parses them from the incoming request before the handler body runs.
    //
    // `CompatQuery` is currently unused by this handler, but we still accept
    // it because it is part of the compat contract and therefore part of the
    // generated OpenAPI operation as well.
    Json(
        server
            .compat()
            .permissions
            .read()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>(),
    )
    .into_response()
}

async fn permission_reply(
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
    Path(PermissionRequestIdPath { request_id }): Path<PermissionRequestIdPath>,
    Json(_body): Json<PermissionReplyRequest>,
) -> Response {
    // This handler is intentionally simple at runtime:
    // - deserialize query/path/body using Axum extractors
    // - remove the pending request from in-memory compat state
    // - return `true`
    //
    // Even though `_body` is not used yet, parsing it here is still important:
    // the runtime and the OpenAPI contract both go through the same DTO,
    // `PermissionReplyRequest`.
    server
        .compat()
        .permissions
        .write()
        .await
        .remove(&request_id);
    Json(true).into_response()
}

#[cfg(test)]
mod tests {
    use crate::{
        compat::opencode::test_utils::{
            normalize_opencode_route_doc, opencode_openapi_options, pinned_opencode_openapi,
        },
        utils::openapi::{generate_from_router, subset_for_paths},
    };

    #[test]
    fn permission_list_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_opencode_route_doc(&generate_from_router(
                super::permission_list_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_paths(
                &pinned_opencode_openapi(),
                &["/permission"],
            ))
        );
    }

    #[test]
    fn permission_reply_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_opencode_route_doc(&generate_from_router(
                super::permission_reply_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_paths(
                &pinned_opencode_openapi(),
                &["/permission/{requestID}/reply"],
            ))
        );
    }
}
