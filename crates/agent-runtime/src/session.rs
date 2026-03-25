use std::collections::{BTreeMap, VecDeque};

use provider::{FinishReason, Message};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::{
    AgentMessage, ApprovalRequest, ChildReport, Envelope, InterruptMode, ResultMode, RuntimeId,
    SessionInputSource, SpawnId, SpawnMode, SteerWhen, ToolCall, WaitRequest,
};

/// Coarse execution phase for an in-memory session engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionPhase {
    /// No provider or tool work is currently running.
    Idle,
    /// The runtime is streaming provider output.
    RunningProvider,
    /// The runtime is executing a tool call.
    RunningTool,
    /// The runtime is waiting for an approval decision.
    AwaitingApproval,
    /// The runtime is waiting for one or more child runtimes.
    AwaitingChildren,
    /// The runtime is paused at a safe boundary.
    Paused,
}

/// Safe boundaries exposed by the session engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionBoundary {
    /// The runtime is idle and waiting for new user input.
    AwaitingInput,
    /// The runtime is waiting for tool approval.
    AwaitingApproval,
    /// The runtime is waiting for child work to complete.
    AwaitingChildren,
    /// The runtime is paused.
    Paused,
    /// The active turn finished.
    TurnFinished,
}

/// Lifecycle state tracked for a child runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChildStatus {
    /// Child runtime was created but not yet started work.
    Starting,
    /// Child runtime is actively doing work.
    Running,
    /// Child runtime is idle and awaiting more input.
    AwaitingInput,
    /// Child runtime is paused.
    Paused,
    /// Child runtime completed successfully.
    Completed,
    /// Child runtime failed.
    Failed,
    /// Child runtime was cancelled.
    Cancelled,
}

impl ChildStatus {
    /// Returns true when the child has reached a terminal state.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

/// Parent reference carried by a child runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParentRef {
    /// Parent runtime identifier.
    pub parent_runtime_id: RuntimeId,
    /// Spawn relationship identifier.
    pub spawn_id: SpawnId,
}

/// Result summary retained for completed child runtimes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChildResult {
    /// Final messages surfaced by the child runtime.
    pub output: Vec<Message>,
    /// Whether the child completed successfully.
    pub success: bool,
}

/// Snapshot metadata tracked for each direct child runtime.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChildRuntimeState {
    /// Child runtime identifier.
    pub runtime_id: RuntimeId,
    /// Spawn relationship identifier.
    pub spawn_id: SpawnId,
    /// Optional display label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Current child lifecycle status.
    pub status: ChildStatus,
    /// Spawn behavior chosen for this child.
    pub spawn_mode: SpawnMode,
    /// Result delivery behavior chosen for this child.
    pub result_mode: ResultMode,
    /// Most recent final result, when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_result: Option<ChildResult>,
}

/// Queued input waiting to be committed at the next input boundary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingInput {
    /// Messages to append once the runtime begins the next turn.
    pub input: Vec<Message>,
    /// Optional source metadata preserved for multiplexers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<SessionInputSource>,
}

/// Queued steering input waiting for a safe boundary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingSteering {
    /// Message to inject when the steering boundary is reached.
    pub message: Message,
    /// Injection policy for the queued steering message.
    pub when: SteerWhen,
}

/// Active wait hold currently gating runtime progress.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveWait {
    /// Runtime ids currently being observed.
    pub ids: Vec<RuntimeId>,
    /// Wait policy in effect.
    pub request: WaitRequest,
}

/// In-memory state for one reusable runtime session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionState {
    /// Stable runtime/session identifier.
    pub session_id: Ulid,
    /// Optional parent reference for child runtimes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<ParentRef>,
    /// Direct child runtime snapshots keyed by runtime id.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub children: BTreeMap<RuntimeId, ChildRuntimeState>,
    /// Mailbox/inbox for directed runtime envelopes.
    #[serde(default, skip_serializing_if = "VecDeque::is_empty")]
    pub inbox: VecDeque<Envelope>,
    /// Stored cross-agent messages, including mail and direct messages.
    #[serde(default, skip_serializing_if = "VecDeque::is_empty")]
    pub mailbox: VecDeque<AgentMessage>,
    /// Direct messages waiting to be surfaced at a safe boundary.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending_direct_messages: Vec<AgentMessage>,
    /// Semantic child reports waiting for safe-boundary transcript injection.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending_child_reports: Vec<ChildReport>,
    /// Monotonic turn counter.
    pub turn_index: u64,
    /// Provider iterations executed within the active turn.
    pub iteration_count: u32,
    /// Canonical model-visible transcript.
    pub transcript: Vec<Message>,
    /// Current runtime phase.
    pub phase: SessionPhase,
    /// Current externally visible boundary, when any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boundary: Option<SessionBoundary>,
    /// Whether a turn is currently active.
    pub active_turn: bool,
    /// Pending turn-start input batches.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending_inputs: Vec<PendingInput>,
    /// Steering queued for a safe boundary.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending_steering: Vec<PendingSteering>,
    /// Pending tool calls requested by the model.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending_tool_calls: Vec<ToolCall>,
    /// Tool call actively executing, when any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_tool_call: Option<ToolCall>,
    /// Approval request currently gating tool execution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_approval: Option<ApprovalRequest>,
    /// Active wait hold currently blocking on child/runtime completion.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_wait: Option<ActiveWait>,
    /// Interrupt requested by the caller but not yet applied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_interrupt: Option<InterruptMode>,
    /// Whether the loop may finish the current turn at the next decision point.
    pub pending_completion: bool,
    /// Most recent provider finish reason for the active turn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_finish_reason: Option<FinishReason>,
}

impl SessionState {
    /// Creates an empty in-memory session state.
    pub fn new(session_id: Ulid) -> Self {
        Self {
            session_id,
            parent: None,
            children: BTreeMap::new(),
            inbox: VecDeque::new(),
            mailbox: VecDeque::new(),
            pending_direct_messages: Vec::new(),
            pending_child_reports: Vec::new(),
            turn_index: 0,
            iteration_count: 0,
            transcript: Vec::new(),
            phase: SessionPhase::Idle,
            boundary: Some(SessionBoundary::AwaitingInput),
            active_turn: false,
            pending_inputs: Vec::new(),
            pending_steering: Vec::new(),
            pending_tool_calls: Vec::new(),
            active_tool_call: None,
            pending_approval: None,
            active_wait: None,
            pending_interrupt: None,
            pending_completion: false,
            last_finish_reason: None,
        }
    }

    /// Creates an in-memory session state seeded with an existing transcript.
    pub fn with_transcript(session_id: Ulid, transcript: Vec<Message>) -> Self {
        let mut state = Self::new(session_id);
        state.transcript = transcript;
        state
    }

    /// Returns the stable runtime identifier for this session.
    pub fn runtime_id(&self) -> RuntimeId {
        self.session_id
    }
}
