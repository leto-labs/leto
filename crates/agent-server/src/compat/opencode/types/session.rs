use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::errors::{
    ApiErrorDoc, ContextOverflowErrorDoc, MessageAbortedErrorDoc, MessageOutputLengthErrorDoc,
    ProviderAuthErrorDoc, StructuredOutputErrorDoc, UnknownErrorDoc,
};
use super::files::{
    FileDiffDoc, FilePartSourceTextDoc, FileSourceDoc, ResourceSourceDoc, SymbolSourceDoc,
};
use super::permission::PermissionRuleset;
use super::project::ProjectSummaryDoc;
use super::provider::MessageTokensCacheDoc;

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
    pub parts: Vec<CommandFilePartInputDoc>,
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
pub struct MessageListQuery {
    #[serde(default)]
    #[schemars(with = "String")]
    pub directory: Option<String>,
    #[serde(default)]
    #[schemars(with = "String")]
    pub workspace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 0, max = 9007199254740991i64))]
    pub limit: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct SessionListQuery {
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
    pub search: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<f64>,
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
pub struct SessionIdPath {
    #[serde(rename = "sessionID")]
    #[schemars(regex(pattern = "^ses.*"))]
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SessionMessagePath {
    #[serde(rename = "sessionID")]
    #[schemars(regex(pattern = "^ses.*"))]
    pub session_id: String,
    #[serde(rename = "messageID")]
    #[schemars(regex(pattern = "^msg.*"))]
    pub message_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SessionMessagePartPath {
    #[serde(rename = "sessionID")]
    #[schemars(regex(pattern = "^ses.*"))]
    pub session_id: String,
    #[serde(rename = "messageID")]
    #[schemars(regex(pattern = "^msg.*"))]
    pub message_id: String,
    #[serde(rename = "partID")]
    #[schemars(regex(pattern = "^prt.*"))]
    pub part_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SessionPermissionPath {
    #[serde(rename = "sessionID")]
    #[schemars(regex(pattern = "^ses.*"))]
    pub session_id: String,
    #[serde(rename = "permissionID")]
    #[schemars(regex(pattern = "^per.*"))]
    pub permission_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct SessionDiffQuery {
    #[serde(default)]
    #[schemars(with = "String")]
    pub directory: Option<String>,
    #[serde(default)]
    #[schemars(with = "String")]
    pub workspace: Option<String>,
    #[serde(default, rename = "messageID", skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = "^msg.*"))]
    pub message_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "MessageWithParts")]
pub struct MessageWithPartsDoc {
    pub info: MessageDoc,
    pub parts: Vec<PartDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "AssistantMessageWithParts")]
pub struct AssistantMessageWithPartsDoc {
    pub info: AssistantMessageDoc,
    pub parts: Vec<PartDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum PermissionResponseKindDoc {
    #[serde(rename = "once")]
    Once,
    #[serde(rename = "always")]
    Always,
    #[serde(rename = "reject")]
    Reject,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "JSONSchema")]
pub struct JsonSchemaDoc(
    #[schemars(with = "std::collections::BTreeMap<String, Value>")]
    pub  std::collections::BTreeMap<String, Value>,
);

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "OutputFormatText")]
pub struct OutputFormatTextDoc {
    #[serde(rename = "type")]
    pub format_type: OutputFormatTextKindDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum OutputFormatTextKindDoc {
    #[serde(rename = "text")]
    Text,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "OutputFormatJsonSchema")]
pub struct OutputFormatJsonSchemaDoc {
    #[serde(rename = "type")]
    pub format_type: OutputFormatJsonSchemaKindDoc,
    pub schema: JsonSchemaDoc,
    #[serde(default = "default_retry_count", rename = "retryCount")]
    #[schemars(range(min = 0, max = 9007199254740991i64))]
    pub retry_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum OutputFormatJsonSchemaKindDoc {
    #[serde(rename = "json_schema")]
    JsonSchema,
}

const fn default_retry_count() -> u64 {
    2
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "OutputFormat")]
#[serde(untagged)]
pub enum OutputFormatDoc {
    Text(OutputFormatTextDoc),
    JsonSchema(OutputFormatJsonSchemaDoc),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MessagePathDoc {
    pub cwd: String,
    pub root: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MessageSummaryDoc {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    pub diffs: Vec<FileDiffDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MessageModelDoc {
    #[serde(rename = "providerID")]
    pub provider_id: String,
    #[serde(rename = "modelID")]
    pub model_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MessageTimeCreatedDoc {
    pub created: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MessageTimeCreatedCompletedDoc {
    pub created: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MessageTokensDoc {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<f64>,
    pub input: f64,
    pub output: f64,
    pub reasoning: f64,
    pub cache: MessageTokensCacheDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum CompatMessageErrorDoc {
    ProviderAuth(ProviderAuthErrorDoc),
    Unknown(UnknownErrorDoc),
    OutputLength(MessageOutputLengthErrorDoc),
    Aborted(MessageAbortedErrorDoc),
    Structured(StructuredOutputErrorDoc),
    ContextOverflow(ContextOverflowErrorDoc),
    Api(ApiErrorDoc),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "UserMessage")]
pub struct UserMessageDoc {
    #[schemars(regex(pattern = "^msg.*"))]
    pub id: String,
    #[serde(rename = "sessionID")]
    #[schemars(regex(pattern = "^ses.*"))]
    pub session_id: String,
    pub role: UserMessageRoleDoc,
    pub time: MessageTimeCreatedDoc,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<OutputFormatDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<MessageSummaryDoc>,
    pub agent: String,
    pub model: MessageModelDoc,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "std::collections::BTreeMap<String, bool>")]
    pub tools: Option<std::collections::BTreeMap<String, bool>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum UserMessageRoleDoc {
    #[serde(rename = "user")]
    User,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "AssistantMessage")]
pub struct AssistantMessageDoc {
    #[schemars(regex(pattern = "^msg.*"))]
    pub id: String,
    #[serde(rename = "sessionID")]
    #[schemars(regex(pattern = "^ses.*"))]
    pub session_id: String,
    pub role: AssistantMessageRoleDoc,
    pub time: MessageTimeCreatedCompletedDoc,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<CompatMessageErrorDoc>,
    #[serde(rename = "parentID")]
    #[schemars(regex(pattern = "^msg.*"))]
    pub parent_id: String,
    #[serde(rename = "modelID")]
    pub model_id: String,
    #[serde(rename = "providerID")]
    pub provider_id: String,
    pub mode: String,
    pub agent: String,
    pub path: MessagePathDoc,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<bool>,
    pub cost: f64,
    pub tokens: MessageTokensDoc,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub structured: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finish: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum AssistantMessageRoleDoc {
    #[serde(rename = "assistant")]
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "Message")]
#[serde(untagged)]
pub enum MessageDoc {
    User(UserMessageDoc),
    Assistant(AssistantMessageDoc),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "FilePartSource")]
#[serde(untagged)]
pub enum FilePartSourceDoc {
    File(FileSourceDoc),
    Symbol(SymbolSourceDoc),
    Resource(ResourceSourceDoc),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PartTimeDoc {
    pub start: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "TextPart")]
pub struct TextPartDoc {
    #[schemars(regex(pattern = "^prt.*"))]
    pub id: String,
    #[serde(rename = "sessionID")]
    #[schemars(regex(pattern = "^ses.*"))]
    pub session_id: String,
    #[serde(rename = "messageID")]
    #[schemars(regex(pattern = "^msg.*"))]
    pub message_id: String,
    #[serde(rename = "type")]
    pub part_type: TextPartKindDoc,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub synthetic: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ignored: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<PartTimeDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "std::collections::BTreeMap<String, Value>")]
    pub metadata: Option<std::collections::BTreeMap<String, Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum TextPartKindDoc {
    #[serde(rename = "text")]
    Text,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "TextPartInput")]
pub struct TextPartInputDoc {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = "^prt.*"))]
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub part_type: TextPartKindDoc,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub synthetic: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ignored: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<PartTimeDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "std::collections::BTreeMap<String, Value>")]
    pub metadata: Option<std::collections::BTreeMap<String, Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "FilePart")]
