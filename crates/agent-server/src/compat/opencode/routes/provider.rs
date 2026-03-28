use aide::axum::{
    ApiRouter,
    routing::{delete_with, get_with, post_with, put_with},
};

use super::super::types::common::CompatQuery;
use super::super::types::errors::BadRequestErrorDoc;
use super::super::types::provider::{
    AuthSetRequest, ModelInterleavedFieldValueDoc, ProviderAuthAuthorizationCodeTypeDoc,
    ProviderAuthAuthorizationDoc, ProviderAuthAuthorizationMethodDoc, ProviderAuthMethodApiTypeDoc,
    ProviderAuthMethodDoc, ProviderAuthMethodTypeDoc, ProviderCostConfigDoc,
    ProviderCostTierConfigDoc, ProviderIdPath, ProviderInterleavedConfigDoc,
    ProviderInterleavedFieldDoc, ProviderInterleavedFieldValueDoc, ProviderLimitConfigDoc,
    ProviderListItemDoc, ProviderListModelDoc, ProviderListResponseDoc,
    ProviderModalitiesConfigDoc, ProviderModalityDoc, ProviderOAuthAuthorizeRequest,
    ProviderOAuthCallbackRequest, ProviderSourceConfigDoc, ProviderStatusConfigDoc, TrueConstDoc,
};
use super::super::*;
use axum::extract::Query;

pub(super) fn routes() -> ApiRouter<AppState> {
    ApiRouter::new()
        .merge(provider_list_route())
        .merge(provider_auth_route())
        .merge(provider_oauth_authorize_route())
        .merge(provider_oauth_callback_route())
        .merge(auth_set_route())
        .merge(auth_remove_route())
}

fn provider_list_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/provider",
        get_with(providers, |operation| {
            operation
                .id("provider.list")
                .summary("List providers")
                .description(
                    "Get a list of all available AI providers, including both available and connected ones.",
                )
                .response_with::<200, Json<ProviderListResponseDoc>, _>(|res| {
                    res.description("List of providers")
                })
        }),
    )
}

fn provider_auth_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/provider/auth",
        get_with(provider_auth, |operation| {
            operation
                .id("provider.auth")
                .summary("Get provider auth methods")
                .description("Retrieve available authentication methods for all AI providers.")
                .response_with::<200, Json<BTreeMap<String, Vec<ProviderAuthMethodDoc>>>, _>(
                    |res| res.description("Provider auth methods"),
                )
        }),
    )
}

fn provider_oauth_authorize_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/provider/{providerID}/oauth/authorize",
        post_with(provider_oauth_authorize, |operation| {
            operation
                .id("provider.oauth.authorize")
                .summary("OAuth authorize")
                .description(
                    "Initiate OAuth authorization for a specific AI provider to get an authorization URL.",
                )
                .response_with::<200, Json<ProviderAuthAuthorizationDoc>, _>(|res| {
                    res.description("Authorization URL and method")
                })
                .response_with::<400, Json<BadRequestErrorDoc>, _>(|res| {
                    res.description("Bad request")
                })
        }),
    )
}

fn provider_oauth_callback_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/provider/{providerID}/oauth/callback",
        post_with(provider_oauth_callback, |operation| {
            operation
                .id("provider.oauth.callback")
                .summary("OAuth callback")
                .description("Handle the OAuth callback from a provider after user authorization.")
                .response_with::<200, Json<bool>, _>(|res| {
                    res.description("OAuth callback processed successfully")
                })
                .response_with::<400, Json<BadRequestErrorDoc>, _>(|res| {
                    res.description("Bad request")
                })
        }),
    )
}

fn auth_set_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/auth/{providerID}",
        put_with(auth_set, |operation| {
            operation
                .id("auth.set")
                .summary("Set auth credentials")
                .description("Set authentication credentials")
                .response_with::<200, Json<bool>, _>(|res| {
                    res.description("Successfully set authentication credentials")
                })
                .response_with::<400, Json<BadRequestErrorDoc>, _>(|res| {
                    res.description("Bad request")
                })
        }),
    )
}

fn auth_remove_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/auth/{providerID}",
        delete_with(auth_remove, |operation| {
            operation
                .id("auth.remove")
                .summary("Remove auth credentials")
                .description("Remove authentication credentials")
                .response_with::<200, Json<bool>, _>(|res| {
                    res.description("Successfully removed authentication credentials")
                })
                .response_with::<400, Json<BadRequestErrorDoc>, _>(|res| {
                    res.description("Bad request")
                })
        }),
    )
}

async fn providers(State(server): State<AppState>, Query(_query): Query<CompatQuery>) -> Response {
    let providers = provider_array(&server);
    let all = providers.iter().map(provider_list_item).collect::<Vec<_>>();
    let connected = match server.core().store().credentials().list().await {
        Ok(entries) => entries
            .into_iter()
            .map(|(key, _)| key.0)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>(),
        Err(_) => Vec::new(),
    };
    Json(ProviderListResponseDoc {
        default: provider_default_map(&providers),
        all,
        connected,
    })
    .into_response()
}

