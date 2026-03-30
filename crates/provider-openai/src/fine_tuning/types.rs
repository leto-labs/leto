//! Typed response models for the Fine-Tuning API.
//!
//! Official reference:
//! <https://platform.openai.com/docs/api-reference/fine-tuning/jobs/list>

use serde::{Deserialize, Serialize};

/// Paginated response returned by `GET /fine_tuning/jobs`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FineTuningJobPage {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub data: Vec<FineTuningJob>,
    pub object: String,
    pub has_more: bool,
}

/// Fine-tuning job record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FineTuningJob {
    pub id: String,
    pub object: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fine_tuned_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
    #[serde(flatten)]
    pub raw: serde_json::Value,
}
