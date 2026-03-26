use aide::axum::{
    ApiRouter,
    routing::{get_with, patch_with},
};

use super::super::types::common::CompatQuery;
use super::super::types::errors::BadRequestErrorDoc;
use super::super::types::global::CompatConfigDoc;
use super::super::types::provider::ConfigProvidersResponseDoc;
use super::super::*;

pub(super) fn routes() -> ApiRouter<AppState> {
    ApiRouter::new()
        .merge(config_get_route())
        .merge(config_patch_route())
        .merge(config_providers_route())
}

fn config_get_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/config",
        get_with(config_get, |operation| {
            operation
                .id("config.get")
                .summary("Get configuration")
                .description(
                    "Retrieve the current OpenCode configuration settings and preferences.",
                )
                .response_with::<200, Json<CompatConfigDoc>, _>(|res| {
                    res.description("Get config info")
                })
        }),
    )
}

fn config_patch_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/config",
        patch_with(config_patch, |operation| {
            operation
                .id("config.update")
                .summary("Update configuration")
                .description("Update OpenCode configuration settings and preferences.")
                .response_with::<200, Json<CompatConfigDoc>, _>(|res| {
                    res.description("Successfully updated config")
                })
                .response_with::<400, Json<BadRequestErrorDoc>, _>(|res| {
                    res.description("Bad request")
                })
        }),
    )
}

fn config_providers_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/config/providers",
        get_with(config_providers, |operation| {
            operation
                .id("config.providers")
                .summary("List config providers")
                .description("Get a list of all configured AI providers and their default models.")
                .response_with::<200, Json<ConfigProvidersResponseDoc>, _>(|res| {
                    res.description("List of providers")
                })
        }),
    )
}

async fn config_get(State(server): State<AppState>, Query(_query): Query<CompatQuery>) -> Response {
    let value = server.compat().config.read().await.clone();
    let body = serde_json::from_value::<CompatConfigDoc>(value).unwrap_or_default();
    Json(body).into_response()
}

async fn config_patch(
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
    Json(body): Json<CompatConfigDoc>,
) -> Response {
    *server.compat().config.write().await =
        serde_json::to_value(&body).unwrap_or_else(|_| Value::Object(Default::default()));
    Json(body).into_response()
}

async fn config_providers(
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
) -> Response {
    let providers = provider_array(&server);
    Json(ConfigProvidersResponseDoc {
        default: provider_default_map(&providers),
        providers,
    })
    .into_response()
}

#[cfg(test)]
mod tests {
    use crate::{
        compat::opencode::test_utils::{
            normalize_generated_opencode_route_doc, normalize_opencode_route_doc,
            opencode_openapi_options, pinned_opencode_openapi,
        },
        utils::openapi::{generate_from_router, subset_for_operations},
    };

    #[test]
    fn config_get_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_generated_opencode_route_doc(&generate_from_router(
                super::config_get_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/config", "get")],
            ))
        );
    }

    #[test]
    fn config_patch_route_openapi_matches_pinned_subset() {
        let generated = subset_for_operations(
            &generate_from_router(super::config_patch_route, opencode_openapi_options()),
            &[("/config", "patch")],
        );
        let expected = subset_for_operations(&pinned_opencode_openapi(), &[("/config", "patch")]);
        assert_eq!(
            normalize_generated_opencode_route_doc(&generated),
            normalize_opencode_route_doc(&expected)
        );
    }

    #[test]
    fn config_providers_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_generated_opencode_route_doc(&generate_from_router(
                super::config_providers_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/config/providers", "get")],
            ))
        );
    }
}
