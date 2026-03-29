//! Typed request and response models for the Vector Stores API.
//!
//! Official reference:
//! <https://platform.openai.com/docs/api-reference/vector-stores/create>

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Request body for `POST /vector_stores`.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct VectorStoreCreateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, String>,
}

/// Response returned by the vector stores API.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VectorStoreObject {
    pub id: String,
    pub object: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_counts: Option<VectorStoreFileCounts>,
}

/// File-count summary for a vector store.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VectorStoreFileCounts {
    pub in_progress: u32,
    pub completed: u32,
    pub failed: u32,
    pub cancelled: u32,
    pub total: u32,
}
