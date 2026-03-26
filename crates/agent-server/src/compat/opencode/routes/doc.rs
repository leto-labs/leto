use std::sync::Arc;

use aide::axum::ApiRouter;
use axum::Extension;
use axum::routing as axum_routing;
use serde_json::Value;

use super::super::*;

pub(super) fn routes() -> ApiRouter<AppState> {
    ApiRouter::new().merge(doc_route())
}

fn doc_route() -> ApiRouter<AppState> {
    ApiRouter::new().route("/doc", axum_routing::get(doc_openapi))
}

async fn doc_openapi(Extension(doc): Extension<Arc<Value>>) -> Json<Value> {
    Json(doc.as_ref().clone())
}
