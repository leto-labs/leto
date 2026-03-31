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
