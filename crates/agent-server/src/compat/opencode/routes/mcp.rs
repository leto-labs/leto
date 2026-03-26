use aide::axum::{
    ApiRouter,
    routing::{delete_with, get_with, post_with},
};

use super::super::types::common::{CompatQuery, NamedPath};
use super::super::types::errors::{BadRequestErrorDoc, NotFoundErrorDoc};
use super::super::types::mcp::{
    AgentDoc, AgentModeDoc, CommandDoc, FormatterStatusDoc, LspStatusDoc, SkillDoc,
};
use super::super::types::mcp::{
    McpAddRequest, McpAuthCallbackRequest, McpAuthStartResponseDoc, McpStatusDoc,
    SuccessResponseDoc,
};
use super::super::types::permission::PermissionRuleset;
use super::super::*;

pub(super) fn routes() -> ApiRouter<AppState> {
    ApiRouter::new()
        .merge(command_list_route())
        .merge(agent_list_route())
        .merge(skill_list_route())
        .merge(lsp_status_route())
        .merge(formatter_status_route())
        .merge(mcp_status_route())
        .merge(mcp_add_route())
        .merge(mcp_auth_start_route())
        .merge(mcp_auth_remove_route())
        .merge(mcp_auth_callback_route())
        .merge(mcp_auth_authenticate_route())
        .merge(mcp_connect_route())
        .merge(mcp_disconnect_route())
}

fn command_list_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/command",
        get_with(command_list, |operation| {
            operation
                .id("command.list")
                .summary("List commands")
                .description("Get a list of all available commands in the OpenCode system.")
                .response_with::<200, Json<Vec<CommandDoc>>, _>(|res| {
                    res.description("List of commands")
                })
        }),
    )
}

fn agent_list_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/agent",
        get_with(agent_list, |operation| {
            operation
                .id("app.agents")
                .summary("List agents")
                .description("Get a list of all available AI agents in the OpenCode system.")
                .response_with::<200, Json<Vec<AgentDoc>>, _>(|res| {
                    res.description("List of agents")
                })
        }),
    )
}

fn skill_list_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/skill",
        get_with(skill_list, |operation| {
            operation
                .id("app.skills")
                .summary("List skills")
                .description("Get a list of all available skills in the OpenCode system.")
                .response_with::<200, Json<Vec<SkillDoc>>, _>(|res| {
                    res.description("List of skills")
                })
        }),
    )
}

fn lsp_status_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/lsp",
        get_with(lsp_status, |operation| {
            operation
                .id("lsp.status")
                .summary("Get LSP status")
                .description("Get LSP server status")
                .response_with::<200, Json<Vec<LspStatusDoc>>, _>(|res| {
                    res.description("LSP server status")
                })
        }),
    )
}

fn formatter_status_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/formatter",
        get_with(formatter_status, |operation| {
            operation
                .id("formatter.status")
                .summary("Get formatter status")
                .description("Get formatter status")
                .response_with::<200, Json<Vec<FormatterStatusDoc>>, _>(|res| {
                    res.description("Formatter status")
                })
        }),
    )
}

fn mcp_status_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/mcp",
        get_with(mcp_status, |operation| {
            operation
                .id("mcp.status")
                .summary("Get MCP status")
                .description("Get the status of all Model Context Protocol (MCP) servers.")
                .response_with::<200, Json<BTreeMap<String, McpStatusDoc>>, _>(|res| {
                    res.description("MCP server status")
                })
        }),
    )
}

fn mcp_add_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/mcp",
        post_with(mcp_add, |operation| {
            operation
                .id("mcp.add")
                .summary("Add MCP server")
                .description(
                    "Dynamically add a new Model Context Protocol (MCP) server to the system.",
                )
                .response_with::<200, Json<BTreeMap<String, McpStatusDoc>>, _>(|res| {
                    res.description("MCP server added successfully")
                })
                .response_with::<400, Json<BadRequestErrorDoc>, _>(|res| {
                    res.description("Bad request")
                })
        }),
    )
}