pub struct FilePartDoc {
    #[schemars(regex(pattern = "^prt.*"))]
    pub id: String,
    #[serde(rename = "sessionID")]
    #[schemars(regex(pattern = "^ses.*"))]
    pub session_id: String,
    #[serde(rename = "messageID")]
    #[schemars(regex(pattern = "^msg.*"))]
    pub message_id: String,
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
pub enum FilePartKindDoc {
    #[serde(rename = "file")]
    File,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "FilePartInput")]
pub struct FilePartInputDoc {
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
#[schemars(rename = "AgentPart")]
pub struct AgentPartDoc {
    #[schemars(regex(pattern = "^prt.*"))]
    pub id: String,
    #[serde(rename = "sessionID")]
    #[schemars(regex(pattern = "^ses.*"))]
    pub session_id: String,
    #[serde(rename = "messageID")]
    #[schemars(regex(pattern = "^msg.*"))]
    pub message_id: String,
    #[serde(rename = "type")]
    pub part_type: AgentPartKindDoc,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<FilePartSourceTextDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum AgentPartKindDoc {
    #[serde(rename = "agent")]
    Agent,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "AgentPartInput")]
pub struct AgentPartInputDoc {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = "^prt.*"))]
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub part_type: AgentPartKindDoc,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<FilePartSourceTextDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "SubtaskPart")]
pub struct SubtaskPartDoc {
    #[schemars(regex(pattern = "^prt.*"))]
    pub id: String,
    #[serde(rename = "sessionID")]
    #[schemars(regex(pattern = "^ses.*"))]
    pub session_id: String,
    #[serde(rename = "messageID")]
    #[schemars(regex(pattern = "^msg.*"))]
    pub message_id: String,
    #[serde(rename = "type")]
    pub part_type: SubtaskPartKindDoc,
    pub prompt: String,
    pub description: String,
    pub agent: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<MessageModelDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum SubtaskPartKindDoc {
    #[serde(rename = "subtask")]
    Subtask,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "SubtaskPartInput")]
