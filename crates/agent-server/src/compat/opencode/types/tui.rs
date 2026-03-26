use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::events::{
    EventTuiCommandExecuteDoc, EventTuiPromptAppendDoc, EventTuiSessionSelectDoc,
    EventTuiToastShowDoc, ToastVariantDoc,
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PromptAppendRequest {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExecuteCommandRequest {
    pub command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToastRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub title: Option<String>,
    pub message: String,
    pub variant: ToastVariantDoc,
    #[serde(
        default = "toast_duration_default",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with = "f64")]
    #[schemars(
        description = "Duration in milliseconds",
        default = "toast_duration_default"
    )]
    pub duration: Option<f64>,
}

fn toast_duration_default() -> Option<f64> {
    Some(5000.0)
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum PublishRequest {
    PromptAppend(EventTuiPromptAppendDoc),
    CommandExecute(EventTuiCommandExecuteDoc),
    ToastShow(EventTuiToastShowDoc),
    SessionSelect(EventTuiSessionSelectDoc),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SelectSessionRequest {
    #[serde(rename = "sessionID")]
    #[schemars(regex(pattern = "^ses.*"))]
    #[schemars(description = "Session ID to navigate to")]
    pub session_id: String,
}
