use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProjectIdPath {
    #[serde(rename = "projectID")]
    pub project_id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct ProjectUpdateRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<ProjectIconDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commands: Option<ProjectCommandsDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "Project")]
pub struct ProjectDoc {
    pub id: String,
    pub worktree: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vcs: Option<ProjectVcsDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<ProjectIconDoc>,
    #[serde(default)]
    pub commands: ProjectCommandsDoc,
    pub time: ProjectTimeDoc,
    pub sandboxes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum ProjectVcsDoc {
    #[serde(rename = "git")]
    Git,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProjectIconDoc {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, rename = "override", skip_serializing_if = "Option::is_none")]
    pub override_value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct ProjectCommandsDoc {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Startup script to run when creating a new workspace (worktree)")]
    pub start: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProjectTimeDoc {
    pub created: f64,
    pub updated: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initialized: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "ProjectSummary")]
pub struct ProjectSummaryDoc {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub worktree: String,
}
