use aide::axum::{
    ApiRouter,
    routing::{delete_with, get_with},
};

use super::super::super::types::common::CompatQuery;
use super::super::super::types::files::FileDiffDoc;
use super::super::super::types::session::{
    MessageListQuery, MessageWithPartsDoc, SessionDiffQuery, SessionIdPath, SessionMessagePath,
};
use super::super::super::*;
use super::docs::{bad_request, json_response, not_found, op, session_parameters};

pub(super) fn session_diff_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/session/{sessionID}/diff",
        get_with(session_diff, |operation| {
            op("session.diff")(operation)
                .summary("Get message diff")
                .description("Get the file changes (diff) that resulted from a specific user message in the session.")
                .with(|op| {
                    session_parameters(op, &["directory", "workspace", "sessionID", "messageID"])
                })
                .with(|op| {
                    json_response::<200, Vec<FileDiffDoc>>(op, "Successfully retrieved diff")
                })
        }),
    )
}

pub(super) fn session_messages_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/session/{sessionID}/message",
        get_with(session_messages, |operation| {
            not_found(bad_request(
                op("session.messages")(operation)
                    .summary("Get session messages")
                    .description("Retrieve all messages in a session, including user prompts and AI responses.")
                    .parameter_untyped("limit", |param| {
                        param.description("Maximum number of messages to return")
                    })
                    .with(|op| {
                        session_parameters(
                            op,
                            &["directory", "workspace", "sessionID", "limit", "before"],
                        )
                    })
                    .with(|op| {
                        json_response::<200, Vec<MessageWithPartsDoc>>(
                            op,
                            "List of messages",
                        )
                    }),
            ))
        }),
    )
}

pub(super) fn session_message_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/session/{sessionID}/message/{messageID}",
        get_with(session_message, |operation| {
            not_found(bad_request(
                op("session.message")(operation)
                    .summary("Get message")
                    .description("Retrieve a specific message from a session by its message ID.")
                    .with(|op| {
                        session_parameters(
                            op,
                            &["directory", "workspace", "sessionID", "messageID"],
                        )
                    })
                    .with(|op| json_response::<200, MessageWithPartsDoc>(op, "Message")),
            ))
        }),
    )
}

pub(super) fn session_message_delete_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/session/{sessionID}/message/{messageID}",
        delete_with(session_message_delete, |operation| {
            not_found(bad_request(
                op("session.deleteMessage")(operation)
                    .summary("Delete message")
                    .description("Permanently delete a specific message (and all of its parts) from a session. This does not revert any file changes that may have been made while processing the message.")
                    .with(|op| {
                        session_parameters(
                            op,
                            &["directory", "workspace", "sessionID", "messageID"],
                        )
                    })
                    .with(|op| json_response::<200, bool>(op, "Successfully deleted message")),
            ))
        }),
    )
}

async fn session_diff(
    Path(_path): Path<SessionIdPath>,
    Query(_query): Query<SessionDiffQuery>,
) -> Response {
    Json(Vec::<FileDiffDoc>::new()).into_response()
}

async fn session_messages(
    State(server): State<AppState>,
    Path(SessionIdPath { session_id }): Path<SessionIdPath>,
    Query(params): Query<MessageListQuery>,
) -> Response {
    let session_id = match parse_compat_session_id(&session_id) {
        Ok(id) => id,
        Err(response) => return response.into_response(),
    };
    let mut messages = match server.core().messages(session_id).await {
        Ok(messages) => messages,
        Err(error) => return compat_error("core_error", error.to_string()),
    };
    if let Some(limit) = params.limit {
        messages.truncate(limit);
    }
    Json(compat_messages_with_parts(&messages)).into_response()
}

async fn session_message(
    State(server): State<AppState>,
    Path(SessionMessagePath {
        session_id,
        message_id,
    }): Path<SessionMessagePath>,
    Query(_query): Query<CompatQuery>,
) -> Response {
    let session_id = match parse_compat_session_id(&session_id) {
        Ok(id) => id,
        Err(response) => return response.into_response(),
    };
    let message_id = match parse_compat_message_id(&message_id) {
        Ok(id) => id,
        Err(response) => return response.into_response(),
    };
    match server.core().messages(session_id).await {
        Ok(messages) => match messages.iter().position(|message| message.id == message_id) {
            Some(index) => Json(compat_message_with_parts(
                &messages[index],
                index.checked_sub(1).map(|parent| messages[parent].id),
            ))
            .into_response(),
            None => compat_error("not_found", "message not found"),
        },
        Err(error) => compat_error("core_error", error.to_string()),
    }
}

async fn session_message_delete(
    State(server): State<AppState>,
    Path(SessionMessagePath {
        session_id,
        message_id,
    }): Path<SessionMessagePath>,
    Query(_query): Query<CompatQuery>,
) -> Response {
    let session_id = match parse_compat_session_id(&session_id) {
        Ok(id) => id,
        Err(response) => return response.into_response(),
    };
    let message_id = match parse_compat_message_id(&message_id) {
        Ok(id) => id,
        Err(response) => return response.into_response(),
    };
    let messages = match server.core().messages(session_id).await {
        Ok(messages) => messages,
        Err(error) => return compat_error("core_error", error.to_string()),
    };
    let filtered = messages
        .into_iter()
        .filter(|message| message.id != message_id)
        .collect::<Vec<_>>();
    match server
        .core()
        .store()
        .messages()
        .replace_for_session(session_id, filtered)
        .await
    {
        Ok(_) => Json(true).into_response(),
        Err(error) => compat_error("store_error", error.to_string()),
    }
}
