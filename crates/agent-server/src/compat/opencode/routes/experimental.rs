use aide::axum::{
    ApiRouter,
    routing::{delete_with, get_with, post_with},
};

use super::super::types::common::{CompatQuery, ExperimentalSessionListQueryDoc};
use super::super::types::errors::BadRequestErrorDoc;
use super::super::types::experimental::{
    NullableJsonValueDoc, NullableStringDoc, WorkspaceCreateRequest, WorkspaceDoc, WorkspaceIdPath,
    WorktreeCreateRequest, WorktreeDoc, WorktreeRemoveRequest, WorktreeResetRequest,
};
use super::super::types::mcp::{
    McpResourceDoc, ToolIdsDoc, ToolListDoc, ToolListItemDoc, ToolListQueryDoc,
};
use super::super::types::session::GlobalSessionDoc;
use super::super::*;

pub(super) fn routes() -> ApiRouter<AppState> {
    ApiRouter::new()
        .merge(experimental_tool_ids_route())
        .merge(experimental_tool_list_route())
        .merge(experimental_workspace_list_route())
        .merge(experimental_workspace_create_route())
        .merge(experimental_workspace_remove_route())
        .merge(experimental_worktree_list_route())
        .merge(experimental_worktree_create_route())
        .merge(experimental_worktree_remove_route())
        .merge(experimental_worktree_reset_route())
        .merge(experimental_session_list_route())
        .merge(experimental_resource_list_route())
}

fn experimental_tool_ids_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/experimental/tool/ids",
        get_with(experimental_tool_ids, |operation| {
            operation
                .id("tool.ids")
                .summary("List tool IDs")
                .description(
                    "Get a list of all available tool IDs, including both built-in tools and dynamically registered tools.",
                )
                .response_with::<200, Json<ToolIdsDoc>, _>(|res| res.description("Tool IDs"))
                .response_with::<400, Json<BadRequestErrorDoc>, _>(|res| {
                    res.description("Bad request")
                })
        }),
    )
}

fn experimental_tool_list_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/experimental/tool",
        get_with(experimental_tool_list, |operation| {
            operation
                .id("tool.list")
                .summary("List tools")
                .description(
                    "Get a list of available tools with their JSON schema parameters for a specific provider and model combination.",
                )
                .response_with::<200, Json<ToolListDoc>, _>(|res| res.description("Tools"))
                .response_with::<400, Json<BadRequestErrorDoc>, _>(|res| {
                    res.description("Bad request")
                })
        }),
    )
}

fn experimental_workspace_list_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/experimental/workspace",
        get_with(experimental_workspace_list, |operation| {
            operation
                .id("experimental.workspace.list")
                .summary("List workspaces")
                .description("List all workspaces.")
                .response_with::<200, Json<Vec<WorkspaceDoc>>, _>(|res| {
                    res.description("Workspaces")
                })
        }),
    )
}

fn experimental_workspace_create_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/experimental/workspace",
        post_with(experimental_workspace_create, |operation| {
            operation
                .id("experimental.workspace.create")
                .summary("Create workspace")
                .description("Create a workspace for the current project.")
                .response_with::<200, Json<WorkspaceDoc>, _>(|res| {
                    res.description("Workspace created")
                })
                .response_with::<400, Json<BadRequestErrorDoc>, _>(|res| {
                    res.description("Bad request")
                })
        }),
    )
}

fn experimental_workspace_remove_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/experimental/workspace/{id}",
        delete_with(experimental_workspace_remove, |operation| {
            operation
                .id("experimental.workspace.remove")
                .summary("Remove workspace")
                .description("Remove an existing workspace.")
                .response_with::<200, Json<WorkspaceDoc>, _>(|res| {
                    res.description("Workspace removed")
                })
                .response_with::<400, Json<BadRequestErrorDoc>, _>(|res| {
                    res.description("Bad request")
                })
        }),
    )
}

fn experimental_worktree_list_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/experimental/worktree",
        get_with(experimental_worktree_list, |operation| {
            operation
                .id("worktree.list")
                .summary("List worktrees")
                .description("List all sandbox worktrees for the current project.")
                .response_with::<200, Json<Vec<String>>, _>(|res| {
                    res.description("List of worktree directories")
                })
        }),
    )
}

fn experimental_worktree_create_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/experimental/worktree",
        post_with(experimental_worktree_create, |operation| {
            operation
                .id("worktree.create")
                .summary("Create worktree")
                .description(
                    "Create a new git worktree for the current project and run any configured startup scripts.",
                )
                .response_with::<200, Json<WorktreeDoc>, _>(|res| {
                    res.description("Worktree created")
                })
                .response_with::<400, Json<BadRequestErrorDoc>, _>(|res| {
                    res.description("Bad request")
                })
        }),
    )
}

