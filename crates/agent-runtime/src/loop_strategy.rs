use std::sync::Arc;

use futures::future::BoxFuture;
use provider::ProviderInfo;

use crate::{
    ApprovalRequest, InputDelivery, OpenPtyRequest, PtyCaptureRequest, PtyExecRequest, PtyId,
    PtySubscription, RuntimeConfig, RuntimeError, RuntimeId, SessionState, SpawnRequest,
    SubcallRequest, TranscriptAppend, TranscriptRewrite, WaitRequest,
};

/// Immutable context presented to loop strategies when deciding the next step.
#[derive(Debug, Clone)]
pub struct LoopContext {
    provider_info: ProviderInfo,
    config: Arc<RuntimeConfig>,
    state: SessionState,
}

impl LoopContext {
    /// Creates a new decision context from runtime-owned state.
    pub fn new(
        provider_info: ProviderInfo,
        config: Arc<RuntimeConfig>,
        state: SessionState,
    ) -> Self {
        Self {
            provider_info,
            config,
            state,
        }
    }

    /// Returns provider metadata for the active session engine.
    pub fn provider_info(&self) -> &ProviderInfo {
        &self.provider_info
    }

    /// Returns the shared runtime configuration.
    pub fn config(&self) -> &RuntimeConfig {
        self.config.as_ref()
    }

    /// Returns the current immutable session snapshot.
    pub fn state(&self) -> &SessionState {
        &self.state
    }
}

/// Strategy decision returned to the runtime engine.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub enum LoopDecision {
    /// Run another provider step using the current transcript.
    RunProvider,
    /// Execute the next pending tool call or tool batch.
    ExecuteToolBatch,
    /// Wait for approval before continuing tool execution.
    RequestToolApproval {
        /// Approval request to surface to the caller.
        request: ApprovalRequest,
    },
    /// Wait for more external input before doing more work.
    WaitForInput,
    /// Finish the current turn.
    FinishTurn,
    /// Spawn a child runtime through the registry.
    SpawnAgent {
        /// Child spawn request.
        request: SpawnRequest,
        /// Optional immediate hold after spawning.
        wait: Option<WaitRequest>,
    },
    /// Wait for the specified child runtimes to complete.
    WaitForAgents {
        /// Runtime ids to wait on.
        ids: Vec<RuntimeId>,
        /// Wait behavior.
        wait: WaitRequest,
    },
    /// Send additional input to another runtime through the registry.
    SendAgentInput {
        /// Target runtime id.
        runtime_id: RuntimeId,
        /// Messages to deliver.
        input: Vec<provider::Message>,
        /// Boundary policy for delivery.
        delivery: InputDelivery,
    },
    /// Run an isolated provider subcall without mutating the outer transcript.
    RunSubcall {
        /// Subcall request to execute.
        request: SubcallRequest,
    },
    /// Replace the current transcript with a concrete loop-authored transcript.
    RewriteTranscript {
        /// Transcript replacement plan.
        rewrite: TranscriptRewrite,
    },
    /// Append concrete runtime-authored messages to the transcript.
    AppendTranscriptMessages {
        /// Transcript append plan.
        append: TranscriptAppend,
    },
    /// Queue steering for the current runtime at a safe boundary.
    QueueSteering {
        /// Steering message to inject into the local transcript.
        message: provider::Message,
        /// Boundary policy for applying the steering.
        when: crate::SteerWhen,
    },
    /// Open a new runtime-managed PTY session.
    OpenPty {
        /// PTY creation request.
        request: OpenPtyRequest,
    },
    /// Write raw input into an existing PTY.
    WritePtyInput {
        /// Target PTY identifier.
        pty_id: PtyId,
        /// Raw PTY input.
        input: String,
        /// Optional wait before the runtime continues.
        wait_ms: Option<u64>,
    },
    /// Execute a PTY command or input batch.
    ExecutePtyBatch {
        /// Structured execution request.
        request: PtyExecRequest,
    },
    /// Capture the current PTY screen and optional incremental output.
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
    /// Interrupt an existing PTY.
    InterruptPty {
        /// PTY identifier.
        pty_id: PtyId,
    },
    /// Release the foreground expectation while keeping the PTY alive.
    BackgroundPty {
        /// PTY identifier.
        pty_id: PtyId,
    },
    /// Close an existing PTY session.
    ClosePty {
        /// PTY identifier.
        pty_id: PtyId,
    },
    /// Subscribe the current runtime to PTY events.
    SubscribePty {
        /// PTY event subscription.
        subscription: PtySubscription,
    },
    /// Remove a PTY event subscription for the current runtime.
    UnsubscribePty {
        /// PTY identifier.
        pty_id: PtyId,
    },
    /// Interrupt another runtime through the registry.
    InterruptAgent {
        /// Target runtime id.
        runtime_id: RuntimeId,
        /// Interrupt mode to apply.
        mode: crate::InterruptMode,
    },
    /// Pause another runtime through the registry.
    PauseAgent {
        /// Target runtime id.
        runtime_id: RuntimeId,
    },
    /// Resume another runtime through the registry.
    ResumeAgent {
        /// Target runtime id.
        runtime_id: RuntimeId,
    },
    /// Compact the current transcript context.
    CompactContext,
}

/// Strategy trait implemented by concrete loop crates over the reusable runtime.
pub trait LoopStrategy: Send + Sync {
    /// Stable loop name used for diagnostics.
    fn name(&self) -> &'static str;

    /// Decides what the runtime should do next from the current session state.
    fn decide<'a>(&'a self, ctx: LoopContext) -> BoxFuture<'a, Result<LoopDecision, RuntimeError>>;
}
