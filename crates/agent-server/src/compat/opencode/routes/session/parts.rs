use aide::axum::{
    ApiRouter,
    routing::{delete_with, patch_with},
};
use serde_json::Value;

use super::super::super::types::common::CompatQuery;
use super::super::super::types::session::{PartDoc, SessionMessagePartPath};
use super::super::super::*;
use super::docs::{bad_request, json_response, not_found, op, session_parameters};

pub(super) fn session_part_update_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/session/{sessionID}/message/{messageID}/part/{partID}",
        patch_with(session_part_update, |operation| {
            not_found(bad_request(
                op("part.update")(operation)
                    .description("Update a part in a message")
                    .with(|op| {
                        session_parameters(
                            op,
                            &["directory", "workspace", "sessionID", "messageID", "partID"],
                        )
                    })
                    .with(|op| json_response::<200, PartDoc>(op, "Successfully updated part")),
            ))
        }),
    )
}

pub(super) fn session_part_delete_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/session/{sessionID}/message/{messageID}/part/{partID}",
        delete_with(session_part_delete, |operation| {
            not_found(bad_request(
                op("part.delete")(operation)
                    .description("Delete a part from a message")
                    .with(|op| {
                        session_parameters(
                            op,
                            &["directory", "workspace", "sessionID", "messageID", "partID"],
                        )
                    })
                    .with(|op| json_response::<200, bool>(op, "Successfully deleted part")),
            ))
        }),
    )
}

async fn session_part_delete(
    State(server): State<AppState>,
    Path(SessionMessagePartPath {
        session_id,
        message_id,
        part_id,
    }): Path<SessionMessagePartPath>,
    Query(_query): Query<CompatQuery>,
) -> Response {
    mutate_message_part(&server, &session_id, &message_id, &part_id, None).await
}

async fn session_part_update(
    State(server): State<AppState>,
    Path(SessionMessagePartPath {
        session_id,
        message_id,
        part_id,
    }): Path<SessionMessagePartPath>,
    Query(_query): Query<CompatQuery>,
    Json(body): Json<PartDoc>,
) -> Response {
    let body = serde_json::to_value(body).unwrap_or(Value::Null);
    mutate_message_part(&server, &session_id, &message_id, &part_id, Some(body)).await
}
