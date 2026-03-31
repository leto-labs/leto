use atif::{Agent as AtifAgent, FinalMetrics as AtifFinalMetrics, Step as AtifStep, Trajectory};
use provider::{Block, BlockDelta, FinishReason, Message, Usage};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::{
    AgentMessage, ApprovalDecision, ApprovalRequest, ChildReport, ChildResult, ChildRuntimeState,
    ChildStatus, ControlEvent, DeliveredPtyEvent, Envelope, InputDelivery, InterruptMode,
    PtyCaptureResult, PtyEvent, PtyId, PtySessionState, PtySubscription, RuntimeId,
    SessionBoundary, SessionInputSource, SessionPhase, SteerWhen, SubcallResult, ToolCall,
    ToolExecutionResult, TranscriptAppendResult, TranscriptRewriteResult, WaitRequest, WaitResult,
    WorktreeId, WorktreeState,
};

/// Runtime-level events emitted by the bidirectional session engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RuntimeEvent {
    /// A new input batch was accepted by the session.
    InputQueued {
        /// Number of messages in the queued batch.
        message_count: usize,
        /// Optional source metadata preserved for outer multiplexers.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source: Option<SessionInputSource>,
    },
    /// A control event was accepted by the session.
    ControlQueued {
        /// Control payload.
        control: ControlEvent,
    },
    /// Agent-oriented work was accepted by the session.
    AgentQueued,
    /// A new turn began.
    TurnStarted {
        /// Stable session identifier.
        session_id: Ulid,
        /// Monotonic turn index.
        turn_index: u64,
    },
    /// Native ATIF trajectory metadata for the active turn.
    AtifTrajectoryStarted {
        /// ATIF schema version used by the emitted trajectory.
        schema_version: atif::SchemaVersion,
        /// Stable session identifier.
        session_id: String,
        /// Agent metadata for the trajectory.
        agent: AtifAgent,
    },
    /// A completed ATIF step derived from the active turn transcript.
    AtifStepCompleted {
        /// Completed ATIF step.
        step: AtifStep,
    },
    /// Aggregate ATIF metrics for the completed turn.
    AtifFinalMetrics {
        /// Final ATIF metrics payload.
        metrics: AtifFinalMetrics,
    },
    /// Full ATIF trajectory after appending the completed turn.
    AtifTrajectoryCompleted {
        /// Completed ATIF trajectory.
        trajectory: Trajectory,
    },
    /// The session phase changed.
    PhaseChanged {
        /// Updated runtime phase.
        phase: SessionPhase,
    },
    /// The runtime reached a visible boundary.
    BoundaryReached {
        /// Boundary kind.
        boundary: SessionBoundary,
    },
    /// Steering was queued for later injection.
    SteeringQueued {
        /// Steering message.
        message: Message,
        /// Injection policy.
        when: SteerWhen,
    },
    /// Steering was committed into the transcript.
    SteeringApplied {
        /// Steering message.
        message: Message,
        /// Injection policy originally used.
        when: SteerWhen,
    },
    /// Approval is required before tool execution may continue.
    ApprovalRequested {
        /// Approval request payload.
        request: ApprovalRequest,
    },
    /// A pending approval request was resolved.
    ApprovalResolved {
        /// Approval resolution.
        decision: ApprovalDecision,
    },
    /// An interrupt took effect at a runtime boundary.
    Interrupted {
        /// Interrupt mode that was applied.
        mode: InterruptMode,
    },
    /// A child spawn was requested by the runtime.
    ChildSpawnRequested {
        /// Parent runtime id.
        parent_id: RuntimeId,
        /// Requested child label, when any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
    },
    /// A child runtime was created through the registry.
    ChildSpawned {
        /// Parent runtime id.
        parent_id: RuntimeId,
        /// Snapshot for the spawned child.
        child: ChildRuntimeState,
    },
    /// A child runtime changed status.
    ChildStatusChanged {
        /// Parent runtime id.
        parent_id: RuntimeId,
        /// Child runtime id.
        child_id: RuntimeId,
        /// Updated child status.
        status: ChildStatus,
    },
    /// A child runtime completed.
    ChildCompleted {
        /// Parent runtime id.
        parent_id: RuntimeId,
        /// Child runtime id.
        child_id: RuntimeId,
        /// Final result summary.
        result: ChildResult,
    },
    /// A child runtime failed or was cancelled.
    ChildFailed {
        /// Parent runtime id.
        parent_id: RuntimeId,
        /// Child runtime id.
        child_id: RuntimeId,
        /// Failure message.
        error: String,
    },
    /// A managed worktree was created.
    WorktreeCreated {
        /// Worktree snapshot after creation.
        worktree: WorktreeState,
    },
    /// A runtime was bound to a managed worktree.
    WorktreeBound {
        /// Runtime that is now bound.
        runtime_id: RuntimeId,
        /// Managed worktree identifier.
        worktree_id: WorktreeId,
    },
    /// A runtime was unbound from a managed worktree.
    WorktreeUnbound {
        /// Runtime that was unbound.
        runtime_id: RuntimeId,
        /// Managed worktree identifier.
        worktree_id: WorktreeId,
    },
    /// A managed worktree was removed.
    WorktreeRemoved {
        /// Worktree snapshot retained for diagnostics.
        worktree: WorktreeState,
    },
    /// A managed worktree operation failed.
    WorktreeFailed {
        /// Runtime attempting the operation.
        runtime_id: RuntimeId,
        /// Semantic operation label.
        operation: String,
        /// Managed worktree identifier when known.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        worktree_id: Option<WorktreeId>,
        /// Failure message.
        message: String,
    },
    /// Typed agent input was queued for registry delivery.
    AgentInputQueued {
        /// Sender runtime id.
        from: RuntimeId,
        /// Target runtime id.
        to: RuntimeId,
        /// Delivery policy.
        delivery: InputDelivery,
        /// Number of messages queued.
        message_count: usize,
    },
    /// Typed agent input was delivered through the registry.
    AgentInputDelivered {
        /// Sender runtime id.
        from: RuntimeId,
        /// Target runtime id.
        to: RuntimeId,
        /// Delivery policy.
        delivery: InputDelivery,
        /// Number of messages delivered.
        message_count: usize,
    },
    /// A cross-agent message was queued for registry delivery.
    AgentMessageQueued {
        /// Message payload.
        message: AgentMessage,
    },
    /// A cross-agent message was delivered to its recipient runtime.
    AgentMessageDelivered {
        /// Message payload.
        message: AgentMessage,
    },
    /// A typed interrupt was sent to another runtime.
    AgentInterrupted {
        /// Sender runtime id.
        from: RuntimeId,
        /// Target runtime id.
        to: RuntimeId,
        /// Interrupt mode requested.
        mode: InterruptMode,
    },
    /// A wait hold expired before all requested runtimes completed.
    AgentWaitTimedOut {
        /// Runtime that was holding.
        runtime_id: RuntimeId,
        /// Wait request in effect.
        wait: WaitRequest,
        /// Structured timeout result.
        result: WaitResult,
    },
    /// A semantic child report was received by the runtime.
    ChildReportReceived {
        /// Report payload.
        report: ChildReport,
    },
    /// A child report was injected into the provider-visible transcript.
    ChildReportInjected {
        /// Report payload.
        report: ChildReport,
        /// Rendered transcript message committed for the report.
        message: Message,
    },
    /// A PTY session was opened.
    PtyOpened {
        /// PTY snapshot after opening.
        pty: PtySessionState,
    },
    /// A PTY session changed status or metadata.
    PtyUpdated {
        /// PTY snapshot after the update.
        pty: PtySessionState,
    },
    /// A PTY event was emitted by the PTY manager.
    PtyEventEmitted {
        /// PTY event payload.
        event: PtyEvent,
    },
    /// A runtime subscribed to PTY events.
    PtySubscribed {
        /// Subscriber runtime.
        runtime_id: RuntimeId,
        /// Subscription details.
        subscription: PtySubscription,
    },
    /// A runtime unsubscribed from PTY events.
    PtyUnsubscribed {
        /// Subscriber runtime.
        runtime_id: RuntimeId,
        /// PTY identifier.
        pty_id: PtyId,
    },
    /// A PTY event was delivered to a runtime through an explicit subscription.
    PtyEventDelivered {
        /// Delivered event payload.
        delivered: DeliveredPtyEvent,
    },
    /// A PTY event was injected into the provider-visible transcript.
    PtyEventInjected {
        /// Delivered event payload.
        delivered: DeliveredPtyEvent,
        /// Rendered transcript message committed for the PTY event.
        message: Message,
    },
    /// A PTY capture request completed.
    PtyCaptured {
        /// Capture result.
        result: PtyCaptureResult,
    },
    /// An envelope was queued for routing.
    EnvelopeQueued {
        /// Envelope payload.
        envelope: Envelope,
    },
    /// An envelope was delivered to a recipient runtime.
    EnvelopeDelivered {
        /// Envelope payload.
        envelope: Envelope,
    },
    /// An envelope was received by a runtime.
    EnvelopeReceived {
        /// Envelope payload.
        envelope: Envelope,
    },
    /// Provider output block started.
    OutputBlockStart {
        /// Block metadata.
        block: Block,
    },
    /// Provider output block delta.
    OutputBlockDelta {
        /// Block identifier.
        id: String,
        /// Delta payload.
        delta: BlockDelta,
    },
    /// Provider output block completed.
    OutputBlockStop {
        /// Block identifier.
        id: String,
    },
    /// Provider usage information.
    Usage {
        /// Usage payload.
        usage: Usage,
    },
    /// A transcript message was committed.
    MessageCommitted {
        /// Committed message.
        message: Message,
    },
    /// A loop-authored provider subcall completed.
    SubcallFinished {
        /// Structured subcall result.
        result: SubcallResult,
    },
    /// A loop-authored transcript rewrite completed.
    TranscriptRewritten {
        /// Structured rewrite result.
        result: TranscriptRewriteResult,
    },
    /// A loop-authored transcript append completed.
    TranscriptMessagesAppended {
        /// Structured append result.
        result: TranscriptAppendResult,
    },
    /// A provider-requested tool call is pending execution.
    ToolCallPending {
        /// Requested tool call.
        call: ToolCall,
    },
    /// A tool call started executing.
    ToolCallStarted {
        /// Requested tool call.
        call: ToolCall,
    },
    /// A tool call finished executing.
    ToolCallFinished {
        /// Requested tool call.
        call: ToolCall,
        /// Structured tool result.
        result: ToolExecutionResult,
    },
    /// The runtime retried provider work before any visible output.
    Retry {
        /// Current attempt number.
        attempt: u32,
        /// Maximum attempt count.
        max: u32,
        /// Retry reason.
        error: String,
    },
    /// Context compaction occurred or was requested.
    Compaction {
        /// Original transcript message count.
        original_messages: usize,
        /// Summary token estimate.
        summary_tokens: usize,
    },
    /// Runtime warning for loops stuck in a repeating tool cycle.
    DoomLoopWarning {
        /// Tool name involved in the repeated pattern.
        tool_name: String,
        /// Number of repeated executions observed.
        repetitions: u32,
    },
    /// The active turn finished.
    TurnFinished {
        /// Stable session identifier.
        session_id: Ulid,
        /// Finished turn index.
        turn_index: u64,
        /// Optional provider finish reason.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        finish_reason: Option<FinishReason>,
    },
    /// A runtime error occurred.
    Error {
        /// Human-readable message.
        message: String,
        /// Whether the caller may reasonably retry.
        recoverable: bool,
    },
}
