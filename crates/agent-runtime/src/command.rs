use provider::Message;
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::{ApprovalDecision, RuntimeConfig};

/// Stable identifier for a live runtime instance.
pub type RuntimeId = Ulid;

/// Stable identifier for one parent-child spawn relationship.
pub type SpawnId = Ulid;

/// Optional metadata describing where an inbound session command originated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionInputSource {
    /// Stable origin identifier such as a CLI session or HTTP connection id.
    pub origin: String,
    /// Optional channel label supplied by an outer multiplexer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
}

/// How an interrupt should be applied to the active session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterruptMode {
    /// Cancel active provider work immediately and pause at the next boundary.
    ImmediateCancel,
    /// Finish the active tool call before pausing.
    AfterCurrentTool,
    /// Pause at the next safe boundary.
    PauseAtBoundary,
}

/// When queued steering should be injected into the transcript.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SteerWhen {
    /// Apply steering at the next safe runtime boundary.
    NextSafeBoundary,
    /// Delay steering until any current tool phase has completed.
    AfterCurrentTool,
}

/// External control events accepted by a session engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControlEvent {
    /// Request an interruption using the supplied mode.
    Interrupt { mode: InterruptMode },
    /// Queue steering to be applied at a safe boundary.
    Steer { message: Message, when: SteerWhen },
    /// Pause execution as soon as the runtime can do so safely.
    Pause,
    /// Resume execution from a paused boundary.
    Resume,
}

/// How a child runtime should relate to the parent’s execution flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpawnMode {
    /// Parent intends to wait for the child result.
    AwaitCompletion,
    /// Parent may continue local work while the child runs.
    Concurrent,
    /// Child may outlive the parent’s current turn.
    Background,
}

/// How results should flow back from a child runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultMode {
    /// Only the final completion result is returned automatically.
    FinalResultOnly,
    /// Parent and child may exchange mailbox messages during execution.
    Mailbox,
}

/// How much parent history should be seeded into a child runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HistoryMode {
    /// Start the child with an empty transcript.
    Empty,
    /// Fork the parent transcript into the child.
    ForkParentTranscript,
}

/// When additional runtime-to-runtime input should be committed locally.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputDelivery {
    /// Apply the input at the next safe runtime boundary.
    NextSafeBoundary,
    /// Delay application until any active tool call has finished.
    AfterCurrentTool,
}

/// Behavior to apply when a wait budget expires before completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaitTimeoutAction {
    /// Release the parent/runtime hold and leave the child running.
    ReleaseHold,
    /// Interrupt the child before releasing the hold.
    InterruptChild,
}

/// Structured synchronization policy reused by spawn and explicit waits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaitRequest {
    /// Optional timeout in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    /// Behavior to apply when the timeout expires.
    pub on_timeout: WaitTimeoutAction,
}

impl Default for WaitRequest {
    fn default() -> Self {
        Self {
            timeout_ms: None,
            on_timeout: WaitTimeoutAction::ReleaseHold,
        }
    }
}

/// High-level outcome returned after waiting on one or more runtimes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaitOutcome {
    /// Every requested runtime completed successfully.
    Completed,
    /// The wait budget expired and the hold was released.
    TimedOut,
    /// At least one requested runtime failed.
    Failed,
    /// At least one requested runtime was interrupted by policy.
    Interrupted,
}

/// Outcome recorded for one runtime after a wait operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaitTargetOutcome {
    /// Runtime identifier that was observed.
    pub runtime_id: RuntimeId,
    /// Final outcome classified for this runtime.
    pub outcome: WaitOutcome,
}

/// Structured result returned by runtime-native waiting helpers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaitResult {
    /// Aggregate outcome across the requested runtime ids.
    pub outcome: WaitOutcome,
    /// Per-runtime outcome details.
    pub targets: Vec<WaitTargetOutcome>,
}

/// How broadly the runtime should expose agent listing results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentListScope {
    /// Direct child runtimes of the current runtime only.
    DirectChildren,
}

impl Default for AgentListScope {
    fn default() -> Self {
        Self::DirectChildren
    }
}

/// Semantic child-to-parent report that may later be injected into provider context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChildReport {
    /// Source child runtime identifier.
    pub child_id: RuntimeId,
    /// Optional source label for UI/model provenance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub child_label: Option<String>,
    /// Semantic report kind.
    pub kind: ChildReportKind,
    /// Structured report payload.
    pub payload: serde_json::Value,
    /// Whether the report represents a terminal/final child update.
    pub is_final: bool,
}

