use std::collections::BTreeMap;

use aide::axum::{
    ApiRouter,
    routing::{delete_with, get_with, patch_with, post_with},
};
use aide::openapi::{MediaType, ReferenceOr, RequestBody, SchemaObject, StatusCode};
use aide::transform::{TransformOperation, TransformResponse};
use axum::http::StatusCode as HttpStatusCode;
use axum::response::NoContent;
use schemars::{JsonSchema, schema_for};
use serde_json::Value;

use super::super::types::common::CompatQuery;
use super::super::types::errors::{BadRequestErrorDoc, NotFoundErrorDoc};
use super::super::types::files::FileDiffDoc;
use super::super::types::session::{
    AssistantMessageDoc, AssistantMessageWithPartsDoc, CommandRequest, MessageListQuery,
    MessageWithPartsDoc, PartDoc, PermissionReplyRequest, PromptRequest, RevertRequest,
    SessionCreateRequest, SessionDiffQuery, SessionDoc, SessionForkRequest, SessionIdPath,
    SessionIdleStatusDoc, SessionIdleStatusKindDoc, SessionInitRequest, SessionListQuery,
    SessionMessagePartPath, SessionMessagePath, SessionPermissionPath, SessionStatusDoc,
    SessionSummarizeRequest, SessionUpdateRequest, ShellRequest, TodoDoc,
};
use super::super::*;

pub(super) fn routes() -> ApiRouter<AppState> {
    ApiRouter::new()
        .merge(session_list_route())
        .merge(session_create_route())
        .merge(session_status_route())
        .merge(session_get_route())
        .merge(session_update_route())
        .merge(session_delete_route())
        .merge(session_children_route())
        .merge(session_todo_route())
        .merge(session_init_route())
        .merge(session_fork_route())
        .merge(session_abort_route())
        .merge(session_share_route())
        .merge(session_unshare_route())
        .merge(session_diff_route())
        .merge(session_summarize_route())
        .merge(session_messages_route())
        .merge(session_prompt_route())
        .merge(session_message_route())
        .merge(session_message_delete_route())
        .merge(session_part_update_route())
        .merge(session_part_delete_route())
        .merge(session_prompt_async_route())
        .merge(session_command_route())
        .merge(session_shell_route())
        .merge(session_revert_route())
        .merge(session_unrevert_route())
        .merge(session_permission_reply_route())
}

fn session_list_route() -> ApiRouter<AppState> {
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
                parameter_order(
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

fn session_create_route() -> ApiRouter<AppState> {
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

fn session_status_route() -> ApiRouter<AppState> {
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

fn session_get_route() -> ApiRouter<AppState> {
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

fn session_update_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/session/{sessionID}",
        patch_with(session_update, |operation| {
            not_found(
                bad_request(
                    op("session.update")(operation)
                        .summary("Update session")
                        .description("Update properties of an existing session, such as title or other metadata.")
                        .with(|op| {
                            session_parameters(op, &["directory", "workspace", "sessionID"])
                        })
                        .with(|op| inline_json_request::<SessionUpdateRequest>(op, true))
                        .with(|op| {
                            json_response::<200, SessionDoc>(op, "Successfully updated session")
                        }),
                ),
            )
        }),
    )
}

fn session_delete_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/session/{sessionID}",
        delete_with(session_delete, |operation| {
            not_found(
                bad_request(
                    op("session.delete")(operation)
                        .summary("Delete session")
                        .description("Delete a session and permanently remove all associated data, including messages and history.")
                        .with(|op| {
                            session_parameters(op, &["directory", "workspace", "sessionID"])
                        })
                        .with(|op| {
                            json_response::<200, bool>(op, "Successfully deleted session")
                        }),
                ),
            )
        }),
    )
}

fn session_children_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/session/{sessionID}/children",
        get_with(session_children, |operation| {
            not_found(
                bad_request(
                    op("session.children")(operation)
                        .tag("Session")
                        .summary("Get session children")
                        .description("Retrieve all child sessions that were forked from the specified parent session.")
                        .with(|op| {
                            session_parameters(op, &["directory", "workspace", "sessionID"])
                        })
                        .with(|op| json_response::<200, Vec<SessionDoc>>(op, "List of children")),
                ),
            )
        }),
    )
}

