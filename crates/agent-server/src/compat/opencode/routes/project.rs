use super::super::types::common::CompatQuery;
use super::super::types::errors::{BadRequestErrorDoc, NotFoundErrorDoc};
use super::super::types::project::{ProjectDoc, ProjectIdPath, ProjectUpdateRequest};
use super::super::*;
use aide::axum::{
    ApiRouter,
    routing::{get_with, patch_with, post_with},
};

pub(super) fn routes() -> ApiRouter<AppState> {
    ApiRouter::new()
        .merge(project_list_route())
        .merge(project_current_route())
        .merge(project_git_init_route())
        .merge(project_update_route())
}

fn project_list_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/project",
        get_with(projects, |operation| {
            operation
                .id("project.list")
                .summary("List all projects")
                .description("Get a list of projects that have been opened with OpenCode.")
                .response_with::<200, Json<Vec<ProjectDoc>>, _>(|res| {
                    res.description("List of projects")
                })
        }),
    )
}

fn project_current_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/project/current",
        get_with(project_current, |operation| {
            operation
                .id("project.current")
                .summary("Get current project")
                .description("Retrieve the currently active project that OpenCode is working with.")
                .response_with::<200, Json<ProjectDoc>, _>(|res| {
                    res.description("Current project information")
                })
        }),
    )
}

fn project_git_init_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/project/git/init",
        post_with(project_git_init, |operation| {
            operation
                .id("project.initGit")
                .summary("Initialize git repository")
                .description(
                    "Create a git repository for the current project and return the refreshed project info.",
                )
                .response_with::<200, Json<ProjectDoc>, _>(|res| {
                    res.description("Project information after git initialization")
                })
        }),
    )
}

fn project_update_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/project/{projectID}",
        patch_with(project_update, |operation| {
            operation
                .id("project.update")
                .summary("Update project")
                .description("Update project properties such as name, icon, and commands.")
                .response_with::<200, Json<ProjectDoc>, _>(|res| {
                    res.description("Updated project information")
                })
                .response_with::<400, Json<BadRequestErrorDoc>, _>(|res| {
                    res.description("Bad request")
                })
                .response_with::<404, Json<NotFoundErrorDoc>, _>(|res| res.description("Not found"))
        }),
    )
}

async fn projects(State(server): State<AppState>, Query(_query): Query<CompatQuery>) -> Response {
    match server.core().store().projects().list().await {
        Ok(projects) => {
            let mut docs = Vec::with_capacity(projects.len());
            let compat = server.compat();
            for project in &projects {
                docs.push(compat_project(project, &compat).await);
            }
            Json(docs).into_response()
        }
        Err(error) => compat_error("store_error", error.to_string()),
    }
}

async fn project_current(
    State(server): State<AppState>,
    Query(params): Query<CompatQuery>,
) -> Response {
    match resolve_current_project(&server, &params).await {
        Ok(project) => Json(compat_project(&project, &server.compat()).await).into_response(),
        Err(response) => response,
    }
}

async fn project_git_init(
    State(server): State<AppState>,
    Query(params): Query<CompatQuery>,
) -> Response {
    let project = match resolve_current_project(&server, &params).await {
        Ok(project) => project,
        Err(response) => return response,
    };
    if let Some(root) = &project.root {
        let _ = Command::new("git").arg("-C").arg(root).arg("init").output();
    }
    Json(compat_project(&project, &server.compat()).await).into_response()
}

async fn project_update(
    State(server): State<AppState>,
    Path(ProjectIdPath { project_id }): Path<ProjectIdPath>,
    Query(_query): Query<CompatQuery>,
    Json(body): Json<ProjectUpdateRequest>,
) -> Response {
    let project_id = match project_id.parse() {
        Ok(project_id) => project_id,
        Err(_) => return invalid_request("invalid_project_id"),
    };
    let core = server.core();
    let projects = core.store().projects();
    let mut project = match projects.get(project_id).await {
        Ok(project) => project,
        Err(error) => return compat_error("store_error", error.to_string()),
    };
    if let Some(name) = body.name {
        project.name = Some(name);
    }
    match projects.update(project_id, project.clone()).await {
        Ok(project) => Json(compat_project(&project, &server.compat()).await).into_response(),
        Err(error) => compat_error("store_error", error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use crate::compat::opencode::test_utils::{
        normalize_generated_opencode_route_doc, normalize_opencode_route_doc,
        opencode_openapi_options, pinned_opencode_openapi,
    };
    use crate::utils::openapi::{generate_from_router, subset_for_operations};

    #[test]
    fn project_list_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_generated_opencode_route_doc(&generate_from_router(
                super::project_list_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/project", "get")],
            ))
        );
    }

    #[test]
    fn project_current_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_generated_opencode_route_doc(&generate_from_router(
                super::project_current_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/project/current", "get")],
            ))
        );
    }

    #[test]
    fn project_git_init_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_generated_opencode_route_doc(&generate_from_router(
                super::project_git_init_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/project/git/init", "post")],
            ))
        );
    }

    #[test]
    fn project_update_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_generated_opencode_route_doc(&generate_from_router(
                super::project_update_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/project/{projectID}", "patch")],
            ))
        );
    }
}