pub struct SubtaskPartInputDoc {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = "^prt.*"))]
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub part_type: SubtaskPartKindDoc,
    pub prompt: String,
    pub description: String,
    pub agent: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<MessageModelDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "ReasoningPart")]
pub struct ReasoningPartDoc {
    #[schemars(regex(pattern = "^prt.*"))]
    pub id: String,
    #[serde(rename = "sessionID")]
    #[schemars(regex(pattern = "^ses.*"))]
    pub session_id: String,
    #[serde(rename = "messageID")]
    #[schemars(regex(pattern = "^msg.*"))]
    pub message_id: String,
    #[serde(rename = "type")]
    pub part_type: ReasoningPartKindDoc,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<std::collections::BTreeMap<String, Value>>")]
    pub metadata: Option<std::collections::BTreeMap<String, Value>>,
    pub time: PartTimeDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum ReasoningPartKindDoc {
    #[serde(rename = "reasoning")]
    Reasoning,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "StepStartPart")]
pub struct StepStartPartDoc {
    #[schemars(regex(pattern = "^prt.*"))]
    pub id: String,
    #[serde(rename = "sessionID")]
    #[schemars(regex(pattern = "^ses.*"))]
    pub session_id: String,
    #[serde(rename = "messageID")]
    #[schemars(regex(pattern = "^msg.*"))]
    pub message_id: String,
    #[serde(rename = "type")]
    pub part_type: StepStartPartKindDoc,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum StepStartPartKindDoc {
    #[serde(rename = "step-start")]
    StepStart,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StepFinishTokensDoc {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<f64>,
    pub input: f64,
    pub output: f64,
    pub reasoning: f64,
    pub cache: MessageTokensCacheDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "StepFinishPart")]
pub struct StepFinishPartDoc {
    #[schemars(regex(pattern = "^prt.*"))]
    pub id: String,
    #[serde(rename = "sessionID")]
    #[schemars(regex(pattern = "^ses.*"))]
    pub session_id: String,
    #[serde(rename = "messageID")]
    #[schemars(regex(pattern = "^msg.*"))]
    pub message_id: String,
    #[serde(rename = "type")]
    pub part_type: StepFinishPartKindDoc,
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<String>,
    pub cost: f64,
    pub tokens: StepFinishTokensDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum StepFinishPartKindDoc {
    #[serde(rename = "step-finish")]
    StepFinish,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "SnapshotPart")]
pub struct SnapshotPartDoc {
    #[schemars(regex(pattern = "^prt.*"))]
    pub id: String,
    #[serde(rename = "sessionID")]
    #[schemars(regex(pattern = "^ses.*"))]
    pub session_id: String,
    #[serde(rename = "messageID")]
    #[schemars(regex(pattern = "^msg.*"))]
    pub message_id: String,
    #[serde(rename = "type")]
    pub part_type: SnapshotPartKindDoc,
    pub snapshot: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum SnapshotPartKindDoc {
    #[serde(rename = "snapshot")]
    Snapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "PatchPart")]
pub struct PatchPartDoc {
    #[schemars(regex(pattern = "^prt.*"))]
    pub id: String,
    #[serde(rename = "sessionID")]
    #[schemars(regex(pattern = "^ses.*"))]
    pub session_id: String,
    #[serde(rename = "messageID")]
    #[schemars(regex(pattern = "^msg.*"))]
    pub message_id: String,
    #[serde(rename = "type")]
    pub part_type: PatchPartKindDoc,
    pub hash: String,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum PatchPartKindDoc {
    #[serde(rename = "patch")]
    Patch,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "RetryPart")]
pub struct RetryPartDoc {
    #[schemars(regex(pattern = "^prt.*"))]
    pub id: String,
    #[serde(rename = "sessionID")]
    #[schemars(regex(pattern = "^ses.*"))]
    pub session_id: String,
    #[serde(rename = "messageID")]
    #[schemars(regex(pattern = "^msg.*"))]
    pub message_id: String,
    #[serde(rename = "type")]
    pub part_type: RetryPartKindDoc,
    pub attempt: f64,
    pub error: ApiErrorDoc,
    pub time: MessageTimeCreatedDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum RetryPartKindDoc {
    #[serde(rename = "retry")]
    Retry,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "CompactionPart")]
pub struct CompactionPartDoc {
    #[schemars(regex(pattern = "^prt.*"))]
    pub id: String,
    #[serde(rename = "sessionID")]
    #[schemars(regex(pattern = "^ses.*"))]
    pub session_id: String,
    #[serde(rename = "messageID")]
    #[schemars(regex(pattern = "^msg.*"))]
    pub message_id: String,
    #[serde(rename = "type")]
    pub part_type: CompactionPartKindDoc,
    pub auto: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overflow: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum CompactionPartKindDoc {
    #[serde(rename = "compaction")]
    Compaction,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "ToolStatePending")]
pub struct ToolStatePendingDoc {
    pub status: ToolStatePendingKindDoc,
    #[schemars(with = "std::collections::BTreeMap<String, Value>")]
    pub input: std::collections::BTreeMap<String, Value>,
    pub raw: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum ToolStatePendingKindDoc {
    #[serde(rename = "pending")]
    Pending,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolStateTimeStartDoc {
    pub start: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolStateTimeRangeDoc {
    pub start: f64,
    pub end: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolStateCompletedTimeRangeDoc {
    pub start: f64,
    pub end: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compacted: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "ToolStateRunning")]
pub struct ToolStateRunningDoc {
    pub status: ToolStateRunningKindDoc,
    #[schemars(with = "std::collections::BTreeMap<String, Value>")]
    pub input: std::collections::BTreeMap<String, Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "std::collections::BTreeMap<String, Value>")]
    pub metadata: Option<std::collections::BTreeMap<String, Value>>,
    pub time: ToolStateTimeStartDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum ToolStateRunningKindDoc {
    #[serde(rename = "running")]
    Running,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "ToolStateCompleted")]
pub struct ToolStateCompletedDoc {
    pub status: ToolStateCompletedKindDoc,
    #[schemars(with = "std::collections::BTreeMap<String, Value>")]
    pub input: std::collections::BTreeMap<String, Value>,
    pub output: String,
    pub title: String,
    #[schemars(with = "std::collections::BTreeMap<String, Value>")]
    pub metadata: std::collections::BTreeMap<String, Value>,
    pub time: ToolStateCompletedTimeRangeDoc,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<FilePartDoc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum ToolStateCompletedKindDoc {
    #[serde(rename = "completed")]
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "ToolStateError")]
pub struct ToolStateErrorDoc {
    pub status: ToolStateErrorKindDoc,
    #[schemars(with = "std::collections::BTreeMap<String, Value>")]
    pub input: std::collections::BTreeMap<String, Value>,
    pub error: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "std::collections::BTreeMap<String, Value>")]
    pub metadata: Option<std::collections::BTreeMap<String, Value>>,
    pub time: ToolStateTimeRangeDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum ToolStateErrorKindDoc {
    #[serde(rename = "error")]
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "ToolState")]
#[serde(untagged)]
pub enum ToolStateDoc {
    Pending(ToolStatePendingDoc),
    Running(ToolStateRunningDoc),
    Completed(ToolStateCompletedDoc),
    Error(ToolStateErrorDoc),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "ToolPart")]
