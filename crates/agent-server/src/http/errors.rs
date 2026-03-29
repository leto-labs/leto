use agent_core::CoreError;
use agent_store::StoreError;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use serde_json::json;

use crate::types::ErrorResponse;

pub(super) fn core_error_response(error: CoreError) -> Response {
    match error {
        CoreError::Store(error) => store_error_response(error),
        CoreError::ProviderNotRegistered(name) => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::with_details(
                "provider_not_registered",
                format!("provider not registered: {name}"),
                json!({ "name": name }),
            )),
        )
            .into_response(),
        CoreError::LoopNotRegistered(name) => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::with_details(
                "loop_not_registered",
                format!("loop not registered: {name}"),
                json!({ "name": name }),
            )),
        )
            .into_response(),
        CoreError::TurnActive(session_id) => (
            StatusCode::CONFLICT,
            Json(ErrorResponse::with_details(
                "turn_active",
                format!("a turn is already active for session {session_id}"),
                json!({ "session_id": session_id.to_string() }),
            )),
        )
            .into_response(),
        CoreError::NoProvidersRegistered => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "no_providers_registered",
                "no providers are registered",
            )),
        )
            .into_response(),
        CoreError::NoLoopsRegistered => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "no_loops_registered",
                "no loops are registered",
            )),
        )
            .into_response(),
        CoreError::Runtime(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("runtime_error", error.to_string())),
        )
            .into_response(),
        CoreError::Bootstrap(error) => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "project_bootstrap_error",
                error.to_string(),
            )),
        )
            .into_response(),
        CoreError::ProviderOpenAi(error) => (
            StatusCode::BAD_GATEWAY,
            Json(ErrorResponse::new(
                "provider_openai_error",
                error.to_string(),
            )),
        )
            .into_response(),
        CoreError::Internal(message) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("internal_error", message)),
        )
            .into_response(),
    }
}

pub(super) fn store_error_response(error: StoreError) -> Response {
    match error {
        StoreError::NotFound(message) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("store_not_found", message)),
        )
            .into_response(),
        StoreError::AlreadyExists(message) => (
            StatusCode::CONFLICT,
            Json(ErrorResponse::new("store_already_exists", message)),
        )
            .into_response(),
        StoreError::InvalidInput(message) => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("store_invalid_input", message)),
        )
            .into_response(),
        StoreError::Io(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("store_io", error.to_string())),
        )
            .into_response(),
        StoreError::Serde(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("store_serde", error.to_string())),
        )
            .into_response(),
    }
}