fn session_todo_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/session/{sessionID}/todo",
        get_with(session_todo, |operation| {
            not_found(
                bad_request(
                    op("session.todo")(operation)
                        .summary("Get session todos")
                        .description("Retrieve the todo list associated with a specific session, showing tasks and action items.")
                        .with(|op| {
                            session_parameters(op, &["directory", "workspace", "sessionID"])
                        })
                        .with(|op| json_response::<200, Vec<TodoDoc>>(op, "Todo list")),
                ),
            )
        }),
    )
}

fn session_init_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/session/{sessionID}/init",
        post_with(session_init, |operation| {
            not_found(
                bad_request(
                    op("session.init")(operation)
                        .summary("Initialize session")
                        .description("Analyze the current application and create an AGENTS.md file with project-specific agent configurations.")
                        .with(|op| {
                            session_parameters(op, &["directory", "workspace", "sessionID"])
                        })
                        .with(|op| inline_json_request::<SessionInitRequest>(op, true))
                        .with(|op| json_response::<200, bool>(op, "200")),
                ),
            )
        }),
    )
}

fn session_fork_route() -> ApiRouter<AppState> {
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

fn session_abort_route() -> ApiRouter<AppState> {
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

fn session_share_route() -> ApiRouter<AppState> {
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

fn session_unshare_route() -> ApiRouter<AppState> {
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

fn session_diff_route() -> ApiRouter<AppState> {
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

fn session_summarize_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/session/{sessionID}/summarize",
        post_with(session_summarize, |operation| {
            not_found(
                bad_request(
                    op("session.summarize")(operation)
                        .summary("Summarize session")
                        .description("Generate a concise summary of the session using AI compaction to preserve key information.")
                        .with(|op| {
                            session_parameters(op, &["directory", "workspace", "sessionID"])
                        })
                        .with(|op| inline_json_request::<SessionSummarizeRequest>(op, true))
                        .with(|op| json_response::<200, bool>(op, "Summarized session")),
                ),
            )
        }),
    )
}

fn session_messages_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/session/{sessionID}/message",
        get_with(session_messages, |operation| {
            not_found(
                bad_request(
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
                ),
            )
        }),
    )
}

fn session_prompt_route() -> ApiRouter<AppState> {
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

fn session_message_route() -> ApiRouter<AppState> {
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

fn session_message_delete_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/session/{sessionID}/message/{messageID}",
        delete_with(session_message_delete, |operation| {
            not_found(
                bad_request(
                    op("session.deleteMessage")(operation)
                        .summary("Delete message")
                        .description("Permanently delete a specific message (and all of its parts) from a session. This does not revert any file changes that may have been made while processing the message.")
                        .with(|op| {
                            session_parameters(
                                op,
                                &["directory", "workspace", "sessionID", "messageID"],
                            )
                        })
                        .with(|op| {
                            json_response::<200, bool>(op, "Successfully deleted message")
                        }),
                ),
            )
        }),
    )
}

fn session_part_update_route() -> ApiRouter<AppState> {
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

fn session_part_delete_route() -> ApiRouter<AppState> {
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

fn session_prompt_async_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/session/{sessionID}/prompt_async",
        post_with(session_prompt_async, |operation| {
            not_found(
                bad_request(
                    op("session.prompt_async")(operation)
                        .summary("Send async message")
                        .description("Create and send a new message to a session asynchronously, starting the session if needed and returning immediately.")
                        .with(|op| {
                            session_parameters(op, &["directory", "workspace", "sessionID"])
                        })
                        .with(|op| inline_json_request::<PromptRequest>(op, true))
                        .response_with::<204, NoContent, _>(|res| {
                            res.description("Prompt accepted")
                        }),
                ),
            )
        }),
    )
}

fn session_command_route() -> ApiRouter<AppState> {
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

fn session_shell_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/session/{sessionID}/shell",
        post_with(session_shell, |operation| {
            not_found(
                bad_request(
                    op("session.shell")(operation)
                        .summary("Run shell command")
                        .description("Execute a shell command within the session context and return the AI's response.")
                        .with(|op| {
                            session_parameters(op, &["directory", "workspace", "sessionID"])
                        })
                        .with(|op| inline_json_request::<ShellRequest>(op, true))
                        .with(|op| {
                            json_response::<200, AssistantMessageDoc>(op, "Created message")
                        }),
                ),
            )
        }),
    )
}

