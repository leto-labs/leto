use std::sync::Arc;

use futures::future::BoxFuture;
use provider::ProviderInfo;

use crate::{
    ApprovalRequest, InputDelivery, RuntimeConfig, RuntimeError, RuntimeId, SessionState,
    SpawnRequest, WaitRequest,
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