/// Registry-routed agent-to-agent message stored independently from transcript input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentMessage {
    /// Stable message identifier.
    pub message_id: Ulid,
    /// Sender runtime identifier.
    pub from_runtime_id: RuntimeId,
    /// Recipient runtime identifier.
    pub to_runtime_id: RuntimeId,
    /// Millisecond timestamp assigned when the message is created.
    pub sent_at: u64,
    /// Delivery semantics for the receiver.
    pub delivery_mode: AgentMessageDelivery,
    /// Semantic message kind.
    pub kind: AgentMessageKind,
    /// Optional short title or subject.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Optional tags used for filtering.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Structured message payload.
    pub payload: serde_json::Value,
    /// Optional thread identifier for follow-up messages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<Ulid>,
    /// Optional reply target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<Ulid>,
    /// Timestamp marking when the message was read/pulled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_at: Option<u64>,
}

impl AgentMessage {
    /// Creates a new registry-routed message with current metadata defaults.
    pub fn new(
        from_runtime_id: RuntimeId,
        to_runtime_id: RuntimeId,
        delivery_mode: AgentMessageDelivery,
        kind: AgentMessageKind,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            message_id: Ulid::new(),
            from_runtime_id,
            to_runtime_id,
            sent_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_millis() as u64)
                .unwrap_or_default(),
            delivery_mode,
            kind,
            title: None,
            tags: Vec::new(),
            payload,
            thread_id: None,
            reply_to: None,
            read_at: None,
        }
    }
}

/// How a registry-routed agent message should be consumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentMessageDelivery {
    /// Notify the receiver at the next safe boundary.
    Direct,
    /// Store the message for later explicit reading.
    Mail,
}

/// Semantic kind for a registry-routed agent message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentMessageKind {
    /// Generic message payload.
    Message,
    /// Clarifying question or request for input.
    Question,
    /// Observational note or intermediate finding.
    Observation,
    /// Result payload.
    Result,
    /// Progress update.
    Progress,
}

/// Query used when pulling agent messages from runtime mailbox state.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AgentMessageFilter {
    /// Optional sender filter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_runtime_id: Option<RuntimeId>,
    /// Optional delivery-mode filter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery_mode: Option<AgentMessageDelivery>,
    /// Optional semantic kind filter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<AgentMessageKind>,
    /// Optional tag filter. Any overlap qualifies as a match.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Whether only unread messages should be returned.
    #[serde(default)]
    pub unread_only: bool,
    /// Optional lower-bound timestamp filter in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since: Option<u64>,
    /// Optional maximum number of results to return.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

/// Semantic class for child reports promoted above raw runtime transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChildReportKind {
    /// Non-terminal progress or heartbeat update.
    Progress,
    /// Child asks for clarification or escalation.
    Question,
    /// Informational intermediate observation.
    Observation,
    /// Final task result.
    Result,
    /// Failure or inability to complete.
    Failure,
}

/// Request to spawn a child runtime through the registry.
#[derive(Debug, Clone, PartialEq)]
pub struct SpawnRequest {
    /// Optional human-readable label for diagnostics/UI.
    pub label: Option<String>,
    /// Initial input to commit into the child runtime.
    pub initial_input: Vec<Message>,
    /// How the child should relate to the parent’s execution flow.
    pub spawn_mode: SpawnMode,
    /// How results should flow back from the child.
    pub result_mode: ResultMode,
    /// How much parent history to inherit.
    pub history_mode: HistoryMode,
    /// Optional named runtime profile to use instead of the parent default.
    pub profile: Option<String>,
    /// Optional model override applied on top of the selected profile.
    pub model_override: Option<String>,
    /// Optional runtime-config override applied on top of the selected profile.
    pub config_override: Option<RuntimeConfig>,
    /// Optional logical loop profile override resolved by the runtime registry.
    pub loop_name: Option<String>,
    /// Optional runtime-native child reporting policy identifier.
    pub reporting: Option<String>,
}

impl Default for SpawnRequest {
    fn default() -> Self {
        Self {
            label: None,
            initial_input: Vec::new(),
            spawn_mode: SpawnMode::Background,
            result_mode: ResultMode::FinalResultOnly,
            history_mode: HistoryMode::Empty,
            profile: None,
            model_override: None,
            config_override: None,
            loop_name: None,
            reporting: None,
        }
    }
}

/// Directed runtime-to-runtime message envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Envelope {
    /// Stable envelope identifier.
    pub id: Ulid,
    /// Sender runtime id.
    pub from: RuntimeId,
    /// Recipient runtime id.
    pub to: RuntimeId,
    /// Envelope semantic kind.
    pub kind: EnvelopeKind,
    /// Structured message payload.
    pub payload: serde_json::Value,
}

impl Envelope {
    /// Creates a new directed envelope.
    pub fn new(
        from: RuntimeId,
        to: RuntimeId,
        kind: EnvelopeKind,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            id: Ulid::new(),
            from,
            to,
            kind,
            payload,
        }
    }
}