fn session_revert_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/session/{sessionID}/revert",
        post_with(session_revert, |operation| {
            not_found(
                bad_request(
                    op("session.revert")(operation)
                        .summary("Revert message")
                        .description("Revert a specific message in a session, undoing its effects and restoring the previous state.")
                        .with(|op| {
                            session_parameters(op, &["directory", "workspace", "sessionID"])
                        })
                        .with(|op| inline_json_request::<RevertRequest>(op, true))
                        .with(|op| json_response::<200, SessionDoc>(op, "Updated session")),
                ),
            )
        }),
    )
}

fn session_unrevert_route() -> ApiRouter<AppState> {
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

fn session_permission_reply_route() -> ApiRouter<AppState> {
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

fn op(id: &'static str) -> impl Fn(TransformOperation) -> TransformOperation + Clone {
    move |op| op.id(id)
}

fn op_with_query(
    id: &'static str,
    summary: &'static str,
    description: &'static str,
) -> impl Fn(TransformOperation) -> TransformOperation + Clone {
    move |op| op.id(id).summary(summary).description(description)
}

fn bad_request(op: TransformOperation) -> TransformOperation {
    op.response_with::<400, Json<BadRequestErrorDoc>, _>(|res| res.description("Bad request"))
}

fn not_found(op: TransformOperation) -> TransformOperation {
    op.response_with::<404, Json<NotFoundErrorDoc>, _>(|res| res.description("Not found"))
}

fn json_response<'a, const N: u16, T: JsonSchema>(
    op: TransformOperation<'a>,
    description: &'static str,
) -> TransformOperation<'a> {
    op.response_with::<N, Json<T>, _>(|res: TransformResponse<'_, T>| res.description(description))
}

fn inline_json_request<'a, T: JsonSchema>(
    op: TransformOperation<'a>,
    required: bool,
) -> TransformOperation<'a> {
    let schema = inline_schema_value::<T>();
    inline_json_request_from_value(op, required, schema)
}

fn parameter_order<'a>(mut op: TransformOperation<'a>, names: &[&str]) -> TransformOperation<'a> {
    let order = names
        .iter()
        .enumerate()
        .map(|(idx, name)| (*name, idx))
        .collect::<BTreeMap<_, _>>();
    op.inner_mut()
        .parameters
        .sort_by_key(|parameter| match parameter {
            ReferenceOr::Item(parameter) => order
                .get(parameter.parameter_data_ref().name.as_str())
                .copied()
                .unwrap_or(usize::MAX),
            ReferenceOr::Reference { .. } => usize::MAX,
        });
    op
}

fn inline_schema_value<T: JsonSchema>() -> Value {
    let mut value =
        serde_json::to_value(schema_for!(T)).expect("serializing schemars schema should succeed");
    if let Some(object) = value.as_object_mut() {
        object.remove("$schema");
        object.remove("title");
    }
    inline_local_defs(&mut value);
    value
}

fn inline_local_defs(schema: &mut Value) {
    let defs = schema
        .as_object_mut()
        .and_then(|object| object.remove("$defs"))
        .and_then(|defs| defs.as_object().cloned());

    let Some(defs) = defs else {
        return;
    };

    inline_local_defs_refs(schema, &defs);
}

