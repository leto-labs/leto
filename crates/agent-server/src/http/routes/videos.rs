use axum::extract::{Multipart, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use chrono::Utc;
use provider_openai::VideoObject;
use serde_json::json;

use super::super::AppState;
use crate::types::ErrorResponse;

pub(in crate::http) async fn create_video(
    State(server): State<AppState>,
    mut multipart: Multipart,
) -> Response {
    let mut prompt = None;
    let mut model = None;
    let mut seconds = None;
    let mut size = None;

    loop {
        let next_field = match multipart.next_field().await {
            Ok(field) => field,
            Err(error) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::with_details(
                        "invalid_multipart",
                        "video requests must use a valid multipart body",
                        json!({ "message": error.to_string() }),
                    )),
                )
                    .into_response();
            }
        };

        let Some(field) = next_field else {
            break;
        };

        let name = field.name().map(str::to_owned);
        let value = match field.text().await {
            Ok(value) => value,
            Err(error) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::with_details(
                        "invalid_multipart",
                        "video request fields must be valid UTF-8 text",
                        json!({ "message": error.to_string() }),
                    )),
                )
                    .into_response();
            }
        };

        match name.as_deref() {
            Some("prompt") => prompt = Some(value),
            Some("model") => model = Some(value),
            Some("seconds") => seconds = Some(value),
            Some("size") => size = Some(value),
            _ => {}
        }
    }

    let Some(prompt) = prompt else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "missing_prompt",
                "video requests must include a prompt",
            )),
        )
            .into_response();
    };

    if prompt.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "missing_prompt",
                "video requests must include a prompt",
            )),
        )
            .into_response();
    }

    if let Some(model_id) = model.as_ref() {
        let core = server.core();
        if !core
            .list_models()
            .into_iter()
            .any(|record| record.model.id == *model_id)
        {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::with_details(
                    "model_not_found",
                    format!("model not found: {model_id}"),
                    json!({ "id": model_id }),
                )),
            )
                .into_response();
        }
    }

    Json(VideoObject {
        id: "video_agent_server".to_owned(),
        object: "video".to_owned(),
        model,
        status: Some("queued".to_owned()),
        progress: Some(0),
        created_at: Some(Utc::now().timestamp()),
        size,
        seconds,
        quality: Some("standard".to_owned()),
    })
    .into_response()
}
