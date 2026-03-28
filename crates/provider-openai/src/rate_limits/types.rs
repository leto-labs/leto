//! Typed response models for the Rate Limits API.
//!
//! Official reference:
//! <https://platform.openai.com/docs/api-reference/project-rate-limits>

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Paginated response returned by `GET /organization/projects/{project_id}/rate_limits`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectRateLimitPage {
    pub object: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub data: Vec<ProjectRateLimit>,
}

/// Single model-specific project rate-limit record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectRateLimit {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_requests_per_1_minute: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens_per_1_minute: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_images_per_1_minute: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_1_day_max_input_tokens: Option<u64>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}
