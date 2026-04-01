//! Typed request and response models for the Assistants API.
//!
//! Official reference:
//! <https://platform.openai.com/docs/api-reference/assistants>

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Request body for `POST /assistants`.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct AssistantCreateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<AssistantTool>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, String>,
}

/// Assistant tool descriptor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssistantTool {
    #[serde(rename = "type")]
    pub tool_type: String,
}

impl AssistantTool {
    pub fn file_search() -> Self {
        Self {
            tool_type: "file_search".into(),
        }
    }
}

/// Response returned by the Assistants API.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssistantObject {
    pub id: String,
    pub object: String,
    pub created_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<AssistantTool>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, String>,
}