fn experimental_worktree_remove_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/experimental/worktree",
        delete_with(experimental_worktree_remove, |operation| {
            operation
                .id("worktree.remove")
                .summary("Remove worktree")
                .description("Remove a git worktree and delete its branch.")
                .response_with::<200, Json<bool>, _>(|res| res.description("Worktree removed"))
                .response_with::<400, Json<BadRequestErrorDoc>, _>(|res| {
                    res.description("Bad request")
                })
        }),
    )
}

fn experimental_worktree_reset_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/experimental/worktree/reset",
        post_with(experimental_worktree_reset, |operation| {
            operation
                .id("worktree.reset")
                .summary("Reset worktree")
                .description("Reset a worktree branch to the primary default branch.")
                .response_with::<200, Json<bool>, _>(|res| res.description("Worktree reset"))
                .response_with::<400, Json<BadRequestErrorDoc>, _>(|res| {
                    res.description("Bad request")
                })
        }),
    )
}

fn experimental_session_list_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/experimental/session",
        get_with(experimental_session_list, |operation| {
            operation
                .id("experimental.session.list")
                .summary("List sessions")
                .description(
                    "Get a list of all OpenCode sessions across projects, sorted by most recently updated. Archived sessions are excluded by default.",
                )
                .response_with::<200, Json<Vec<GlobalSessionDoc>>, _>(|res| {
                    res.description("List of sessions")
                })
        }),
    )
}

fn experimental_resource_list_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/experimental/resource",
        get_with(experimental_resource_list, |operation| {
            operation
                .id("experimental.resource.list")
                .summary("Get MCP resources")
                .description(
                    "Get all available MCP resources from connected servers. Optionally filter by name.",
                )
                .response_with::<200, Json<BTreeMap<String, McpResourceDoc>>, _>(|res| {
                    res.description("MCP resources")
                })
        }),
    )
}

async fn experimental_tool_ids(Query(_query): Query<CompatQuery>) -> Response {
    Json(ToolIdsDoc(Vec::new())).into_response()
}

async fn experimental_tool_list(Query(_query): Query<ToolListQueryDoc>) -> Response {
    Json(ToolListDoc(Vec::<ToolListItemDoc>::new())).into_response()
}

async fn experimental_workspace_list(
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
) -> Response {
    Json(
        server
            .compat()
            .workspaces
            .read()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>(),
    )
    .into_response()
}

async fn experimental_workspace_create(
    State(server): State<AppState>,
    Query(params): Query<CompatQuery>,
    Json(body): Json<WorkspaceCreateRequest>,
) -> Response {
    let project = match resolve_current_project(&server, &params).await {
        Ok(project) => project,
        Err(response) => return response,
    };
    let id = body.id.unwrap_or_else(|| format!("wrk{}", Ulid::new()));
    let workspace = WorkspaceDoc {
        id: id.clone(),
        workspace_type: body.workspace_type,
        branch: match body.branch {
            NullableStringDoc::String(value) => NullableStringDoc::String(value),
            NullableStringDoc::Null(()) => NullableStringDoc::Null(()),
        },
        name: NullableStringDoc::Null(()),
        directory: NullableStringDoc::String(default_directory_string()),
        extra: match body.extra {
            NullableJsonValueDoc::Value(value) => NullableJsonValueDoc::Value(value),
            NullableJsonValueDoc::Null(()) => NullableJsonValueDoc::Null(()),
        },
        project_id: project.id.to_string(),
    };
    server
        .compat()
        .workspaces
        .write()
        .await
        .insert(id, workspace.clone());
    Json(workspace).into_response()
}

async fn experimental_workspace_remove(
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
    Path(WorkspaceIdPath { id }): Path<WorkspaceIdPath>,
) -> Response {
    let removed = server
        .compat()
        .workspaces
        .write()
        .await
        .remove(&id)
        .unwrap_or(WorkspaceDoc {
            id,
            workspace_type: "local".to_owned(),
            branch: NullableStringDoc::Null(()),
            name: NullableStringDoc::Null(()),
            directory: NullableStringDoc::Null(()),
            extra: NullableJsonValueDoc::Null(()),
            project_id: String::new(),
        });
    Json(removed).into_response()
}

