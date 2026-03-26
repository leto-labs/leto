use aide::axum::{
    ApiRouter,
    routing::{get_with, post_with},
};

use super::super::types::common::{CompatQuery, PathBody};
use super::super::types::errors::{BadRequestErrorDoc, NotFoundErrorDoc};
use super::super::*;

pub(super) fn routes() -> ApiRouter<AppState> {
    ApiRouter::new()
        .merge(tui_append_prompt_route())
        .merge(tui_clear_prompt_route())
        .merge(tui_open_help_route())
        .merge(tui_open_sessions_route())
        .merge(tui_open_themes_route())
        .merge(tui_open_models_route())
        .merge(tui_submit_prompt_route())
        .merge(tui_show_toast_route())
        .merge(tui_publish_route())
        .merge(tui_select_session_route())
        .merge(tui_execute_command_route())
        .merge(tui_control_next_route())
        .merge(tui_control_response_route())
}

fn tui_append_prompt_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/tui/append-prompt",
        post_with(tui_append_prompt, |operation| {
            operation
                .id("tui.appendPrompt")
                .summary("Append TUI prompt")
                .description("Append prompt to the TUI")
                .response_with::<200, Json<bool>, _>(|res| {
                    res.description("Prompt processed successfully")
                })
                .response_with::<400, Json<BadRequestErrorDoc>, _>(|res| {
                    res.description("Bad request")
                })
        }),
    )
}

fn tui_clear_prompt_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/tui/clear-prompt",
        post_with(tui_clear_prompt, |operation| {
            operation
                .id("tui.clearPrompt")
                .summary("Clear TUI prompt")
                .description("Clear the prompt")
                .response_with::<200, Json<bool>, _>(|res| {
                    res.description("Prompt cleared successfully")
                })
        }),
    )
}

fn tui_open_help_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/tui/open-help",
        post_with(tui_open_help, |operation| {
            operation
                .id("tui.openHelp")
                .summary("Open help dialog")
                .description(
                    "Open the help dialog in the TUI to display user assistance information.",
                )
                .response_with::<200, Json<bool>, _>(|res| {
                    res.description("Help dialog opened successfully")
                })
        }),
    )
}

fn tui_open_sessions_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/tui/open-sessions",
        post_with(tui_open_sessions, |operation| {
            operation
                .id("tui.openSessions")
                .summary("Open sessions dialog")
                .description("Open the session dialog")
                .response_with::<200, Json<bool>, _>(|res| {
                    res.description("Session dialog opened successfully")
                })
        }),
    )
}

fn tui_open_themes_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/tui/open-themes",
        post_with(tui_open_themes, |operation| {
            operation
                .id("tui.openThemes")
                .summary("Open themes dialog")
                .description("Open the theme dialog")
                .response_with::<200, Json<bool>, _>(|res| {
                    res.description("Theme dialog opened successfully")
                })
        }),
    )
}

fn tui_open_models_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/tui/open-models",
        post_with(tui_open_models, |operation| {
            operation
                .id("tui.openModels")
                .summary("Open models dialog")
                .description("Open the model dialog")
                .response_with::<200, Json<bool>, _>(|res| {
                    res.description("Model dialog opened successfully")
                })
        }),
    )
}

fn tui_submit_prompt_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/tui/submit-prompt",
        post_with(tui_submit_prompt, |operation| {
            operation
                .id("tui.submitPrompt")
                .summary("Submit TUI prompt")
                .description("Submit the prompt")
                .response_with::<200, Json<bool>, _>(|res| {
                    res.description("Prompt submitted successfully")
                })
        }),
    )
}

fn tui_show_toast_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/tui/show-toast",
        post_with(tui_show_toast, |operation| {
            operation
                .id("tui.showToast")
                .summary("Show TUI toast")
                .description("Show a toast notification in the TUI")
                .response_with::<200, Json<bool>, _>(|res| {
                    res.description("Toast notification shown successfully")
                })
        }),
    )
}

fn tui_publish_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/tui/publish",
        post_with(tui_publish, |operation| {
            operation
                .id("tui.publish")
                .summary("Publish TUI event")
                .description("Publish a TUI event")
                .response_with::<200, Json<bool>, _>(|res| {
                    res.description("Event published successfully")
                })
                .response_with::<400, Json<BadRequestErrorDoc>, _>(|res| {
                    res.description("Bad request")
                })
        }),
    )
}

fn tui_select_session_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/tui/select-session",
        post_with(tui_select_session, |operation| {
            operation
                .id("tui.selectSession")
                .summary("Select session")
                .description("Navigate the TUI to display the specified session.")
                .response_with::<200, Json<bool>, _>(|res| {
                    res.description("Session selected successfully")
                })
                .response_with::<400, Json<BadRequestErrorDoc>, _>(|res| {
                    res.description("Bad request")
                })
                .response_with::<404, Json<NotFoundErrorDoc>, _>(|res| res.description("Not found"))
        }),
    )
}

fn tui_execute_command_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/tui/execute-command",
        post_with(tui_execute_command, |operation| {
            operation
                .id("tui.executeCommand")
                .summary("Execute TUI command")
                .description("Execute a TUI command (e.g. agent_cycle)")
                .response_with::<200, Json<bool>, _>(|res| {
                    res.description("Command executed successfully")
                })
                .response_with::<400, Json<BadRequestErrorDoc>, _>(|res| {
                    res.description("Bad request")
                })
        }),
    )
}

fn tui_control_next_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/tui/control/next",
        get_with(tui_control_next, |operation| {
            operation
                .id("tui.control.next")
                .summary("Get next TUI request")
                .description(
                    "Retrieve the next TUI (Terminal User Interface) request from the queue for processing.",
                )
                .response_with::<200, Json<PathBody>, _>(|res| {
                    res.description("Next TUI request")
                })
        }),
    )
}

