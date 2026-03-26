use aide::axum::{
    ApiRouter,
    routing::{get_with, post_with},
};
use axum::extract::Query;

use super::super::types::common::CompatQuery;
use super::super::types::errors::{BadRequestErrorDoc, NotFoundErrorDoc};
use super::super::types::question::{
    QuestionReplyRequest, QuestionRequestDoc, QuestionRequestIdPath,
};
use super::super::*;

pub(super) fn routes() -> ApiRouter<AppState> {
    ApiRouter::new()
        .merge(question_list_route())
        .merge(question_reply_route())
        .merge(question_reject_route())
}

fn question_list_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/question",
        get_with(question_list, |operation| {
            operation
                .id("question.list")
                .summary("List pending questions")
                .description("Get all pending question requests across all sessions.")
                .response_with::<200, Json<Vec<QuestionRequestDoc>>, _>(|res| {
                    res.description("List of pending questions")
                })
        }),
    )
}

fn question_reply_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/question/{requestID}/reply",
        post_with(question_reply, |operation| {
            operation
                .id("question.reply")
                .summary("Reply to question request")
                .description("Provide answers to a question request from the AI assistant.")
                .response_with::<200, Json<bool>, _>(|res| {
                    res.description("Question answered successfully")
                })
                .response_with::<400, Json<BadRequestErrorDoc>, _>(|res| {
                    res.description("Bad request")
                })
                .response_with::<404, Json<NotFoundErrorDoc>, _>(|res| res.description("Not found"))
        }),
    )
}

fn question_reject_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/question/{requestID}/reject",
        post_with(question_reject, |operation| {
            operation
                .id("question.reject")
                .summary("Reject question request")
                .description("Reject a question request from the AI assistant.")
                .response_with::<200, Json<bool>, _>(|res| {
                    res.description("Question rejected successfully")
                })
                .response_with::<400, Json<BadRequestErrorDoc>, _>(|res| {
                    res.description("Bad request")
                })
                .response_with::<404, Json<NotFoundErrorDoc>, _>(|res| res.description("Not found"))
        }),
    )
}

async fn question_list(
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
) -> Response {
    Json(
        server
            .compat()
            .questions
            .read()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>(),
    )
    .into_response()
}

async fn question_reply(
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
    Path(QuestionRequestIdPath { request_id }): Path<QuestionRequestIdPath>,
    Json(_body): Json<QuestionReplyRequest>,
) -> Response {
    server.compat().questions.write().await.remove(&request_id);
    Json(true).into_response()
}

async fn question_reject(
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
    Path(QuestionRequestIdPath { request_id }): Path<QuestionRequestIdPath>,
) -> Response {
    server.compat().questions.write().await.remove(&request_id);
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
    fn question_list_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_opencode_route_doc(&generate_from_router(
                super::question_list_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_paths(
                &pinned_opencode_openapi(),
                &["/question"],
            ))
        );
    }

    #[test]
    fn question_reply_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_opencode_route_doc(&generate_from_router(
                super::question_reply_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_paths(
                &pinned_opencode_openapi(),
                &["/question/{requestID}/reply"],
            ))
        );
    }

    #[test]
    fn question_reject_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_opencode_route_doc(&generate_from_router(
                super::question_reject_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_paths(
                &pinned_opencode_openapi(),
                &["/question/{requestID}/reject"],
            ))
        );
    }
}
