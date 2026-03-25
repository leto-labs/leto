use provider::{FinishReason, Message};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::{ApprovalDecision, RuntimeConfig};

/// Stable identifier for a live runtime instance.
pub type RuntimeId = Ulid;

/// Stable identifier for one parent-child spawn relationship.
pub type SpawnId = Ulid;

/// Stable identifier for one managed PTY session.
pub type PtyId = Ulid;

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

/// Pure provider subcall requested by a loop strategy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubcallRequest {
    /// Stable purpose label used by loops to correlate results across ticks.
    pub purpose: String,
    /// Provider messages for the isolated subcall.
    pub messages: Vec<Message>,
    /// Optional model override applied only to the subcall.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_override: Option<String>,
}

/// Result captured from a loop-authored provider subcall.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubcallResult {
    /// Stable purpose label echoed from the originating request.
    pub purpose: String,
    /// Final assistant message, when the subcall produced one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<Message>,
    /// Provider finish reason captured for the subcall.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<FinishReason>,
    /// Optional error string when the subcall failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Concrete transcript replacement requested by a loop strategy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranscriptRewrite {
    /// Stable purpose label used for diagnostics and loop coordination.
    pub purpose: String,
    /// Full transcript that should replace the current transcript.
    pub messages: Vec<Message>,
}

/// Outcome recorded after a loop-authored transcript rewrite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranscriptRewriteResult {
    /// Stable purpose label echoed from the originating rewrite request.
    pub purpose: String,
    /// Number of messages before the rewrite.
    pub previous_message_count: usize,
    /// Number of messages after the rewrite.
    pub new_message_count: usize,
}

/// Concrete transcript append requested by a loop strategy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranscriptAppend {
    /// Stable purpose label used for diagnostics and loop coordination.
    pub purpose: String,
    /// Messages to append to the current transcript.
    pub messages: Vec<Message>,
}

/// Outcome recorded after a loop-authored transcript append.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranscriptAppendResult {
    /// Stable purpose label echoed from the originating append request.
    pub purpose: String,
    /// Number of messages appended.
    pub appended_messages: usize,
    /// Final transcript length after the append.
    pub transcript_message_count: usize,
}

/// Recent runtime-native operation outcome exposed back to loop strategies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RuntimeOperationResult {
    /// Result of a pure provider subcall.
    Subcall { result: SubcallResult },
    /// Result of a transcript replacement.
    TranscriptRewrite { result: TranscriptRewriteResult },
    /// Result of a transcript append.
    TranscriptAppend { result: TranscriptAppendResult },
    /// Result of a PTY command/input batch.
    PtyExecution { result: PtyExecResult },
    /// Result of a PTY capture request.
    PtyCapture { result: PtyCaptureResult },
}

/// Lifecycle status tracked for one runtime-managed PTY session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PtyStatus {
    /// PTY was created and is still starting up.
    Starting,
    /// PTY is alive and currently idle.
    Idle,
    /// PTY recently received input and is considered foreground-active.
    Running,
    /// PTY is still alive but explicitly backgrounded.
    Backgrounded,
    /// PTY terminated cleanly.
    Closed,
    /// PTY failed unexpectedly.
    Failed,
}

/// Structured snapshot of a runtime-managed PTY session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PtySessionState {
    /// Stable PTY identifier.
    pub pty_id: PtyId,
    /// Optional human-readable label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Runtime that originally opened the PTY.
    pub owner_runtime_id: RuntimeId,
    /// Working directory used when the PTY was opened.
    pub cwd: String,
    /// Current row count.
    pub rows: u16,
    /// Current column count.
    pub cols: u16,
    /// Current PTY lifecycle status.
    pub status: PtyStatus,
    /// Monotonic cursor representing buffered PTY output progress.
    pub output_cursor: u64,
    /// Most recent child shell/process exit code when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
}

/// Snapshot captured from the current PTY screen and output buffer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PtySnapshot {
    /// PTY this snapshot came from.
    pub pty_id: PtyId,
    /// Visible screen contents rendered from the virtual terminal.
    pub visible_screen: String,
    /// Incremental output since a previously observed cursor, when requested.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub incremental_output: Option<String>,
    /// Cursor value callers may reuse for future incremental reads.
    pub output_cursor: u64,
}

/// Structured result of a PTY execution request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PtyExecResult {
    /// PTY this execution ran against.
    pub pty_id: PtyId,
    /// Commands or input chunks that were executed.
    pub steps: Vec<String>,
    /// Whether execution was intentionally backgrounded.
    pub backgrounded: bool,
    /// Whether the request completed without interruption.
    pub completed: bool,
    /// Whether the request was interrupted.
    pub interrupted: bool,
    /// Whether the PTY child process has exited.
    pub shell_exited: bool,
    /// Exit code when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// Snapshot captured after execution.
    pub snapshot: PtySnapshot,
}

/// Structured result of a PTY capture request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PtyCaptureResult {
    /// PTY this capture came from.
    pub pty_id: PtyId,
    /// Snapshot captured from the PTY.
    pub snapshot: PtySnapshot,
}

/// Semantic kind for PTY-originated events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PtyEventKind {
    /// Output bytes became available.
    Output,
    /// A PTY execution request started.
    ExecutionStarted,
    /// A PTY execution request completed.
    ExecutionCompleted,
    /// PTY status changed.
    StatusChanged,
    /// PTY dimensions changed.
    Resized,
    /// PTY received an interrupt.
    Interrupted,
    /// PTY closed.
    Closed,
    /// PTY failed.
    Failed,
}