/// Semantic kind carried by a directed runtime envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvelopeKind {
    /// Additional transcript input for the recipient runtime.
    Input,
    /// Steering message to be applied at a safe boundary.
    Steer,
    /// Interrupt the recipient runtime.
    Interrupt,
    /// Pause the recipient runtime.
    Pause,
    /// Resume the recipient runtime.
    Resume,
    /// Approval request raised to another runtime.
    ApprovalRequest,
    /// Approval decision returned to another runtime.
    ApprovalDecision,
    /// Progress or heartbeat update.
    Progress,
    /// Final or partial result message.
    Result,
    /// Arbitrary mailbox message.
    Mail,
}

/// Agent-oriented commands accepted by a runtime.
#[derive(Debug, Clone, PartialEq)]
pub enum AgentCommand {
    /// Spawn a child runtime using the registry.
    SpawnAgent {
        /// Child spawn request.
        request: SpawnRequest,
        /// Optional immediate hold after spawn.
        wait: Option<WaitRequest>,
    },
    /// Queue transcript-like input for another runtime through the registry.
    SendAgentInput {
        /// Target runtime identifier.
        runtime_id: RuntimeId,
        /// Messages to deliver.
        input: Vec<Message>,
        /// Boundary policy for applying the input.
        delivery: InputDelivery,
    },
    /// Send a registry-routed message to another runtime.
    SendAgentMessage {
        /// Target runtime identifier.
        runtime_id: RuntimeId,
        /// Structured message payload and metadata.
        message: AgentMessage,
    },
    /// Interrupt another runtime through the registry.
    InterruptAgent {
        /// Target runtime identifier.
        runtime_id: RuntimeId,
        /// Interrupt mode to apply.
        mode: InterruptMode,
    },
    /// Pause another runtime through the registry.
    PauseAgent {
        /// Target runtime identifier.
        runtime_id: RuntimeId,
    },
    /// Resume another runtime through the registry.
    ResumeAgent {
        /// Target runtime identifier.
        runtime_id: RuntimeId,
    },
    /// Wait for one or more runtimes using the supplied wait policy.
    WaitForAgents {
        /// Runtime ids to observe.
        ids: Vec<RuntimeId>,
        /// Wait behavior.
        wait: WaitRequest,
    },
    /// Read mailbox/direct messages visible to the current runtime.
    ReadAgentMessages {
        /// Filter to apply while reading.
        filter: AgentMessageFilter,
    },
    /// List visible agents for the requested scope.
    ListAgents {
        /// Listing scope to apply.
        scope: AgentListScope,
    },
}

/// Command surface for the bidirectional session engine.
#[derive(Debug, Clone)]
pub enum SessionCommand {
    /// Queue transcript input for a session.
    SubmitInput {
        /// Messages to append when the runtime reaches the next input boundary.
        input: Vec<Message>,
        /// Optional source metadata preserved for outer multiplexers.
        source: Option<SessionInputSource>,
    },
    /// Submit a control event such as steering or interruption.
    Control(ControlEvent),
    /// Resolve a pending approval request.
    Approve(ApprovalDecision),
    /// Submit agent-oriented work such as spawn or messaging.
    Agent(AgentCommand),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wait_request_defaults_to_release_hold() {
        let wait = WaitRequest::default();

        assert_eq!(wait.timeout_ms, None);
        assert_eq!(wait.on_timeout, WaitTimeoutAction::ReleaseHold);
    }

    #[test]
    fn spawn_request_defaults_to_background_non_blocking() {
        let request = SpawnRequest::default();

        assert_eq!(request.label, None);
        assert!(request.initial_input.is_empty());
        assert_eq!(request.spawn_mode, SpawnMode::Background);
        assert_eq!(request.result_mode, ResultMode::FinalResultOnly);
        assert_eq!(request.history_mode, HistoryMode::Empty);
        assert_eq!(request.profile, None);
        assert_eq!(request.model_override, None);
        assert_eq!(request.config_override, None);
        assert_eq!(request.loop_name, None);
        assert_eq!(request.reporting, None);
    }

    #[test]
    fn agent_message_new_populates_runtime_metadata_defaults() {
        let from_runtime_id = Ulid::new();
        let to_runtime_id = Ulid::new();
        let payload = serde_json::json!({ "message": "hello" });

        let message = AgentMessage::new(
            from_runtime_id,
            to_runtime_id,
            AgentMessageDelivery::Mail,
            AgentMessageKind::Observation,
            payload.clone(),
        );

        assert_eq!(message.from_runtime_id, from_runtime_id);
        assert_eq!(message.to_runtime_id, to_runtime_id);
        assert_eq!(message.delivery_mode, AgentMessageDelivery::Mail);
        assert_eq!(message.kind, AgentMessageKind::Observation);
        assert_eq!(message.payload, payload);
        assert!(message.sent_at > 0);
        assert_eq!(message.title, None);
        assert!(message.tags.is_empty());
        assert_eq!(message.thread_id, None);
        assert_eq!(message.reply_to, None);
        assert_eq!(message.read_at, None);
    }
}