fn inline_local_defs_refs(value: &mut Value, defs: &serde_json::Map<String, Value>) {
    match value {
        Value::Object(map) => {
            if let Some(reference) = map.get("$ref").and_then(Value::as_str) {
                if let Some(name) = reference.strip_prefix("#/$defs/") {
                    if let Some(schema) = defs.get(name) {
                        if let Some(component_name) = schema.get("title").and_then(Value::as_str) {
                            *value = serde_json::json!({
                                "$ref": format!("#/components/schemas/{component_name}")
                            });
                            return;
                        }

                        *value = schema.clone();
                        inline_local_defs_refs(value, defs);
                        return;
                    }
                }
            }

            if let Some(local_defs) = map
                .remove("$defs")
                .and_then(|defs| defs.as_object().cloned())
            {
                let mut merged_defs = defs.clone();
                for (name, schema) in local_defs {
                    merged_defs.insert(name, schema);
                }
                for child in map.values_mut() {
                    inline_local_defs_refs(child, &merged_defs);
                }
                return;
            }

            for child in map.values_mut() {
                inline_local_defs_refs(child, defs);
            }
        }
        Value::Array(items) => {
            for item in items {
                inline_local_defs_refs(item, defs);
            }
        }
        _ => {}
    }
}

fn inline_json_request_from_value<'a>(
    mut op: TransformOperation<'a>,
    required: bool,
    schema: Value,
) -> TransformOperation<'a> {
    op.inner_mut().request_body = Some(ReferenceOr::Item(RequestBody {
        description: None,
        content: [(
            "application/json".to_owned(),
            MediaType {
                schema: Some(SchemaObject {
                    json_schema: serde_json::from_value(schema)
                        .expect("request-body schema should deserialize"),
                    external_docs: None,
                    example: None,
                }),
                ..Default::default()
            },
        )]
        .into_iter()
        .collect(),
        required,
        ..Default::default()
    }));
    if let Some(responses) = op.inner_mut().responses.as_mut() {
        let remove_default_400 = matches!(
            responses
                .responses
                .get(&StatusCode::Code(400))
                .and_then(ReferenceOr::as_item)
                .map(|response| response.description.as_str()),
            Some("Failed to parse the request body as JSON")
        );
        if remove_default_400 {
            responses.responses.shift_remove(&StatusCode::Code(400));
        }
    }
    op
}

fn session_parameters<'a>(
    op: aide::transform::TransformOperation<'a>,
    names: &[&str],
) -> aide::transform::TransformOperation<'a> {
    parameter_order(op, names)
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