fn mcp_auth_start_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/mcp/{name}/auth",
        post_with(mcp_auth_start, |operation| {
            operation
                .id("mcp.auth.start")
                .summary("Start MCP OAuth")
                .description(
                    "Start OAuth authentication flow for a Model Context Protocol (MCP) server.",
                )
                .response_with::<200, Json<McpAuthStartResponseDoc>, _>(|res| {
                    res.description("OAuth flow started")
                })
                .response_with::<400, Json<BadRequestErrorDoc>, _>(|res| {
                    res.description("Bad request")
                })
                .response_with::<404, Json<NotFoundErrorDoc>, _>(|res| res.description("Not found"))
        }),
    )
}

fn mcp_auth_remove_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/mcp/{name}/auth",
        delete_with(mcp_auth_remove, |operation| {
            operation
                .id("mcp.auth.remove")
                .summary("Remove MCP OAuth")
                .description("Remove OAuth credentials for an MCP server")
                .response_with::<200, Json<SuccessResponseDoc>, _>(|res| {
                    res.description("OAuth credentials removed")
                })
                .response_with::<404, Json<NotFoundErrorDoc>, _>(|res| res.description("Not found"))
        }),
    )
}

fn mcp_auth_callback_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/mcp/{name}/auth/callback",
        post_with(mcp_auth_callback, |operation| {
            operation
                .id("mcp.auth.callback")
                .summary("Complete MCP OAuth")
                .description(
                    "Complete OAuth authentication for a Model Context Protocol (MCP) server using the authorization code.",
                )
                .response_with::<200, Json<McpStatusDoc>, _>(|res| {
                    res.description("OAuth authentication completed")
                })
                .response_with::<400, Json<BadRequestErrorDoc>, _>(|res| {
                    res.description("Bad request")
                })
                .response_with::<404, Json<NotFoundErrorDoc>, _>(|res| {
                    res.description("Not found")
                })
        }),
    )
}

fn mcp_auth_authenticate_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/mcp/{name}/auth/authenticate",
        post_with(mcp_auth_authenticate, |operation| {
            operation
                .id("mcp.auth.authenticate")
                .summary("Authenticate MCP OAuth")
                .description("Start OAuth flow and wait for callback (opens browser)")
                .response_with::<200, Json<McpStatusDoc>, _>(|res| {
                    res.description("OAuth authentication completed")
                })
                .response_with::<400, Json<BadRequestErrorDoc>, _>(|res| {
                    res.description("Bad request")
                })
                .response_with::<404, Json<NotFoundErrorDoc>, _>(|res| res.description("Not found"))
        }),
    )
}

fn mcp_connect_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/mcp/{name}/connect",
        post_with(mcp_connect, |operation| {
            operation
                .id("mcp.connect")
                .description("Connect an MCP server")
                .response_with::<200, Json<bool>, _>(|res| {
                    res.description("MCP server connected successfully")
                })
        }),
    )
}

fn mcp_disconnect_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/mcp/{name}/disconnect",
        post_with(mcp_disconnect, |operation| {
            operation
                .id("mcp.disconnect")
                .description("Disconnect an MCP server")
                .response_with::<200, Json<bool>, _>(|res| {
                    res.description("MCP server disconnected successfully")
                })
        }),
    )
}

