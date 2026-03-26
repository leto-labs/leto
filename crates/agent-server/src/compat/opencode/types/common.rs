use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PathBody {
    pub path: String,
    pub body: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NamedPath {
    pub name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct CompatQuery {
    #[serde(default)]
    #[schemars(with = "String")]
    pub directory: Option<String>,
    #[serde(default)]
    #[schemars(with = "String")]
    pub workspace: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct ExperimentalSessionListQueryDoc {
    #[serde(default)]
    #[schemars(with = "String")]
    pub directory: Option<String>,
    #[serde(default)]
    #[schemars(with = "String")]
    pub workspace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roots: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archived: Option<bool>,
}
