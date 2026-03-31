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

// Keep this untagged shape stable because the compat OpenAPI output is pinned.
#[expect(
    clippy::large_enum_variant,
    reason = "compat untagged schema shape must stay inline for pinned OpenAPI parity"
)]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "Message")]
#[serde(untagged)]
pub enum MessageDoc {
    User(UserMessageDoc),
    Assistant(AssistantMessageDoc),
}