fn provider_list_item(provider: &ProviderDoc) -> ProviderListItemDoc {
    ProviderListItemDoc {
        api: None,
        name: provider.name.clone(),
        env: provider.env.clone(),
        id: provider.id.clone(),
        npm: None,
        models: provider
            .models
            .iter()
            .map(|(id, model)| (id.clone(), provider_list_model(model)))
            .collect(),
    }
}

fn provider_list_model(model: &ModelDoc) -> ProviderListModelDoc {
    ProviderListModelDoc {
        id: model.id.clone(),
        name: model.name.clone(),
        family: model.family.clone(),
        release_date: model.release_date.clone(),
        attachment: model.capabilities.attachment,
        reasoning: model.capabilities.reasoning,
        temperature: model.capabilities.temperature,
        tool_call: model.capabilities.toolcall,
        interleaved: match &model.capabilities.interleaved {
            ModelInterleavedDoc::Bool(true) => {
                Some(ProviderInterleavedConfigDoc::Enabled(TrueConstDoc))
            }
            ModelInterleavedDoc::Bool(false) => None,
            ModelInterleavedDoc::Field(field) => Some(ProviderInterleavedConfigDoc::Field(
                ProviderInterleavedFieldDoc {
                    field: match field.field {
                        ModelInterleavedFieldValueDoc::ReasoningContent => {
                            ProviderInterleavedFieldValueDoc::ReasoningContent
                        }
                        ModelInterleavedFieldValueDoc::ReasoningDetails => {
                            ProviderInterleavedFieldValueDoc::ReasoningDetails
                        }
                    },
                },
            )),
        },
        cost: Some(ProviderCostConfigDoc {
            input: model.cost.input,
            output: model.cost.output,
            cache_read: Some(model.cost.cache.read),
            cache_write: Some(model.cost.cache.write),
            context_over_200k: model.cost.experimental_over_200k.as_ref().map(|tier| {
                ProviderCostTierConfigDoc {
                    input: tier.input,
                    output: tier.output,
                    cache_read: Some(tier.cache.read),
                    cache_write: Some(tier.cache.write),
                }
            }),
        }),
        limit: ProviderLimitConfigDoc {
            context: model.limit.context,
            input: model.limit.input,
            output: model.limit.output,
        },
        modalities: Some(ProviderModalitiesConfigDoc {
            input: provider_modalities(&model.capabilities.input),
            output: provider_modalities(&model.capabilities.output),
        }),
        experimental: None,
        status: match model.status {
            ModelStatusDoc::Active => None,
            ModelStatusDoc::Alpha => Some(ProviderStatusConfigDoc::Alpha),
            ModelStatusDoc::Beta => Some(ProviderStatusConfigDoc::Beta),
            ModelStatusDoc::Deprecated => Some(ProviderStatusConfigDoc::Deprecated),
        },
        options: model.options.clone(),
        headers: if model.headers.is_empty() {
            None
        } else {
            Some(model.headers.clone())
        },
        provider: Some(ProviderSourceConfigDoc {
            npm: Some(model.api.npm.clone()),
            api: Some(model.api.url.clone()),
        }),
        variants: model.variants.clone(),
    }
}

fn provider_modalities(io: &ModelIoDoc) -> Vec<ProviderModalityDoc> {
    let mut modalities = Vec::new();
    if io.text {
        modalities.push(ProviderModalityDoc::Text);
    }
    if io.audio {
        modalities.push(ProviderModalityDoc::Audio);
    }
    if io.image {
        modalities.push(ProviderModalityDoc::Image);
    }
    if io.video {
        modalities.push(ProviderModalityDoc::Video);
    }
    if io.pdf {
        modalities.push(ProviderModalityDoc::Pdf);
    }
    modalities
}

async fn provider_auth(
    headers: HeaderMap,
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
) -> Response {
    if let Err(response) = require_bearer_token(&headers) {
        return response.into_response();
    }
    let payload = server
        .core()
        .provider_names()
        .into_iter()
        .map(|name| {
            (
                name,
                vec![ProviderAuthMethodDoc {
                    method_type: ProviderAuthMethodTypeDoc::Api(ProviderAuthMethodApiTypeDoc::Api),
                    label: "API Key".to_owned(),
                    prompts: Vec::new(),
                }],
            )
        })
        .collect::<BTreeMap<String, Vec<ProviderAuthMethodDoc>>>();
    Json(payload).into_response()
}