/// Event emitted by a runtime-managed PTY session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PtyEvent {
    /// PTY identifier that emitted the event.
    pub pty_id: PtyId,
    /// Monotonic per-PTY sequence number.
    pub sequence: u64,
    /// Semantic event kind.
    pub kind: PtyEventKind,
    /// Runtime timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// Structured event payload.
    pub payload: serde_json::Value,
}

/// Delivery policy for PTY event subscriptions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PtySubscriptionDelivery {
    /// Keep events observable in runtime state without transcript promotion.
    StreamOnly,
    /// Queue events for safe-boundary transcript promotion as developer messages.
    PromoteToDeveloper,
}

/// PTY event subscription owned by a runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PtySubscription {
    /// Target PTY identifier.
    pub pty_id: PtyId,
    /// Optional event-kind filter.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kinds: Vec<PtyEventKind>,
    /// Delivery behavior applied to matching events.
    pub delivery: PtySubscriptionDelivery,
}

/// Query used while reading PTY events visible to the current runtime.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PtyEventFilter {
    /// Optional PTY identifier filter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pty_id: Option<PtyId>,
    /// Optional event-kind filter.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kinds: Vec<PtyEventKind>,
    /// Optional lower-bound sequence filter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since_sequence: Option<u64>,
    /// Optional maximum number of events to return.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

/// How a PTY capture should read output from the buffered session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PtyCaptureMode {
    /// Return the visible screen only.
    VisibleScreen,
    /// Return the visible screen and incremental output since a cursor.
    Incremental,
}

/// Request to open a new runtime-managed PTY session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenPtyRequest {
    /// Optional human-readable label for diagnostics/UI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Optional working directory for the PTY shell.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    /// Initial row count.
    pub rows: u16,
    /// Initial column count.
    pub cols: u16,
}

impl Default for OpenPtyRequest {
    fn default() -> Self {
        Self {
            label: None,
            cwd: None,
            rows: 24,
            cols: 80,
        }
    }
}

/// Request to execute commands inside an existing PTY.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PtyExecRequest {
    /// Target PTY identifier.
    pub pty_id: PtyId,
    /// Commands or input chunks to write.
    pub steps: Vec<String>,
    /// Minimum wait after sending input before capturing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wait_ms: Option<u64>,
    /// Whether the PTY should remain backgrounded after the write.
    #[serde(default)]
    pub background: bool,
}

/// Request to capture the current PTY state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PtyCaptureRequest {
    /// Target PTY identifier.
    pub pty_id: PtyId,
    /// Capture mode.
    pub mode: PtyCaptureMode,
    /// Optional output cursor used for incremental capture.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since_cursor: Option<u64>,
}

/// PTY-oriented commands accepted by a runtime.
#[derive(Debug, Clone, PartialEq)]
pub enum PtyCommand {
    /// Open a new runtime-managed PTY session.
    OpenPty {
        /// PTY creation request.
        request: OpenPtyRequest,
    },
    /// List PTY sessions currently known to the runtime registry.
    ListPtys,
    /// Fetch a single PTY snapshot by id.
    GetPty {
        /// PTY identifier.
        pty_id: PtyId,
    },
    /// Write raw input into an existing PTY.
    WritePtyInput {
        /// PTY identifier.
        pty_id: PtyId,
        /// Raw input to write.
        input: String,
        /// Optional wait before capturing the PTY again.
        wait_ms: Option<u64>,
    },
    /// Execute a command/input batch inside an existing PTY.
    ExecutePtyBatch {
        /// Structured execution request.
        request: PtyExecRequest,
    },
    /// Capture PTY state and optionally incremental output.
    CapturePty {
        /// Structured capture request.
        request: PtyCaptureRequest,
    },
    /// Resize an existing PTY.
    ResizePty {
        /// PTY identifier.
        pty_id: PtyId,
        /// New row count.
        rows: u16,
        /// New column count.
        cols: u16,
    },
    /// Interrupt an existing PTY by sending ctrl-c.
    InterruptPty {
        /// PTY identifier.
        pty_id: PtyId,
    },
    /// Mark an existing PTY as backgrounded.
    BackgroundPty {
        /// PTY identifier.
        pty_id: PtyId,
    },
    /// Close an existing PTY.
    ClosePty {
        /// PTY identifier.
        pty_id: PtyId,
    },
    /// Subscribe the current runtime to PTY events.
    SubscribePty {
        /// PTY event subscription.
        subscription: PtySubscription,
    },
    /// Remove a PTY event subscription from the current runtime.
    UnsubscribePty {
        /// PTY identifier.
        pty_id: PtyId,
    },
    /// Read PTY events delivered to the current runtime.
    ReadPtyEvents {
        /// Filter applied to visible PTY events.
        filter: PtyEventFilter,
    },
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
    /// Submit PTY-oriented work such as opening, executing, or subscribing.
    Pty(PtyCommand),
    /// Shut down the in-memory runtime engine and stop processing commands.
    Shutdown,
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

    #[test]
    fn subcall_request_preserves_purpose_and_messages() {
        let request = SubcallRequest {
            purpose: "handoff_summary".into(),
            messages: vec![Message::user_text("summarize this")],
            model_override: Some("summary-model".into()),
        };

        assert_eq!(request.purpose, "handoff_summary");
        assert_eq!(request.messages.len(), 1);
        assert_eq!(request.model_override.as_deref(), Some("summary-model"));
    }

    #[test]
    fn open_pty_request_defaults_match_runtime_shell_expectations() {
        let request = OpenPtyRequest::default();

        assert_eq!(request.label, None);
        assert_eq!(request.cwd, None);
        assert_eq!(request.rows, 24);
        assert_eq!(request.cols, 80);
    }
}
