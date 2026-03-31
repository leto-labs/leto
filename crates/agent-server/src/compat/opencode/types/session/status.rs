#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SessionSummaryDoc {
    pub additions: f64,
    pub deletions: f64,
    pub files: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diffs: Vec<FileDiffDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SessionShareDoc {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SessionTimeDoc {
    pub created: f64,
    pub updated: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compacting: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archived: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SessionRevertDoc {
    #[serde(rename = "messageID")]
    #[schemars(regex(pattern = "^msg.*"))]
    pub message_id: String,
    #[serde(default, rename = "partID", skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = "^prt.*"))]
    pub part_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "Session")]
pub struct SessionDoc {
    #[schemars(regex(pattern = "^ses.*"))]
    pub id: String,
    pub slug: String,
    #[serde(rename = "projectID")]
    pub project_id: String,
    #[serde(
        default,
        rename = "workspaceID",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(regex(pattern = "^wrk.*"))]
    pub workspace_id: Option<String>,
    pub directory: String,
    #[serde(default, rename = "parentID", skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = "^ses.*"))]
    pub parent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<SessionSummaryDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub share: Option<SessionShareDoc>,
    pub title: String,
    pub version: String,
    pub time: SessionTimeDoc,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission: Option<PermissionRuleset>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revert: Option<SessionRevertDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "SessionStatus")]
#[serde(untagged)]
pub enum SessionStatusDoc {
    Idle(SessionIdleStatusDoc),
    Retry(SessionRetryStatusDoc),
    Busy(SessionBusyStatusDoc),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SessionIdleStatusDoc {
    #[serde(rename = "type")]
    pub status_type: SessionIdleStatusKindDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum SessionIdleStatusKindDoc {
    #[serde(rename = "idle")]
    Idle,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SessionBusyStatusDoc {
    #[serde(rename = "type")]
    pub status_type: SessionBusyStatusKindDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum SessionBusyStatusKindDoc {
    #[serde(rename = "busy")]
    Busy,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SessionRetryStatusDoc {
    #[serde(rename = "type")]
    pub status_type: SessionRetryStatusKindDoc,
    pub attempt: f64,
    pub message: String,
    pub next: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum SessionRetryStatusKindDoc {
    #[serde(rename = "retry")]
    Retry,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "GlobalSession")]
pub struct GlobalSessionDoc {
    #[schemars(regex(pattern = "^ses.*"))]
    pub id: String,
    pub slug: String,
    #[serde(rename = "projectID")]
    pub project_id: String,
    #[serde(
        default,
        rename = "workspaceID",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(regex(pattern = "^wrk.*"))]
    pub workspace_id: Option<String>,
    pub directory: String,
    #[serde(default, rename = "parentID", skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = "^ses.*"))]
    pub parent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<SessionSummaryDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub share: Option<SessionShareDoc>,
    pub title: String,
    pub version: String,
    pub time: SessionTimeDoc,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission: Option<PermissionRuleset>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revert: Option<SessionRevertDoc>,
    pub project: NullableProjectSummaryDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum NullableProjectSummaryDoc {
    Project(ProjectSummaryDoc),
    Null(()),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "Todo")]
pub struct TodoDoc {
    #[schemars(description = "Brief description of the task")]
    pub content: String,
    #[schemars(
        description = "Current status of the task: pending, in_progress, completed, cancelled"
    )]
    pub status: String,
    #[schemars(description = "Priority level of the task: high, medium, low")]
    pub priority: String,
}
