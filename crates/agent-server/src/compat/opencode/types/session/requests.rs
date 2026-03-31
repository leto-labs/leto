#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct PromptRequest {
    #[serde(default, rename = "messageID", skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = "^msg.*"))]
    pub message_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<MessageModelDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(default, rename = "noReply", skip_serializing_if = "Option::is_none")]
    pub no_reply: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "@deprecated tools and permissions have been merged, you can set permissions on the session itself now"
    )]
    #[schemars(with = "Option<std::collections::BTreeMap<String, bool>>")]
    pub tools: Option<std::collections::BTreeMap<String, bool>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<OutputFormatDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    pub parts: Vec<PromptPartInputDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct CommandRequest {
    #[serde(default, rename = "messageID", skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = "^msg.*"))]
    pub message_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub arguments: String,
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<CommandPartInputDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct ShellRequest {
    pub agent: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<MessageModelDoc>,
    pub command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct SessionCreateRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, rename = "parentID", skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = "^ses.*"))]
    pub parent_id: Option<String>,
    #[serde(
        default,
        rename = "workspaceID",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(regex(pattern = "^wrk.*"))]
    pub workspace_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission: Option<PermissionRuleset>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct SessionUpdateRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<SessionUpdateTimeDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct SessionUpdateTimeDoc {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archived: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct SessionInitRequest {
    #[serde(rename = "modelID")]
    pub model_id: String,
    #[serde(rename = "providerID")]
    pub provider_id: String,
    #[serde(rename = "messageID")]
    #[schemars(regex(pattern = "^msg.*"))]
    pub message_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct SessionForkRequest {
    #[serde(default, rename = "messageID", skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = "^msg.*"))]
    pub message_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct SessionSummarizeRequest {
    #[serde(rename = "providerID")]
    pub provider_id: String,
    #[serde(rename = "modelID")]
    pub model_id: String,
    #[serde(default)]
    pub auto: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct RevertRequest {
    #[serde(rename = "messageID")]
    #[schemars(regex(pattern = "^msg.*"))]
    pub message_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "partID")]
    #[schemars(regex(pattern = "^prt.*"))]
    pub part_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PermissionReplyRequest {
    pub response: PermissionResponseKindDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum PromptPartInputDoc {
    Text(TextPartInputDoc),
    File(FilePartInputDoc),
    Agent(AgentPartInputDoc),
    Subtask(SubtaskPartInputDoc),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CommandFilePartInputDoc {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = "^prt.*"))]
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub part_type: FilePartKindDoc,
    pub mime: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<FilePartSourceDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum CommandPartInputDoc {
    File(CommandFilePartInputDoc),
}
