use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::super::AppState;
use crate::types::ErrorResponse;

const DEFAULT_IMAGE_COUNT: u32 = 1;
const MAX_IMAGE_COUNT: u32 = 10;
const PLACEHOLDER_IMAGE_PNG_BASE64: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAukB9pM5v1wAAAAASUVORK5CYII=";

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub(in crate::http) struct ImageGenerationRequest {
    pub prompt: String,
    pub model: Option<String>,
    pub n: Option<u32>,
    pub size: Option<String>,
    pub response_format: Option<String>,
    pub user: Option<String>,
}

impl Default for ImageGenerationRequest {
    fn default() -> Self {
        Self {
            prompt: String::new(),
            model: None,
            n: None,
            size: None,
            response_format: None,
            user: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(in crate::http) struct ImageGenerationResponse {
    pub created: i64,
    pub data: Vec<ImageGenerationData>,
}

#[derive(Debug, Clone, Serialize)]
pub(in crate::http) struct ImageGenerationData {
    pub b64_json: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revised_prompt: Option<String>,
}

pub(in crate::http) async fn create_image_generation(
    State(server): State<AppState>,
    Json(body): Json<ImageGenerationRequest>,
) -> Response {
    if body.prompt.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "missing_prompt",
                "image generation requests must include a prompt",
            )),
        )
            .into_response();
    }

    if body
        .response_format
        .as_deref()
        .is_some_and(|format| format != "b64_json")
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::with_details(
                "unsupported_response_format",
                "only b64_json response_format is supported",
                json!({ "response_format": body.response_format }),
            )),
        )
            .into_response();
    }

    let image_count = body.n.unwrap_or(DEFAULT_IMAGE_COUNT);
    if image_count == 0 || image_count > MAX_IMAGE_COUNT {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::with_details(
                "invalid_image_count",
                format!("n must be between 1 and {MAX_IMAGE_COUNT}"),
                json!({ "n": image_count }),
            )),
        )
            .into_response();
    }

    if let Some(model) = body.model.as_ref() {
        let core = server.core();
        if !core
            .list_models()
            .into_iter()
            .any(|record| record.model.id == *model)
        {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::with_details(
                    "model_not_found",
                    format!("model not found: {model}"),
                    json!({ "id": model }),
                )),
            )
                .into_response();
        }
    }

    let revised_prompt = revise_prompt(&body.prompt);
    let data = (0..image_count)
        .map(|_| ImageGenerationData {
            b64_json: PLACEHOLDER_IMAGE_PNG_BASE64.to_owned(),
            revised_prompt: Some(revised_prompt.clone()),
        })
        .collect();

    Json(ImageGenerationResponse {
        created: Utc::now().timestamp(),
        data,
    })
    .into_response()
}

fn revise_prompt(prompt: &str) -> String {
    prompt.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::{MAX_IMAGE_COUNT, revise_prompt};

    #[test]
    fn revise_prompt_normalizes_whitespace() {
        assert_eq!(
            revise_prompt("  hello   image  world "),
            "hello image world"
        );
    }

    #[test]
    fn max_image_count_is_large_enough_for_basic_batch_tests() {
        assert!(MAX_IMAGE_COUNT >= 4);
    }
}
