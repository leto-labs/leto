use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use provider_openai::{
    EmbeddingData, EmbeddingInput, EmbeddingRequest, EmbeddingResponse, EmbeddingUsage,
};
use serde_json::json;

use super::super::AppState;
use crate::types::ErrorResponse;

const DEFAULT_EMBEDDING_DIMENSIONS: usize = 8;
const MAX_EMBEDDING_DIMENSIONS: usize = 1024;

pub(in crate::http) async fn create_embeddings(
    State(server): State<AppState>,
    Json(body): Json<EmbeddingRequest>,
) -> Response {
    if body
        .encoding_format
        .as_deref()
        .is_some_and(|value| value != "float")
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::with_details(
                "unsupported_encoding_format",
                "only float encoding_format is supported",
                json!({ "encoding_format": body.encoding_format }),
            )),
        )
            .into_response();
    }

    let Some(model) = body.model else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "missing_model",
                "embeddings requests must include a model",
            )),
        )
            .into_response();
    };

    let dimensions = body
        .dimensions
        .unwrap_or(DEFAULT_EMBEDDING_DIMENSIONS as u32) as usize;
    if dimensions == 0 || dimensions > MAX_EMBEDDING_DIMENSIONS {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::with_details(
                "invalid_dimensions",
                format!("dimensions must be between 1 and {MAX_EMBEDDING_DIMENSIONS}"),
                json!({ "dimensions": dimensions }),
            )),
        )
            .into_response();
    }

    let core = server.core();
    if !core
        .list_models()
        .into_iter()
        .any(|record| record.model.id == model)
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

    let inputs = match body.input {
        EmbeddingInput::Text(input) => vec![input],
        EmbeddingInput::Texts(inputs) if inputs.is_empty() => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "invalid_input",
                    "input must contain at least one value",
                )),
            )
                .into_response();
        }
        EmbeddingInput::Texts(inputs) => inputs,
    };

    let prompt_tokens = inputs.iter().map(|input| token_count(input)).sum();
    let data = inputs
        .into_iter()
        .enumerate()
        .map(|(index, input)| EmbeddingData {
            object: "embedding".into(),
            index: index as u32,
            embedding: embed_text(&input, dimensions),
        })
        .collect();

    Json(EmbeddingResponse {
        object: "list".into(),
        data,
        model: Some(model),
        usage: Some(EmbeddingUsage {
            prompt_tokens,
            total_tokens: prompt_tokens,
        }),
    })
    .into_response()
}

fn token_count(input: &str) -> u32 {
    let chars = input.chars().count() as u32;
    chars.max(1).div_ceil(4)
}

fn embed_text(input: &str, dimensions: usize) -> Vec<f32> {
    let mut embedding = vec![0.0; dimensions];
    if input.is_empty() {
        return embedding;
    }

    for (index, byte) in input.bytes().enumerate() {
        let slot = index % dimensions;
        let centered = ((byte as f32) / 255.0) * 2.0 - 1.0;
        embedding[slot] += centered + ((index % 11) as f32 * 0.01);
    }

    let norm = embedding
        .iter()
        .map(|value| value * value)
        .sum::<f32>()
        .sqrt();
    if norm > 0.0 {
        for value in &mut embedding {
            *value /= norm;
        }
    }

    embedding
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_EMBEDDING_DIMENSIONS, embed_text, token_count};

    #[test]
    fn embed_text_is_deterministic_and_uses_requested_dimensions() {
        let first = embed_text("hello embeddings", DEFAULT_EMBEDDING_DIMENSIONS);
        let second = embed_text("hello embeddings", DEFAULT_EMBEDDING_DIMENSIONS);

        assert_eq!(first, second);
        assert_eq!(first.len(), DEFAULT_EMBEDDING_DIMENSIONS);
        assert!(first.iter().any(|value| *value != 0.0));
    }

    #[test]
    fn token_count_has_minimum_of_one() {
        assert_eq!(token_count(""), 1);
        assert_eq!(token_count("abcd"), 1);
        assert_eq!(token_count("abcde"), 2);
    }
}