async fn provider_oauth_authorize(
    headers: HeaderMap,
    Query(_query): Query<CompatQuery>,
    Path(ProviderIdPath { provider_id }): Path<ProviderIdPath>,
    Json(_body): Json<ProviderOAuthAuthorizeRequest>,
) -> Response {
    if let Err(response) = require_bearer_token(&headers) {
        return response.into_response();
    }
    Json(ProviderAuthAuthorizationDoc {
        url: format!("https://example.invalid/oauth/{provider_id}"),
        method: ProviderAuthAuthorizationMethodDoc::Code(
            ProviderAuthAuthorizationCodeTypeDoc::Code,
        ),
        instructions: format!("Paste an authorization code for provider {provider_id}."),
    })
    .into_response()
}

async fn provider_oauth_callback(
    headers: HeaderMap,
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
    Path(ProviderIdPath { provider_id }): Path<ProviderIdPath>,
    Json(body): Json<ProviderOAuthCallbackRequest>,
) -> Response {
    if let Err(response) = require_bearer_token(&headers) {
        return response.into_response();
    }
    let entry = compat_oauth_entry(
        provider_id.clone(),
        body.code.unwrap_or_default(),
        "compat-refresh".to_owned(),
        Utc::now() + chrono::Duration::hours(1),
        None,
    );
    let _ = upsert_auth_credential(&server, &provider_id, entry).await;
    Json(true).into_response()
}

async fn auth_set(
    headers: HeaderMap,
    State(server): State<AppState>,
    Path(ProviderIdPath { provider_id }): Path<ProviderIdPath>,
    Json(body): Json<AuthSetRequest>,
) -> Response {
    if let Err(response) = require_bearer_token(&headers) {
        return response.into_response();
    }
    let entry = match body {
        AuthSetRequest::Api(ApiAuthRequest { key, .. }) => {
            CredentialEntry::api_key(provider_id.clone(), &key)
        }
        AuthSetRequest::Oauth(OAuthAuthRequest {
            access,
            refresh,
            expires,
            account_id,
            ..
        }) => compat_oauth_entry(
            provider_id.clone(),
            access,
            refresh,
            DateTime::<Utc>::from_timestamp_millis(expires as i64)
                .unwrap_or_else(|| Utc::now() + chrono::Duration::hours(1)),
            account_id,
        ),
        AuthSetRequest::Wellknown(WellKnownAuthRequest { key, token, .. }) => {
            CredentialEntry::api_key(if key.is_empty() { &provider_id } else { &key }, &token)
        }
    };
    match upsert_auth_credential(&server, &provider_id, entry).await {
        Ok(()) => Json(true).into_response(),
        Err(response) => response,
    }
}

async fn auth_remove(
    headers: HeaderMap,
    State(server): State<AppState>,
    Path(ProviderIdPath { provider_id }): Path<ProviderIdPath>,
) -> Response {
    if let Err(response) = require_bearer_token(&headers) {
        return response.into_response();
    }
    let core = server.core();
    let store = core.store();
    let credentials = store.credentials();
    let _ = credentials
        .delete((provider_id.clone(), "default".to_owned()))
        .await;
    let _ = credentials.delete((provider_id.clone(), provider_id)).await;
    Json(true).into_response()
}

fn compat_oauth_entry(
    provider_id: String,
    access_token: String,
    refresh_token: String,
    expires_at: DateTime<Utc>,
    account_id: Option<String>,
) -> CredentialEntry {
    CredentialEntry {
        id: "default".to_owned(),
        label: provider_id,
        credential: ProviderCredential::OAuth(OAuthCredentials {
            access_token,
            refresh_token,
            client_id: "compat-client".to_owned(),
            token_endpoint: "https://example.invalid/oauth/token".to_owned(),
            account_id,
            token_type: Some("bearer".to_owned()),
            expires_at,
            scopes: Vec::new(),
        }),
        enabled: true,
        created_at: Utc::now(),
        health: Default::default(),
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
    fn provider_list_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_generated_opencode_route_doc(&generate_from_router(
                super::provider_list_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/provider", "get")],
            ))
        );
    }

    #[test]
    fn provider_auth_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_generated_opencode_route_doc(&generate_from_router(
                super::provider_auth_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/provider/auth", "get")],
            ))
        );
    }

    #[test]
    fn provider_oauth_authorize_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_generated_opencode_route_doc(&generate_from_router(
                super::provider_oauth_authorize_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/provider/{providerID}/oauth/authorize", "post")],
            ))
        );
    }

    #[test]
    fn provider_oauth_callback_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_generated_opencode_route_doc(&generate_from_router(
                super::provider_oauth_callback_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/provider/{providerID}/oauth/callback", "post")],
            ))
        );
    }

    #[test]
    fn auth_set_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_generated_opencode_route_doc(&generate_from_router(
                super::auth_set_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/auth/{providerID}", "put")],
            ))
        );
    }

    #[test]
    fn auth_remove_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_generated_opencode_route_doc(&generate_from_router(
                super::auth_remove_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/auth/{providerID}", "delete")],
            ))
        );
    }
}