pub struct ToolPartDoc {
    #[schemars(regex(pattern = "^prt.*"))]
    pub id: String,
    #[serde(rename = "sessionID")]
    #[schemars(regex(pattern = "^ses.*"))]
    pub session_id: String,
    #[serde(rename = "messageID")]
    #[schemars(regex(pattern = "^msg.*"))]
    pub message_id: String,
    #[serde(rename = "type")]
    pub part_type: ToolPartKindDoc,
    #[serde(rename = "callID")]
    pub call_id: String,
    pub tool: String,
    pub state: ToolStateDoc,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "std::collections::BTreeMap<String, Value>")]
    pub metadata: Option<std::collections::BTreeMap<String, Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum ToolPartKindDoc {
    #[serde(rename = "tool")]
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "Part")]
#[serde(untagged)]
pub enum PartDoc {
    Text(TextPartDoc),
    Subtask(SubtaskPartDoc),
    Reasoning(ReasoningPartDoc),
    File(FilePartDoc),
    Tool(ToolPartDoc),
    StepStart(StepStartPartDoc),
    StepFinish(StepFinishPartDoc),
    Snapshot(SnapshotPartDoc),
    Patch(PatchPartDoc),
    Agent(AgentPartDoc),
    Retry(RetryPartDoc),
    Compaction(CompactionPartDoc),
}

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
