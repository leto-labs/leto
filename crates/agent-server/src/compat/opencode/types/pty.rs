use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PtyIdPath {
    #[serde(rename = "ptyID")]
    #[schemars(regex(pattern = "^pty.*"))]
    pub pty_id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct PtyCreateRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Vec<String>")]
    pub args: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub cwd: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "BTreeMap<String, String>")]
    pub env: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct PtyUpdateRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "PtySizeDoc")]
    pub size: Option<PtySizeDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PtySizeDoc {
    pub rows: f64,
    pub cols: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "Pty")]
pub struct PtyDoc {
    #[schemars(regex(pattern = "^pty.*"))]
    pub id: String,
    pub title: String,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub status: PtyStatusDoc,
    pub pid: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum PtyStatusDoc {
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "exited")]
    Exited,
}
