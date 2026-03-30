use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

use super::super::{AppState, grouped_provider_models};
use crate::types::{ErrorResponse, ProviderModelRecord};

pub(in crate::http) async fn list_providers(State(server): State<AppState>) -> Response {
    let core = server.core();
    Json(grouped_provider_models(&core)).into_response()
}

pub(in crate::http) async fn list_models(State(server): State<AppState>) -> Response {
    let core = server.core();
    Json(
        core.list_models()
            .into_iter()
            .map(ProviderModelRecord::from)
            .collect::<Vec<_>>(),
    )
    .into_response()
}

pub(in crate::http) async fn get_model(
    State(server): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    let core = server.core();
    match core
        .list_models()
        .into_iter()
        .find(|record| record.model.id == id)
    {
        Some(record) => Json(ProviderModelRecord::from(record)).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::with_details(
                "model_not_found",
                format!("model not found: {id}"),
                json!({ "id": id }),
            )),
        )
            .into_response(),
    }
}
