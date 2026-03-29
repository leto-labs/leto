use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::super::AppState;
use crate::types::ErrorResponse;

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub(in crate::http) enum ModerationInput {
    Text(String),
    Texts(Vec<String>),
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub(in crate::http) struct ModerationRequest {
    pub model: Option<String>,
    pub input: Option<ModerationInput>,
}

#[derive(Debug, Clone, Serialize)]
pub(in crate::http) struct ModerationResponse {
    pub id: String,
    pub model: Option<String>,
    pub results: Vec<ModerationResult>,
}

#[derive(Debug, Clone, Serialize)]
pub(in crate::http) struct ModerationResult {
    pub flagged: bool,
    pub categories: ModerationCategories,
    pub category_scores: ModerationCategoryScores,
}

#[derive(Debug, Clone, Serialize)]
pub(in crate::http) struct ModerationCategories {
    pub hate: bool,
    #[serde(rename = "hate/threatening")]
    pub hate_threatening: bool,
    pub harassment: bool,
    #[serde(rename = "harassment/threatening")]
    pub harassment_threatening: bool,
    pub self_harm: bool,
    #[serde(rename = "self-harm/intent")]
    pub self_harm_intent: bool,
    #[serde(rename = "self-harm/instructions")]
    pub self_harm_instructions: bool,
    pub sexual: bool,
    #[serde(rename = "sexual/minors")]
    pub sexual_minors: bool,
    pub violence: bool,
    #[serde(rename = "violence/graphic")]
    pub violence_graphic: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(in crate::http) struct ModerationCategoryScores {
    pub hate: f32,
    #[serde(rename = "hate/threatening")]
    pub hate_threatening: f32,
    pub harassment: f32,
    #[serde(rename = "harassment/threatening")]
    pub harassment_threatening: f32,
    #[serde(rename = "self-harm")]
    pub self_harm: f32,
    #[serde(rename = "self-harm/intent")]
    pub self_harm_intent: f32,
    #[serde(rename = "self-harm/instructions")]
    pub self_harm_instructions: f32,
    pub sexual: f32,
    #[serde(rename = "sexual/minors")]
    pub sexual_minors: f32,
    pub violence: f32,
    #[serde(rename = "violence/graphic")]
    pub violence_graphic: f32,
}

pub(in crate::http) async fn create_moderation(
    State(server): State<AppState>,
    Json(body): Json<ModerationRequest>,
) -> Response {
    let Some(input) = body.input else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "missing_input",
                "moderation requests must include input",
            )),
        )
            .into_response();
    };

    let inputs = match input {
        ModerationInput::Text(text) if text.trim().is_empty() => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "invalid_input",
                    "input must contain at least one value",
                )),
            )
                .into_response();
        }
        ModerationInput::Text(text) => vec![text],
        ModerationInput::Texts(inputs) if inputs.is_empty() => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "invalid_input",
                    "input must contain at least one value",
                )),
            )
                .into_response();
        }
        ModerationInput::Texts(inputs) if inputs.iter().all(|input| input.trim().is_empty()) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "invalid_input",
                    "input must contain at least one value",
                )),
            )
                .into_response();
        }
        ModerationInput::Texts(inputs) => inputs,
    };

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

    let results = inputs
        .into_iter()
        .map(|_| empty_moderation_result())
        .collect();

    Json(ModerationResponse {
        id: "modr-agent-server".to_owned(),
        model: body.model,
        results,
    })
    .into_response()
}

fn empty_moderation_result() -> ModerationResult {
    ModerationResult {
        flagged: false,
        categories: ModerationCategories {
            hate: false,
            hate_threatening: false,
            harassment: false,
            harassment_threatening: false,
            self_harm: false,
            self_harm_intent: false,
            self_harm_instructions: false,
            sexual: false,
            sexual_minors: false,
            violence: false,
            violence_graphic: false,
        },
        category_scores: ModerationCategoryScores {
            hate: 0.0,
            hate_threatening: 0.0,
            harassment: 0.0,
            harassment_threatening: 0.0,
            self_harm: 0.0,
            self_harm_intent: 0.0,
            self_harm_instructions: 0.0,
            sexual: 0.0,
            sexual_minors: 0.0,
            violence: 0.0,
            violence_graphic: 0.0,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::ModerationInput;

    #[test]
    fn moderation_input_deserializes_single_text() {
        let input: ModerationInput = serde_json::from_value(serde_json::json!("hello")).unwrap();

        assert!(matches!(input, ModerationInput::Text(text) if text == "hello"));
    }

    #[test]
    fn moderation_input_deserializes_text_arrays() {
        let input: ModerationInput =
            serde_json::from_value(serde_json::json!(["hello", "world"])).unwrap();

        assert!(matches!(
            input,
            ModerationInput::Texts(values) if values == ["hello", "world"]
        ));
    }
}