async fn experimental_worktree_create(
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
    Json(body): Json<WorktreeCreateRequest>,
) -> Response {
    let name = body
        .name
        .unwrap_or_else(|| format!("sandbox-{}", Ulid::new()));
    let branch = name.clone();
    let directory = format!("{}/{}", default_directory_string(), name);
    let worktree = WorktreeDoc {
        name,
        branch,
        directory,
    };
    server
        .compat()
        .worktrees
        .write()
        .await
        .insert(worktree.directory.clone(), worktree.clone());
    Json(worktree).into_response()
}

async fn experimental_worktree_list(
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
) -> Response {
    Json(
        server
            .compat()
            .worktrees
            .read()
            .await
            .keys()
            .cloned()
            .collect::<Vec<_>>(),
    )
    .into_response()
}

async fn experimental_worktree_remove(
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
    Json(body): Json<WorktreeRemoveRequest>,
) -> Response {
    server
        .compat()
        .worktrees
        .write()
        .await
        .remove(&body.directory);
    Json(true).into_response()
}

async fn experimental_worktree_reset(
    Query(_query): Query<CompatQuery>,
    Json(_body): Json<WorktreeResetRequest>,
) -> Response {
    Json(true).into_response()
}

async fn experimental_session_list(
    State(server): State<AppState>,
    Query(params): Query<ExperimentalSessionListQueryDoc>,
) -> Response {
    match filtered_sessions(&server, &params).await {
        Ok(sessions) => {
            let payload = sessions
                .iter()
                .map(|(session, project, meta)| {
                    compat_global_session(session, project.as_ref(), meta)
                })
                .collect::<Vec<_>>();
            Json(payload).into_response()
        }
        Err(response) => response,
    }
}

async fn experimental_resource_list(
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
) -> Response {
    let payload = server
        .compat()
        .mcp_servers
        .read()
        .await
        .keys()
        .map(|name| {
            (
                name.clone(),
                McpResourceDoc {
                    name: name.clone(),
                    uri: format!("mcp://{name}"),
                    description: None,
                    mime_type: None,
                    client: name.clone(),
                },
            )
        })
        .collect::<BTreeMap<String, McpResourceDoc>>();
    Json(payload).into_response()
}

#[cfg(test)]
mod tests {
    use crate::compat::opencode::test_utils::{
        normalize_generated_opencode_route_doc, normalize_opencode_route_doc,
        opencode_openapi_options, pinned_opencode_openapi,
    };
    use crate::utils::openapi::{generate_from_router, subset_for_operations};

    #[test]
    fn experimental_tool_ids_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_generated_opencode_route_doc(&generate_from_router(
                super::experimental_tool_ids_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/experimental/tool/ids", "get")],
            ))
        );
    }

    #[test]
    fn experimental_tool_list_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_generated_opencode_route_doc(&generate_from_router(
                super::experimental_tool_list_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/experimental/tool", "get")],
            ))
        );
    }

    #[test]
    fn experimental_workspace_list_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_generated_opencode_route_doc(&generate_from_router(
                super::experimental_workspace_list_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/experimental/workspace", "get")],
            ))
        );
    }

    #[test]
    fn experimental_workspace_create_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_generated_opencode_route_doc(&generate_from_router(
                super::experimental_workspace_create_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/experimental/workspace", "post")],
            ))
        );
    }

    #[test]
    fn experimental_workspace_remove_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_generated_opencode_route_doc(&generate_from_router(
                super::experimental_workspace_remove_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/experimental/workspace/{id}", "delete")],
            ))
        );
    }

    #[test]
    fn experimental_worktree_list_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_generated_opencode_route_doc(&generate_from_router(
                super::experimental_worktree_list_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/experimental/worktree", "get")],
            ))
        );
    }

    #[test]
    fn experimental_worktree_create_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_generated_opencode_route_doc(&generate_from_router(
                super::experimental_worktree_create_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/experimental/worktree", "post")],
            ))
        );
    }

    #[test]
    fn experimental_worktree_remove_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_generated_opencode_route_doc(&generate_from_router(
                super::experimental_worktree_remove_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/experimental/worktree", "delete")],
            ))
        );
    }

    #[test]
    fn experimental_worktree_reset_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_generated_opencode_route_doc(&generate_from_router(
                super::experimental_worktree_reset_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/experimental/worktree/reset", "post")],
            ))
        );
    }

    #[test]
    fn experimental_session_list_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_generated_opencode_route_doc(&generate_from_router(
                super::experimental_session_list_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/experimental/session", "get")],
            ))
        );
    }

    #[test]
    fn experimental_resource_list_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_generated_opencode_route_doc(&generate_from_router(
                super::experimental_resource_list_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/experimental/resource", "get")],
            ))
        );
    }
}