async fn command_list(
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
) -> Response {
    let commands = server
        .compat()
        .config
        .read()
        .await
        .get("command")
        .and_then(Value::as_object)
        .map(|items| {
            items
                .iter()
                .map(|(name, value)| CommandDoc {
                    name: name.clone(),
                    template: value
                        .get("template")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned(),
                    description: value
                        .get("description")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned),
                    agent: None,
                    model: None,
                    source: None,
                    subtask: None,
                    hints: Vec::new(),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Json(commands).into_response()
}

async fn agent_list(Query(_query): Query<CompatQuery>) -> Response {
    Json(
        vec![
            "plan",
            "build",
            "general",
            "explore",
            "title",
            "summary",
            "compaction",
        ]
        .into_iter()
        .map(|name| AgentDoc {
            name: name.to_owned(),
            description: Some(name.to_owned()),
            mode: AgentModeDoc::All,
            native: None,
            hidden: None,
            top_p: None,
            temperature: None,
            color: None,
            permission: PermissionRuleset(Vec::new()),
            model: None,
            variant: None,
            prompt: None,
            options: BTreeMap::new(),
            steps: None,
        })
        .collect::<Vec<_>>(),
    )
    .into_response()
}

async fn skill_list(Query(_query): Query<CompatQuery>) -> Response {
    Json(Vec::<SkillDoc>::new()).into_response()
}

async fn lsp_status(Query(_query): Query<CompatQuery>) -> Response {
    Json(Vec::<LspStatusDoc>::new()).into_response()
}

async fn formatter_status(Query(_query): Query<CompatQuery>) -> Response {
    Json(Vec::<FormatterStatusDoc>::new()).into_response()
}

async fn mcp_status(State(server): State<AppState>, Query(_query): Query<CompatQuery>) -> Response {
    Json(
        server
            .compat()
            .mcp_servers
            .read()
            .await
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect::<BTreeMap<String, _>>(),
    )
    .into_response()
}

async fn mcp_add(
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
    Json(body): Json<McpAddRequest>,
) -> Response {
    let name = body.name;
    let status = mcp_connected_status();
    server
        .compat()
        .mcp_servers
        .write()
        .await
        .insert(name, status);
    mcp_status(State(server), Query(CompatQuery::default())).await
}

async fn mcp_auth_start(
    Query(_query): Query<CompatQuery>,
    Path(NamedPath { name }): Path<NamedPath>,
) -> Response {
    Json(McpAuthStartResponseDoc {
        authorization_url: format!("https://example.invalid/mcp/{name}/oauth"),
    })
    .into_response()
}

async fn mcp_auth_callback(
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
    Path(NamedPath { name }): Path<NamedPath>,
    Json(_body): Json<McpAuthCallbackRequest>,
) -> Response {
    let status = mcp_connected_status();
    server
        .compat()
        .mcp_servers
        .write()
        .await
        .insert(name, status.clone());
    Json(status).into_response()
}

async fn mcp_auth_authenticate(
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
    Path(NamedPath { name }): Path<NamedPath>,
) -> Response {
    let body = McpAuthCallbackRequest {
        code: String::new(),
    };
    mcp_auth_callback(
        State(server),
        Query(CompatQuery::default()),
        Path(NamedPath { name }),
        Json(body),
    )
    .await
}

async fn mcp_auth_remove(
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
    Path(NamedPath { name }): Path<NamedPath>,
) -> Response {
    server.compat().mcp_servers.write().await.remove(&name);
    Json(SuccessResponseDoc {
        success: super::super::types::provider::TrueConstDoc,
    })
    .into_response()
}

async fn mcp_connect(
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
    Path(NamedPath { name }): Path<NamedPath>,
) -> Response {
    server
        .compat()
        .mcp_servers
        .write()
        .await
        .insert(name, mcp_connected_status());
    Json(true).into_response()
}

async fn mcp_disconnect(
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
    Path(NamedPath { name }): Path<NamedPath>,
) -> Response {
    server
        .compat()
        .mcp_servers
        .write()
        .await
        .insert(name, mcp_disabled_status());
    Json(true).into_response()
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use crate::compat::opencode::test_utils::{
        normalize_generated_opencode_route_doc, normalize_opencode_route_doc,
        opencode_openapi_options, pinned_opencode_openapi,
    };
    use crate::utils::openapi::{generate_from_router, subset_for_operations};

    fn normalize_command_list_template_pinned_quirk(doc: &Value) -> Value {
        let mut doc = doc.clone();

        let duplicated_template_union = json!({
            "anyOf": [
                { "type": "string" },
                { "type": "string" }
            ]
        });

        // Upstream OpenCode models this as `z.promise(z.string()).or(z.string())`.
        // The pinned spec preserves that as `anyOf[string, string]`, while our
        // raw aide/schemars output collapses it to a plain `string`.
        if let Some(template) = doc
            .get_mut("components")
            .and_then(Value::as_object_mut)
            .and_then(|components| components.get_mut("schemas"))
            .and_then(Value::as_object_mut)
            .and_then(|schemas| schemas.get_mut("Command"))
            .and_then(Value::as_object_mut)
            .and_then(|command| command.get_mut("properties"))
            .and_then(Value::as_object_mut)
            .and_then(|properties| properties.get_mut("template"))
        {
            if *template == json!({ "type": "string" }) {
                *template = duplicated_template_union.clone();
            }
        }

        let Some(template) = doc
            .get_mut("paths")
            .and_then(Value::as_object_mut)
            .and_then(|paths| paths.get_mut("/command"))
            .and_then(Value::as_object_mut)
            .and_then(|path_item| path_item.get_mut("get"))
            .and_then(Value::as_object_mut)
            .and_then(|operation| operation.get_mut("responses"))
            .and_then(Value::as_object_mut)
            .and_then(|responses| responses.get_mut("200"))
            .and_then(Value::as_object_mut)
            .and_then(|response| response.get_mut("content"))
            .and_then(Value::as_object_mut)
            .and_then(|content| content.get_mut("application/json"))
            .and_then(Value::as_object_mut)
            .and_then(|media_type| media_type.get_mut("schema"))
            .and_then(Value::as_object_mut)
            .and_then(|schema| schema.get_mut("items"))
            .and_then(Value::as_object_mut)
            .and_then(|items| items.get_mut("properties"))
            .and_then(Value::as_object_mut)
            .and_then(|properties| properties.get_mut("template"))
        else {
            return doc;
        };

        if *template == json!({ "type": "string" }) {
            *template = duplicated_template_union;
        }

        doc
    }

    #[test]
    fn command_list_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_generated_opencode_route_doc(&normalize_command_list_template_pinned_quirk(
                &generate_from_router(super::command_list_route, opencode_openapi_options()),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/command", "get")],
            ))
        );
    }

    #[test]
    fn agent_list_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_generated_opencode_route_doc(&generate_from_router(
                super::agent_list_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/agent", "get")],
            ))
        );
    }

    #[test]
    fn skill_list_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_generated_opencode_route_doc(&generate_from_router(
                super::skill_list_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/skill", "get")],
            ))
        );
    }

    #[test]
    fn lsp_status_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_generated_opencode_route_doc(&generate_from_router(
                super::lsp_status_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/lsp", "get")],
            ))
        );
    }

    #[test]
    fn formatter_status_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_generated_opencode_route_doc(&generate_from_router(
                super::formatter_status_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/formatter", "get")],
            ))
        );
    }

    #[test]
    fn mcp_status_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_generated_opencode_route_doc(&generate_from_router(
                super::mcp_status_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/mcp", "get")],
            ))
        );
    }

    #[test]
    fn mcp_add_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_generated_opencode_route_doc(&generate_from_router(
                super::mcp_add_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/mcp", "post")],
            ))
        );
    }

    #[test]
    fn mcp_auth_start_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_generated_opencode_route_doc(&generate_from_router(
                super::mcp_auth_start_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/mcp/{name}/auth", "post")],
            ))
        );
    }

    #[test]
    fn mcp_auth_remove_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_generated_opencode_route_doc(&generate_from_router(
                super::mcp_auth_remove_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/mcp/{name}/auth", "delete")],
            ))
        );
    }

    #[test]
    fn mcp_auth_callback_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_generated_opencode_route_doc(&generate_from_router(
                super::mcp_auth_callback_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/mcp/{name}/auth/callback", "post")],
            ))
        );
    }

    #[test]
    fn mcp_auth_authenticate_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_generated_opencode_route_doc(&generate_from_router(
                super::mcp_auth_authenticate_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/mcp/{name}/auth/authenticate", "post")],
            ))
        );
    }

    #[test]
    fn mcp_connect_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_generated_opencode_route_doc(&generate_from_router(
                super::mcp_connect_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/mcp/{name}/connect", "post")],
            ))
        );
    }

    #[test]
    fn mcp_disconnect_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_generated_opencode_route_doc(&generate_from_router(
                super::mcp_disconnect_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/mcp/{name}/disconnect", "post")],
            ))
        );
    }
}
