mod config;
mod doc;
mod experimental;
mod files;
mod global;
mod mcp;
mod permission;
mod project;
mod provider;
mod pty;
mod question;
mod session;
mod tui;

use std::sync::Arc;

use aide::axum::ApiRouter;
use aide::openapi::OpenApi;
use aide::transform::TransformOpenApi;
use axum::Extension;
use axum::Router;

use super::AppState;

fn api_router() -> ApiRouter<AppState> {
    ApiRouter::new()
        .merge(doc::routes())
        .merge(global::routes())
        .merge(project::routes())
        .merge(config::routes())
        .merge(provider::routes())
        .merge(session::routes())
        .merge(permission::routes())
        .merge(question::routes())
        .merge(files::routes())
        .merge(mcp::routes())
        .merge(experimental::routes())
        .merge(pty::routes())
        .merge(tui::routes())
}

pub fn build_router() -> Router<AppState> {
    aide::generate::reset_context();
    aide::generate::on_error(|error| {
        tracing::warn!(?error, "aide failed while generating compat /doc");
    });
    aide::generate::extract_schemas(true);
    aide::generate::infer_responses(false);

    let mut openapi = OpenApi::default();
    let router = api_router().finish_api_with(&mut openapi, |api: TransformOpenApi<'_>| {
        api.title("opencode")
            .description("opencode api")
            .version("0.0.3")
    });

    let mut doc =
        serde_json::to_value(openapi).unwrap_or_else(|_| serde_json::json!({"paths": {}}));
    doc["openapi"] = serde_json::json!("3.1.1");
    router.layer(Extension(Arc::new(doc)))
}
