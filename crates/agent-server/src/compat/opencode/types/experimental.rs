use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceIdPath {
    #[schemars(regex(pattern = "^wrk.*"))]
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceCreateRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = "^wrk.*"))]
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub workspace_type: String,
    pub branch: NullableStringDoc,
    pub extra: NullableJsonValueDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum NullableStringDoc {
    String(String),
    Null(()),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum NullableJsonValueDoc {
    Value(Value),
    Null(()),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "WorktreeCreateInput")]
pub struct WorktreeCreateRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "startCommand")]
    #[schemars(description = "Additional startup script to run after the project's start command")]
    pub start_command: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "WorktreeRemoveInput")]
pub struct WorktreeRemoveRequest {
    pub directory: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "WorktreeResetInput")]
pub struct WorktreeResetRequest {
    pub directory: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "Workspace")]
pub struct WorkspaceDoc {
    #[schemars(regex(pattern = "^wrk.*"))]
    pub id: String,
    #[serde(rename = "type")]
    pub workspace_type: String,
    #[schemars(required)]
    pub branch: NullableStringDoc,
    #[schemars(required)]
    pub name: NullableStringDoc,
    #[schemars(required)]
    pub directory: NullableStringDoc,
    #[schemars(required)]
    #[schemars(with = "NullableJsonValueDoc")]
    pub extra: NullableJsonValueDoc,
    #[serde(rename = "projectID")]
    pub project_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "Worktree")]
pub struct WorktreeDoc {
    pub name: String,
    pub branch: String,
    pub directory: String,
}