async fn session_create(
    State(server): State<AppState>,
    Query(params): Query<CompatQuery>,
    Json(body): Json<SessionCreateRequest>,
) -> Response {
    let project = match resolve_current_project(&server, &params).await {
        Ok(project) => project,
        Err(response) => return response,
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

async fn session_get(
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

async fn session_update(
    State(server): State<AppState>,
    Path(SessionIdPath { session_id }): Path<SessionIdPath>,
    Query(_query): Query<CompatQuery>,
    Json(body): Json<SessionUpdateRequest>,
) -> Response {
    let session_id = match parse_compat_session_id(&session_id) {
        Ok(id) => id,
        Err(response) => return response,
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
    let project = match core.project(session.project_id).await {
        Ok(project) => Some(project),
        Err(_) => None,
    };
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
        Err(response) => return response,
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

async fn session_fork(
    State(server): State<AppState>,
    Path(SessionIdPath { session_id }): Path<SessionIdPath>,
    Query(_query): Query<CompatQuery>,
    Json(_body): Json<SessionForkRequest>,
) -> Response {
    let parent_id = match parse_compat_session_id(&session_id) {
        Ok(id) => id,
        Err(response) => return response,
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
        Err(response) => return response,
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
        Err(response) => return response,
    };
    {
        let compat = server.compat();
        let mut meta = compat.session_meta.write().await;
        meta.entry(compat_session_id(internal))
            .or_default()
            .share_url = Some(format!("https://example.invalid/s/{session_id}"));
    }
    session_get(
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
        Err(response) => return response,
    };
    {
        let compat = server.compat();
        let mut meta = compat.session_meta.write().await;
        meta.entry(compat_session_id(internal))
            .or_default()
            .share_url = None;
    }
    session_get(
        State(server),
        Path(SessionIdPath { session_id }),
        Query(CompatQuery::default()),
    )
    .await
}

async fn session_diff(
    Path(_path): Path<SessionIdPath>,
    Query(_query): Query<SessionDiffQuery>,
) -> Response {
    Json(Vec::<FileDiffDoc>::new()).into_response()
}

async fn session_summarize(
    Path(_path): Path<SessionIdPath>,
    Query(_query): Query<CompatQuery>,
    Json(_body): Json<SessionSummarizeRequest>,
) -> Response {
    Json(true).into_response()
}

async fn session_messages(
    State(server): State<AppState>,
    Path(SessionIdPath { session_id }): Path<SessionIdPath>,
    Query(params): Query<MessageListQuery>,
) -> Response {
    let session_id = match parse_compat_session_id(&session_id) {
        Ok(id) => id,
        Err(response) => return response,
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
        Err(response) => return response,
    };
    let message_id = match parse_compat_message_id(&message_id) {
        Ok(id) => id,
        Err(response) => return response,
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
        Err(response) => return response,
    };
    let message_id = match parse_compat_message_id(&message_id) {
        Ok(id) => id,
        Err(response) => return response,
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

async fn session_revert(
    State(server): State<AppState>,
    Path(SessionIdPath { session_id }): Path<SessionIdPath>,
    Query(_query): Query<CompatQuery>,
    Json(_body): Json<RevertRequest>,
) -> Response {
    session_get(
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
    session_get(
        State(server),
        Path(SessionIdPath { session_id }),
        Query(CompatQuery::default()),
    )
    .await
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

#[cfg(test)]
mod tests {
    use crate::compat::opencode::AppState;
    use crate::compat::opencode::test_utils::{
        normalize_generated_opencode_route_doc, normalize_opencode_route_doc,
        opencode_openapi_options, pinned_opencode_openapi,
    };
    use crate::utils::openapi::{generate_from_router, subset_for_operations};
    use aide::axum::ApiRouter;

    fn assert_route_matches(route: fn() -> ApiRouter<AppState>, path: &str, method: &str) {
        assert_eq!(
            normalize_generated_opencode_route_doc(&generate_from_router(
                route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[(path, method)],
            ))
        );
    }

    #[test]
    fn session_list_route_openapi_matches_pinned_subset() {
        assert_route_matches(super::session_list_route, "/session", "get");
    }

    #[test]
    fn session_create_route_openapi_matches_pinned_subset() {
        assert_route_matches(super::session_create_route, "/session", "post");
    }

    #[test]
    fn session_status_route_openapi_matches_pinned_subset() {
        assert_route_matches(super::session_status_route, "/session/status", "get");
    }

    #[test]
    fn session_get_route_openapi_matches_pinned_subset() {
        assert_route_matches(super::session_get_route, "/session/{sessionID}", "get");
    }

    #[test]
    fn session_update_route_openapi_matches_pinned_subset() {
        assert_route_matches(super::session_update_route, "/session/{sessionID}", "patch");
    }

    #[test]
    fn session_delete_route_openapi_matches_pinned_subset() {
        assert_route_matches(
            super::session_delete_route,
            "/session/{sessionID}",
            "delete",
        );
    }

    #[test]
    fn session_children_route_openapi_matches_pinned_subset() {
        assert_route_matches(
            super::session_children_route,
            "/session/{sessionID}/children",
            "get",
        );
    }

    #[test]
    fn session_todo_route_openapi_matches_pinned_subset() {
        assert_route_matches(
            super::session_todo_route,
            "/session/{sessionID}/todo",
            "get",
        );
    }

    #[test]
    fn session_init_route_openapi_matches_pinned_subset() {
        assert_route_matches(
            super::session_init_route,
            "/session/{sessionID}/init",
            "post",
        );
    }

    #[test]
    fn session_fork_route_openapi_matches_pinned_subset() {
        assert_route_matches(
            super::session_fork_route,
            "/session/{sessionID}/fork",
            "post",
        );
    }

    #[test]
    fn session_abort_route_openapi_matches_pinned_subset() {
        assert_route_matches(
            super::session_abort_route,
            "/session/{sessionID}/abort",
            "post",
        );
    }

    #[test]
    fn session_share_route_openapi_matches_pinned_subset() {
        assert_route_matches(
            super::session_share_route,
            "/session/{sessionID}/share",
            "post",
        );
    }

    #[test]
    fn session_unshare_route_openapi_matches_pinned_subset() {
        assert_route_matches(
            super::session_unshare_route,
            "/session/{sessionID}/share",
            "delete",
        );
    }

    #[test]
    fn session_diff_route_openapi_matches_pinned_subset() {
        assert_route_matches(
            super::session_diff_route,
            "/session/{sessionID}/diff",
            "get",
        );
    }

    #[test]
    fn session_summarize_route_openapi_matches_pinned_subset() {
        assert_route_matches(
            super::session_summarize_route,
            "/session/{sessionID}/summarize",
            "post",
        );
    }

    #[test]
    fn session_messages_route_openapi_matches_pinned_subset() {
        assert_route_matches(
            super::session_messages_route,
            "/session/{sessionID}/message",
            "get",
        );
    }

    #[test]
    fn session_prompt_route_openapi_matches_pinned_subset() {
        assert_route_matches(
            super::session_prompt_route,
            "/session/{sessionID}/message",
            "post",
        );
    }

    #[test]
    fn session_message_route_openapi_matches_pinned_subset() {
        assert_route_matches(
            super::session_message_route,
            "/session/{sessionID}/message/{messageID}",
            "get",
        );
    }

    #[test]
    fn session_message_delete_route_openapi_matches_pinned_subset() {
        assert_route_matches(
            super::session_message_delete_route,
            "/session/{sessionID}/message/{messageID}",
            "delete",
        );
    }

    #[test]
    fn session_part_update_route_openapi_matches_pinned_subset() {
        assert_route_matches(
            super::session_part_update_route,
            "/session/{sessionID}/message/{messageID}/part/{partID}",
            "patch",
        );
    }

    #[test]
    fn session_part_delete_route_openapi_matches_pinned_subset() {
        assert_route_matches(
            super::session_part_delete_route,
            "/session/{sessionID}/message/{messageID}/part/{partID}",
            "delete",
        );
    }

    #[test]
    fn session_prompt_async_route_openapi_matches_pinned_subset() {
        assert_route_matches(
            super::session_prompt_async_route,
            "/session/{sessionID}/prompt_async",
            "post",
        );
    }

    #[test]
    fn session_command_route_openapi_matches_pinned_subset() {
        assert_route_matches(
            super::session_command_route,
            "/session/{sessionID}/command",
            "post",
        );
    }

    #[test]
    fn session_shell_route_openapi_matches_pinned_subset() {
        assert_route_matches(
            super::session_shell_route,
            "/session/{sessionID}/shell",
            "post",
        );
    }

    #[test]
    fn session_revert_route_openapi_matches_pinned_subset() {
        assert_route_matches(
            super::session_revert_route,
            "/session/{sessionID}/revert",
            "post",
        );
    }

    #[test]
    fn session_unrevert_route_openapi_matches_pinned_subset() {
        assert_route_matches(
            super::session_unrevert_route,
            "/session/{sessionID}/unrevert",
            "post",
        );
    }

    #[test]
    fn session_permission_reply_route_openapi_matches_pinned_subset() {
        assert_route_matches(
            super::session_permission_reply_route,
            "/session/{sessionID}/permissions/{permissionID}",
            "post",
        );
    }
}
