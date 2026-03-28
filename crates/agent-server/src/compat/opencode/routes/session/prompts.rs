use aide::axum::ApiRouter;
use aide::axum::routing::post_with;
use axum::http::StatusCode as HttpStatusCode;
use axum::response::NoContent;
use futures::StreamExt;

use super::super::super::types::common::CompatQuery;
use super::super::super::types::session::{
    AssistantMessageDoc, AssistantMessageWithPartsDoc, CommandRequest, PermissionReplyRequest,
    PromptRequest, SessionIdPath, SessionPermissionPath, ShellRequest,
};
use super::super::super::*;
use super::docs::{
    bad_request, inline_json_request, json_response, not_found, op, session_parameters,
};

pub(super) fn session_prompt_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/session/{sessionID}/message",
        post_with(session_prompt, |operation| {
            not_found(bad_request(
                op("session.prompt")(operation)
                    .summary("Send message")
                    .description(
                        "Create and send a new message to a session, streaming the AI response.",
                    )
                    .with(|op| session_parameters(op, &["directory", "workspace", "sessionID"]))
                    .with(|op| inline_json_request::<PromptRequest>(op, true))
                    .with(|op| {
                        json_response::<200, AssistantMessageWithPartsDoc>(op, "Created message")
                    }),
            ))
        }),
    )
}

pub(super) fn session_prompt_async_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/session/{sessionID}/prompt_async",
        post_with(session_prompt_async, |operation| {
            not_found(bad_request(
                op("session.prompt_async")(operation)
                    .summary("Send async message")
                    .description("Create and send a new message to a session asynchronously, starting the session if needed and returning immediately.")
                    .with(|op| session_parameters(op, &["directory", "workspace", "sessionID"]))
                    .with(|op| inline_json_request::<PromptRequest>(op, true))
                    .response_with::<204, NoContent, _>(|res| res.description("Prompt accepted")),
            ))
        }),
    )
}

pub(super) fn session_command_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/session/{sessionID}/command",
        post_with(session_command, |operation| {
            not_found(bad_request(
                op("session.command")(operation)
                    .summary("Send command")
                    .description(
                        "Send a new command to a session for execution by the AI assistant.",
                    )
                    .with(|op| session_parameters(op, &["directory", "workspace", "sessionID"]))
                    .with(|op| inline_json_request::<CommandRequest>(op, true))
                    .with(|op| {
                        json_response::<200, AssistantMessageWithPartsDoc>(op, "Created message")
                    }),
            ))
        }),
    )
}

pub(super) fn session_shell_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/session/{sessionID}/shell",
        post_with(session_shell, |operation| {
            not_found(bad_request(
                op("session.shell")(operation)
                    .summary("Run shell command")
                    .description("Execute a shell command within the session context and return the AI's response.")
                    .with(|op| session_parameters(op, &["directory", "workspace", "sessionID"]))
                    .with(|op| inline_json_request::<ShellRequest>(op, true))
                    .with(|op| json_response::<200, AssistantMessageDoc>(op, "Created message")),
            ))
        }),
    )
}

pub(super) fn session_permission_reply_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/session/{sessionID}/permissions/{permissionID}",
        post_with(session_permission_reply, |operation| {
            not_found(bad_request(
                op("permission.respond")(operation)
                    .with(|mut op| {
                        op.inner_mut().deprecated = true;
                        op
                    })
                    .summary("Respond to permission")
                    .description("Approve or deny a permission request from the AI assistant.")
                    .with(|op| {
                        session_parameters(
                            op,
                            &["directory", "workspace", "sessionID", "permissionID"],
                        )
                    })
                    .with(|op| inline_json_request::<PermissionReplyRequest>(op, true))
                    .with(|op| json_response::<200, bool>(op, "Permission processed successfully")),
            ))
        }),
    )
}

async fn session_prompt(
    State(server): State<AppState>,
    Path(SessionIdPath { session_id }): Path<SessionIdPath>,
    Query(_query): Query<CompatQuery>,
    Json(body): Json<PromptRequest>,
) -> Response {
    prompt_like(&server, &session_id, body, PromptKind::Message).await
}

async fn session_prompt_async(
    State(server): State<AppState>,
    Path(SessionIdPath { session_id }): Path<SessionIdPath>,
    Query(_query): Query<CompatQuery>,
    Json(body): Json<PromptRequest>,
) -> Response {
    let internal = match parse_compat_session_id(&session_id) {
        Ok(id) => id,
        Err(response) => return response,
    };
    let input = prompt_input_from_body(&body);
    let core = server.core();
    tokio::spawn(async move {
        if let Ok(stream) = core.turn(internal, input).await {
            futures::pin_mut!(stream);
            while stream.next().await.is_some() {}
        }
    });
    HttpStatusCode::NO_CONTENT.into_response()
}

async fn session_command(
    State(server): State<AppState>,
    Path(SessionIdPath { session_id }): Path<SessionIdPath>,
    Query(_query): Query<CompatQuery>,
    Json(body): Json<CommandRequest>,
) -> Response {
    prompt_command_like(&server, &session_id, body).await
}

async fn session_shell(
    State(server): State<AppState>,
    Path(SessionIdPath { session_id }): Path<SessionIdPath>,
    Query(_query): Query<CompatQuery>,
    Json(body): Json<ShellRequest>,
) -> Response {
    prompt_shell_like(&server, &session_id, body).await
}

async fn session_permission_reply(
    State(server): State<AppState>,
    Path(SessionPermissionPath {
        session_id: _session_id,
        permission_id,
    }): Path<SessionPermissionPath>,
    Query(_query): Query<CompatQuery>,
    Json(_body): Json<PermissionReplyRequest>,
) -> Response {
    server
        .compat()
        .permissions
        .write()
        .await
        .remove(&permission_id);
    Json(true).into_response()
}
