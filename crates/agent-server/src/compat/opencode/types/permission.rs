use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PermissionRequestIdPath {
    #[serde(rename = "requestID")]
    #[schemars(regex(pattern = "^per.*"))]
    pub request_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "PermissionAction")]
pub enum PermissionActionDoc {
    #[serde(rename = "allow")]
    Allow,
    #[serde(rename = "deny")]
    Deny,
    #[serde(rename = "ask")]
    Ask,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "PermissionRule")]
pub struct PermissionRuleDoc {
    pub permission: String,
    pub pattern: String,
    pub action: PermissionActionDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(transparent)]
#[serde(transparent)]
pub struct PermissionRuleset(pub Vec<PermissionRuleDoc>);

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "PermissionRequest")]
pub struct PermissionRequestDoc {
    #[schemars(regex(pattern = "^per.*"))]
    pub id: String,
    #[serde(rename = "sessionID")]
    #[schemars(regex(pattern = "^ses.*"))]
    pub session_id: String,
    pub permission: String,
    pub patterns: Vec<String>,
    #[schemars(with = "std::collections::BTreeMap<String, Value>")]
    pub metadata: std::collections::BTreeMap<String, Value>,
    pub always: Vec<String>,
    #[schemars(with = "ToolRequestDoc")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool: Option<ToolRequestDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolRequestDoc {
    #[serde(rename = "messageID")]
    #[schemars(regex(pattern = "^msg.*"))]
    pub message_id: String,
    #[serde(rename = "callID")]
    pub call_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PermissionReplyRequest {
    pub reply: PermissionReplyValueDoc,
    #[schemars(with = "String")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum PermissionReplyValueDoc {
    #[serde(rename = "once")]
    Once,
    #[serde(rename = "always")]
    Always,
    #[serde(rename = "reject")]
    Reject,
}