fn tui_control_response_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/tui/control/response",
        post_with(tui_control_response, |operation| {
            operation
                .id("tui.control.response")
                .summary("Submit TUI response")
                .description(
                    "Submit a response to the TUI request queue to complete a pending request.",
                )
                .response_with::<200, Json<bool>, _>(|res| {
                    res.description("Response submitted successfully")
                })
        }),
    )
}

async fn tui_append_prompt(
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
    Json(body): Json<PromptAppendRequest>,
) -> Response {
    let body = serde_json::to_value(body).unwrap_or(Value::Null);
    tui_enqueue(&server, "/tui/append-prompt", body).await;
    Json(true).into_response()
}

async fn tui_clear_prompt(
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
) -> Response {
    tui_enqueue(&server, "/tui/clear-prompt", Value::Null).await;
    Json(true).into_response()
}

async fn tui_open_help(
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
) -> Response {
    tui_enqueue(&server, "/tui/open-help", Value::Null).await;
    Json(true).into_response()
}

async fn tui_open_sessions(
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
) -> Response {
    tui_enqueue(&server, "/tui/open-sessions", Value::Null).await;
    Json(true).into_response()
}

async fn tui_open_themes(
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
) -> Response {
    tui_enqueue(&server, "/tui/open-themes", Value::Null).await;
    Json(true).into_response()
}

async fn tui_open_models(
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
) -> Response {
    tui_enqueue(&server, "/tui/open-models", Value::Null).await;
    Json(true).into_response()
}

async fn tui_submit_prompt(
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
) -> Response {
    tui_enqueue(&server, "/tui/submit-prompt", Value::Null).await;
    Json(true).into_response()
}

async fn tui_show_toast(
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
    Json(body): Json<ToastRequest>,
) -> Response {
    let body = serde_json::to_value(body).unwrap_or(Value::Null);
    tui_enqueue(&server, "/tui/show-toast", body).await;
    Json(true).into_response()
}

async fn tui_publish(
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
    Json(body): Json<PublishRequest>,
) -> Response {
    let body = serde_json::to_value(body).unwrap_or(Value::Null);
    tui_enqueue(&server, "/tui/publish", body).await;
    Json(true).into_response()
}

async fn tui_select_session(
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
    Json(body): Json<SelectSessionRequest>,
) -> Response {
    let body = serde_json::to_value(body).unwrap_or(Value::Null);
    tui_enqueue(&server, "/tui/select-session", body).await;
    Json(true).into_response()
}

async fn tui_execute_command(
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
    Json(body): Json<ExecuteCommandRequest>,
) -> Response {
    let body = serde_json::to_value(body).unwrap_or(Value::Null);
    tui_enqueue(&server, "/tui/execute-command", body).await;
    Json(true).into_response()
}

async fn tui_control_next(
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
) -> Response {
    let next = server
        .compat()
        .tui_requests
        .write()
        .await
        .pop_front()
        .unwrap_or_else(|| json!({"path": "/tui/noop", "body": null}));
    Json(next).into_response()
}

async fn tui_control_response(
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
    Json(body): Json<Value>,
) -> Response {
    server.compat().tui_responses.write().await.push_back(body);
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
    fn tui_append_prompt_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_opencode_route_doc(&generate_from_router(
                super::tui_append_prompt_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/tui/append-prompt", "post")],
            ))
        );
    }

    #[test]
    fn tui_clear_prompt_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_opencode_route_doc(&generate_from_router(
                super::tui_clear_prompt_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/tui/clear-prompt", "post")],
            ))
        );
    }

    #[test]
    fn tui_open_help_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_opencode_route_doc(&generate_from_router(
                super::tui_open_help_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/tui/open-help", "post")],
            ))
        );
    }

    #[test]
    fn tui_open_sessions_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_opencode_route_doc(&generate_from_router(
                super::tui_open_sessions_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/tui/open-sessions", "post")],
            ))
        );
    }

    #[test]
    fn tui_open_themes_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_opencode_route_doc(&generate_from_router(
                super::tui_open_themes_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/tui/open-themes", "post")],
            ))
        );
    }

    #[test]
    fn tui_open_models_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_opencode_route_doc(&generate_from_router(
                super::tui_open_models_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/tui/open-models", "post")],
            ))
        );
    }

    #[test]
    fn tui_submit_prompt_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_opencode_route_doc(&generate_from_router(
                super::tui_submit_prompt_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/tui/submit-prompt", "post")],
            ))
        );
    }

    #[test]
    fn tui_show_toast_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_opencode_route_doc(&generate_from_router(
                super::tui_show_toast_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/tui/show-toast", "post")],
            ))
        );
    }

    #[test]
    fn tui_publish_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_opencode_route_doc(&generate_from_router(
                super::tui_publish_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/tui/publish", "post")],
            ))
        );
    }

    #[test]
    fn tui_select_session_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_opencode_route_doc(&generate_from_router(
                super::tui_select_session_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/tui/select-session", "post")],
            ))
        );
    }

    #[test]
    fn tui_execute_command_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_opencode_route_doc(&generate_from_router(
                super::tui_execute_command_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/tui/execute-command", "post")],
            ))
        );
    }

    #[test]
    fn tui_control_next_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_opencode_route_doc(&generate_from_router(
                super::tui_control_next_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/tui/control/next", "get")],
            ))
        );
    }

    #[test]
    fn tui_control_response_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_opencode_route_doc(&generate_from_router(
                super::tui_control_response_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/tui/control/response", "post")],
            ))
        );
    }
}
