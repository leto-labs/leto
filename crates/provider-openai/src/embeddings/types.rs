//! Typed request and response models for the Embeddings API.
//!
//! Official reference:
//! <https://platform.openai.com/docs/api-reference/embeddings/create>

use serde::{Deserialize, Serialize};

/// Request body for `POST /embeddings`.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct EmbeddingRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub input: EmbeddingInput,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

/// Input accepted by the embeddings API.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum EmbeddingInput {
    Text(String),
    Texts(Vec<String>),
}

impl Default for EmbeddingInput {
    fn default() -> Self {
        Self::Text(String::new())
    }
}

/// Response returned by `POST /embeddings`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EmbeddingResponse {
    pub object: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub data: Vec<EmbeddingData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<EmbeddingUsage>,
}

/// Single embedding vector item.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EmbeddingData {
    pub object: String,
    pub embedding: Vec<f32>,
    pub index: u32,
}

/// Token usage returned by the embeddings API.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EmbeddingUsage {
    pub prompt_tokens: u32,
    pub total_tokens: u32,
}
