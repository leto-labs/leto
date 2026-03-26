use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::permission::ToolRequestDoc;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct QuestionRequestIdPath {
    #[serde(rename = "requestID")]
    #[schemars(regex(pattern = "^que.*"))]
    pub request_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "QuestionRequest")]
pub struct QuestionRequestDoc {
    #[schemars(regex(pattern = "^que.*"))]
    pub id: String,
    #[serde(rename = "sessionID")]
    #[schemars(regex(pattern = "^ses.*"))]
    pub session_id: String,
    #[schemars(description = "Questions to ask")]
    pub questions: Vec<QuestionInfoDoc>,
    #[schemars(with = "ToolRequestDoc")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool: Option<ToolRequestDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "QuestionInfo")]
pub struct QuestionInfoDoc {
    #[serde(rename = "question")]
    #[schemars(description = "Complete question")]
    pub question: String,
    #[schemars(description = "Very short label (max 30 chars)")]
    pub header: String,
    #[schemars(description = "Available choices")]
    pub options: Vec<QuestionOptionDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    #[schemars(description = "Allow selecting multiple choices")]
    pub multiple: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    #[schemars(description = "Allow typing a custom answer (default: true)")]
    pub custom: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "QuestionOption")]
pub struct QuestionOptionDoc {
    #[schemars(description = "Display text (1-5 words, concise)")]
    pub label: String,
    #[schemars(description = "Explanation of choice")]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "QuestionAnswer")]
#[schemars(transparent)]
#[serde(transparent)]
pub struct QuestionAnswerDoc(pub Vec<String>);

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(inline)]
pub struct QuestionReplyRequest {
    #[schemars(
        description = "User answers in order of questions (each answer is an array of selected labels)"
    )]
    pub answers: Vec<QuestionAnswerDoc>,
}
