use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::sync::{Arc, Mutex as StdMutex};

use futures::StreamExt;
use provider::{
    Block, BlockDelta, BlockKind, Event, FinishReason, Message, MessageRole, Provider,
    ProviderInfo, Request,
};
use serde::Deserialize;
use tokio::sync::{Mutex, broadcast, mpsc};
use tokio::time::{Duration, sleep};
use tokio_util::sync::CancellationToken;
use ulid::Ulid;

use crate::{
    ActiveWait, AgentCommand, AgentListScope, AgentMessage, AgentMessageDelivery,
    AgentMessageFilter, AgentMessageKind, ApprovalDecision, ApprovalRequest, ChildReport,
    ChildReportKind, ChildResult, ChildRuntimeState, ChildStatus, ControlEvent, Envelope,
    EnvelopeKind, HistoryMode, InputDelivery, InterruptMode, LoopContext, LoopDecision,
    LoopStrategy, ParentRef, PendingInput, PendingSteering, ResultMode, RuntimeConfig,
    RuntimeError, RuntimeEvent, RuntimeId, SessionBoundary, SessionCommand, SessionPhase,
    SessionState, SpawnId, SpawnMode, SpawnRequest, SteerWhen, ToolApproval, ToolCall,
    ToolExecutionResult, ToolExecutor, WaitOutcome, WaitRequest, WaitResult, WaitTargetOutcome,
    WaitTimeoutAction,
};

#[derive(Debug, Clone)]
pub struct InferenceStep {
    /// Assistant message assembled from streamed provider blocks.
    pub message: Option<Message>,
    /// Tool calls requested by the provider.
    pub tool_calls: Vec<ToolCall>,
    /// Optional usage summary emitted by the provider.
    pub usage: Option<provider::Usage>,
    /// Optional provider finish reason.
    pub finish_reason: Option<FinishReason>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct RoutedAgentInput {
    input: Vec<Message>,
    delivery: InputDelivery,
}

/// Reusable profile describing how a runtime should be constructed.
#[derive(Clone)]
pub struct RuntimeProfile {
    provider: Arc<dyn Provider>,
    tools: Arc<dyn ToolExecutor>,
    strategy: Arc<dyn LoopStrategy>,
    config: RuntimeConfig,
}

impl RuntimeProfile {
    /// Creates a new reusable runtime profile.
    pub fn new(
        provider: Arc<dyn Provider>,
        tools: Arc<dyn ToolExecutor>,
        strategy: Arc<dyn LoopStrategy>,
        config: RuntimeConfig,
    ) -> Self {
        Self {
            provider,
            tools,
            strategy,
            config,
        }
    }
}

/// Handle to a long-lived in-memory bidirectional session engine.
#[derive(Clone)]
pub struct SessionEngine {
    session_id: RuntimeId,
    provider: Arc<dyn Provider>,
    tools: Arc<dyn ToolExecutor>,
    config: Arc<RuntimeConfig>,
    state: Arc<Mutex<SessionState>>,
    command_tx: mpsc::Sender<EngineCommand>,
    event_tx: broadcast::Sender<RuntimeEvent>,
    registry: Arc<RuntimeRegistryInner>,
}

#[derive(Clone)]
struct RuntimeHandle {
    runtime_id: RuntimeId,
    state: Arc<Mutex<SessionState>>,
    command_tx: mpsc::Sender<EngineCommand>,
    event_tx: broadcast::Sender<RuntimeEvent>,
}

struct RuntimeRegistryState {
    handles: BTreeMap<RuntimeId, RuntimeHandle>,
    parent_to_children: BTreeMap<RuntimeId, BTreeSet<RuntimeId>>,
    child_to_parent: BTreeMap<RuntimeId, ParentRef>,
    profiles: BTreeMap<String, RuntimeProfile>,
}

struct RuntimeRegistryInner {
    state: StdMutex<RuntimeRegistryState>,
}

enum EngineCommand {
    External(SessionCommand),
    ProviderFinished(Result<InferenceStep, RuntimeError>),
    ToolFinished {
        call: ToolCall,
        result: Result<ToolExecutionResult, RuntimeError>,
    },
    InboundEnvelope(Envelope),
    ChildUpdate(ChildUpdate),
    WaitTimedOut(WaitResult),
}

enum ChildUpdate {
    Status {
        child_id: RuntimeId,
        status: ChildStatus,
    },
    Completed {
        child_id: RuntimeId,
        result: ChildResult,
    },
    Failed {
        child_id: RuntimeId,
        error: String,
    },
}

struct EngineRuntime {
    runtime_id: RuntimeId,
    provider: Arc<dyn Provider>,
    tools: Arc<dyn ToolExecutor>,
    strategy: Arc<dyn LoopStrategy>,
    config: Arc<RuntimeConfig>,
    state: Arc<Mutex<SessionState>>,
    command_tx: mpsc::Sender<EngineCommand>,
    command_rx: mpsc::Receiver<EngineCommand>,
    event_tx: broadcast::Sender<RuntimeEvent>,
    registry: Arc<RuntimeRegistryInner>,
    active_provider_cancel: Option<CancellationToken>,
}

impl SessionEngine {
    /// Creates a new root session engine with its own internal runtime registry.
    pub fn new(
        provider: Arc<dyn Provider>,
        tools: Arc<dyn ToolExecutor>,
        strategy: Arc<dyn LoopStrategy>,
        config: RuntimeConfig,
        initial_state: SessionState,
    ) -> Self {
        let registry = Arc::new(RuntimeRegistryInner::new());
        registry.register_profile(
            "default",
            RuntimeProfile::new(
                provider.clone(),
                tools.clone(),
                strategy.clone(),
                config.clone(),
            ),
        );
        Self::with_registry(registry, provider, tools, strategy, config, initial_state)
    }

    fn with_registry(
        registry: Arc<RuntimeRegistryInner>,
        provider: Arc<dyn Provider>,
        tools: Arc<dyn ToolExecutor>,
        strategy: Arc<dyn LoopStrategy>,
        config: RuntimeConfig,
        initial_state: SessionState,
    ) -> Self {
        let session_id = initial_state.session_id;
        let state = Arc::new(Mutex::new(initial_state));
        let (command_tx, command_rx) = mpsc::channel(256);
        let (event_tx, _) = broadcast::channel(512);
        let config = Arc::new(config);

        let handle = RuntimeHandle {
            runtime_id: session_id,
            state: state.clone(),
            command_tx: command_tx.clone(),
            event_tx: event_tx.clone(),
        };
        registry.register_handle(handle);

        let runtime = EngineRuntime {
            runtime_id: session_id,
            provider: provider.clone(),
            tools: tools.clone(),
            strategy,
            config: config.clone(),
            state: state.clone(),
            command_tx: command_tx.clone(),
            command_rx,
            event_tx: event_tx.clone(),
            registry: registry.clone(),
            active_provider_cancel: None,
        };
        tokio::spawn(async move {
            runtime.run().await;
        });

        Self {
            session_id,
            provider,
            tools,
            config,
            state,
            command_tx,
            event_tx,
            registry,
        }
    }

    /// Registers a named runtime profile for future child spawns.
    pub fn register_profile(&self, name: impl Into<String>, profile: RuntimeProfile) {
        self.registry.register_profile(name, profile);
    }

    /// Returns the current session identifier.
    pub fn session_id(&self) -> RuntimeId {
        self.session_id
    }

    /// Returns static provider metadata for this session engine.
    pub fn provider_info(&self) -> ProviderInfo {
        self.provider.info()
    }

    /// Returns provider-visible tool definitions for this session.
    pub fn tool_definitions(&self) -> Vec<provider::ToolDefinition> {
        merge_tool_definitions(self.tools.definitions(), native_agent_tool_definitions())
    }

    /// Returns the currently known direct child snapshots.
    pub async fn children(&self) -> Vec<ChildRuntimeState> {
        let state = self.state.lock().await;
        state.children.values().cloned().collect()
    }

    /// Submits a command into the long-lived session engine.
    pub async fn submit(&self, command: SessionCommand) -> Result<(), RuntimeError> {
        self.command_tx
            .send(EngineCommand::External(command))
            .await
            .map_err(|_| RuntimeError::Closed)
    }

    /// Returns a new event subscription receiver for runtime events.
    pub fn subscribe(&self) -> broadcast::Receiver<RuntimeEvent> {
        self.event_tx.subscribe()
    }

    /// Returns a snapshot of the current in-memory session state.
    pub async fn snapshot(&self) -> SessionState {
        self.state.lock().await.clone()
    }

    /// Backwards-compatible alias for callers expecting a session-state getter.
    pub async fn session_state(&self) -> SessionState {
        self.snapshot().await
    }

    fn emit(&self, event: RuntimeEvent) {
        let _ = self.event_tx.send(event);
    }

    /// Runs a provider subcall without mutating the outer session transcript.
    pub async fn subcall(
        &self,
        request: Request,
        cancel: CancellationToken,
    ) -> Result<InferenceStep, RuntimeError> {
        run_provider_step(
            self.provider.clone(),
            self.config.clone(),
            request,
            None,
            cancel,
        )
        .await
    }

    async fn spawn_child_tool(&self, request: SpawnRequest) -> Result<RuntimeId, RuntimeError> {
        self.emit(RuntimeEvent::ChildSpawnRequested {
            parent_id: self.session_id,
            label: request.label.clone(),
        });
        let child_id = self.spawn_child_runtime_from_handle(request).await?;
        let child = self.registry.handle(child_id)?;
        let child_snapshot = {
            let state = self.state.lock().await;
            state.children.get(&child_id).cloned().ok_or_else(|| {
                RuntimeError::Internal(format!("missing child snapshot: {child_id}"))
            })?
        };
        self.emit(RuntimeEvent::ChildSpawned {
            parent_id: self.session_id,
            child: child_snapshot,
        });
        let parent_command = self.command_tx.clone();
        tokio::spawn(async move {
            let mut events = child.event_tx.subscribe();
            loop {
                let event = match events.recv().await {
                    Ok(event) => event,
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                };
                let update = match event {
                    RuntimeEvent::TurnStarted { .. } => Some(ChildUpdate::Status {
                        child_id,
                        status: ChildStatus::Running,
                    }),
                    RuntimeEvent::BoundaryReached {
                        boundary: SessionBoundary::AwaitingInput,
                    } => Some(ChildUpdate::Status {
                        child_id,
                        status: ChildStatus::AwaitingInput,
                    }),
                    RuntimeEvent::BoundaryReached {
                        boundary: SessionBoundary::Paused,
                    } => Some(ChildUpdate::Status {
                        child_id,
                        status: ChildStatus::Paused,
                    }),
                    RuntimeEvent::TurnFinished { .. } => {
                        let snapshot = child.state.lock().await.clone();
                        Some(ChildUpdate::Completed {
                            child_id,
                            result: ChildResult {
                                output: snapshot.transcript.clone(),
                                success: true,
                            },
                        })
                    }
                    RuntimeEvent::Error {
                        message,
                        recoverable: false,
                    } => Some(ChildUpdate::Failed {
                        child_id,
                        error: message,
                    }),
                    _ => None,
                };
                let Some(update) = update else {
                    continue;
                };
                if parent_command
                    .send(EngineCommand::ChildUpdate(update))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        });
        Ok(child_id)
    }

    async fn spawn_child_runtime_from_handle(
        &self,
        request: SpawnRequest,
    ) -> Result<RuntimeId, RuntimeError> {
        let parent_transcript = {
            let state = self.state.lock().await;
            state.transcript.clone()
        };
        let profile = self.registry.profile(request.profile.as_deref())?;
        let mut config = request
            .config_override
            .clone()
            .unwrap_or_else(|| profile.config.clone());
        if config.model.is_none() {
            config.model = self.config.model.clone().or(profile.config.model.clone());
        }
        if let Some(model_override) = request.model_override.clone() {
            config.model = Some(model_override);
        }

        let child_id = Ulid::new();
        let spawn_id = Ulid::new();
        let mut child_state = match request.history_mode {
            HistoryMode::Empty => SessionState::new(child_id),
            HistoryMode::ForkParentTranscript => {
                SessionState::with_transcript(child_id, parent_transcript)
            }
        };
        child_state.parent = Some(ParentRef {
            parent_runtime_id: self.session_id,
            spawn_id,
        });

        let child = SessionEngine::with_registry(
            self.registry.clone(),
            profile.provider.clone(),
            profile.tools.clone(),
            profile.strategy.clone(),
            config,
            child_state,
        );
        self.registry
            .link_child(self.session_id, child_id, spawn_id);

        let child_snapshot = ChildRuntimeState {
            runtime_id: child_id,
            spawn_id,
            label: request.label.clone(),
            status: if request.initial_input.is_empty() {
                ChildStatus::AwaitingInput
            } else {
                ChildStatus::Starting
            },
            spawn_mode: request.spawn_mode,
            result_mode: request.result_mode,
            last_result: None,
        };
        {
            let mut state = self.state.lock().await;
            state.children.insert(child_id, child_snapshot);
        }

        if !request.initial_input.is_empty() {
            child
                .submit(SessionCommand::SubmitInput {
                    input: request.initial_input,
                    source: Some(crate::SessionInputSource {
                        origin: self.session_id.to_string(),
                        channel: Some("parent-runtime".into()),
                    }),
                })
                .await?;
        }

        Ok(child_id)
    }
}

impl RuntimeRegistryInner {
    fn new() -> Self {
        Self {
            state: StdMutex::new(RuntimeRegistryState {
                handles: BTreeMap::new(),
                parent_to_children: BTreeMap::new(),
                child_to_parent: BTreeMap::new(),
                profiles: BTreeMap::new(),
            }),
        }
    }

    fn register_profile(&self, name: impl Into<String>, profile: RuntimeProfile) {
        self.state
            .lock()
            .expect("runtime registry poisoned")
            .profiles
            .insert(name.into(), profile);
    }

    fn register_handle(&self, handle: RuntimeHandle) {
        self.state
            .lock()
            .expect("runtime registry poisoned")
            .handles
            .insert(handle.runtime_id, handle);
    }

    fn profile(&self, name: Option<&str>) -> Result<RuntimeProfile, RuntimeError> {
        let state = self.state.lock().expect("runtime registry poisoned");
        let key = name.unwrap_or("default");
        state
            .profiles
            .get(key)
            .cloned()
            .ok_or_else(|| RuntimeError::Internal(format!("unknown runtime profile: {key}")))
    }

    fn handle(&self, runtime_id: RuntimeId) -> Result<RuntimeHandle, RuntimeError> {
        self.state
            .lock()
            .expect("runtime registry poisoned")
            .handles
            .get(&runtime_id)
            .cloned()
            .ok_or_else(|| RuntimeError::Internal(format!("unknown runtime id: {runtime_id}")))
    }

    fn link_child(&self, parent_id: RuntimeId, child_id: RuntimeId, spawn_id: SpawnId) {
        let mut state = self.state.lock().expect("runtime registry poisoned");
        state
            .parent_to_children
            .entry(parent_id)
            .or_default()
            .insert(child_id);
        state.child_to_parent.insert(
            child_id,
            ParentRef {
                parent_runtime_id: parent_id,
                spawn_id,
            },
        );
    }

    async fn route_envelope(&self, envelope: Envelope) -> Result<(), RuntimeError> {
        let handle = self.handle(envelope.to)?;
        handle
            .command_tx
            .send(EngineCommand::InboundEnvelope(envelope))
            .await
            .map_err(|_| RuntimeError::Closed)
    }
}

impl EngineRuntime {
    async fn run(mut self) {
        let _ = self
            .set_phase_and_boundary(SessionPhase::Idle, Some(SessionBoundary::AwaitingInput))
            .await;

        while let Some(command) = self.command_rx.recv().await {
            let result = match command {
                EngineCommand::External(command) => self.handle_external_command(command).await,
                EngineCommand::ProviderFinished(result) => {
                    self.handle_provider_finished(result).await
                }
                EngineCommand::ToolFinished { call, result } => {
                    self.handle_tool_finished(call, result).await
                }
                EngineCommand::InboundEnvelope(envelope) => {
                    self.handle_inbound_envelope(envelope).await
                }
                EngineCommand::ChildUpdate(update) => self.handle_child_update(update).await,
                EngineCommand::WaitTimedOut(result) => self.handle_wait_timed_out(result).await,
            };

            if let Err(error) = result {
                self.emit(RuntimeEvent::Error {
                    message: error.to_string(),
                    recoverable: error.recoverable(),
                });
            }
        }
    }

    async fn handle_external_command(
        &mut self,
        command: SessionCommand,
    ) -> Result<(), RuntimeError> {
        match command {
            SessionCommand::SubmitInput { input, source } => {
                {
                    let mut state = self.state.lock().await;
                    state.pending_inputs.push(PendingInput {
                        input: input.clone(),
                        source: source.clone(),
                    });
                }
                self.emit(RuntimeEvent::InputQueued {
                    message_count: input.len(),
                    source,
                });
                self.drive().await
            }
            SessionCommand::Control(control) => {
                self.emit(RuntimeEvent::ControlQueued {
                    control: control.clone(),
                });
                self.handle_control(control).await?;
                self.drive().await
            }
            SessionCommand::Approve(decision) => {
                self.handle_approval(decision).await?;
                self.drive().await
            }
            SessionCommand::Agent(command) => {
                self.emit(RuntimeEvent::AgentQueued);
                self.handle_agent_command(command).await?;
                self.drive().await
            }
        }
    }

    async fn handle_agent_command(&mut self, command: AgentCommand) -> Result<(), RuntimeError> {
        match command {
            AgentCommand::SpawnAgent { request, wait } => {
                let child_id = self.spawn_child_runtime(request).await?;
                if let Some(wait) = wait {
                    self.start_wait(vec![child_id], wait).await?;
                }
            }
            AgentCommand::SendAgentInput {
                runtime_id,
                input,
                delivery,
            } => {
                self.send_agent_input(runtime_id, input, delivery).await?;
            }
            AgentCommand::SendAgentMessage {
                runtime_id,
                message,
            } => {
                self.send_agent_message(runtime_id, message).await?;
            }
            AgentCommand::InterruptAgent { runtime_id, mode } => {
                self.send_runtime_control(
                    runtime_id,
                    EnvelopeKind::Interrupt,
                    serde_json::to_value(mode).map_err(|error| {
                        RuntimeError::Internal(format!("invalid interrupt payload: {error}"))
                    })?,
                )
                .await?;
                self.emit(RuntimeEvent::AgentInterrupted {
                    from: self.runtime_id,
                    to: runtime_id,
                    mode,
                });
            }
            AgentCommand::PauseAgent { runtime_id } => {
                self.send_runtime_control(runtime_id, EnvelopeKind::Pause, serde_json::Value::Null)
                    .await?;
            }
            AgentCommand::ResumeAgent { runtime_id } => {
                self.send_runtime_control(
                    runtime_id,
                    EnvelopeKind::Resume,
                    serde_json::Value::Null,
                )
                .await?;
            }
            AgentCommand::WaitForAgents { ids, wait } => {
                self.start_wait(ids, wait).await?;
            }
            AgentCommand::ReadAgentMessages { filter } => {
                let _ = self.read_agent_messages(filter).await;
            }
            AgentCommand::ListAgents { scope } => {
                let _ = self.list_visible_agents(scope).await;
            }
        }
        Ok(())
    }

    async fn handle_control(&mut self, control: ControlEvent) -> Result<(), RuntimeError> {
        match control {
            ControlEvent::Interrupt { mode } => {
                {
                    let mut state = self.state.lock().await;
                    state.pending_interrupt = Some(mode);
                }
                if matches!(mode, InterruptMode::ImmediateCancel) {
                    if let Some(cancel) = &self.active_provider_cancel {
                        cancel.cancel();
                    }
                }
            }
            ControlEvent::Steer { message, when } => {
                {
                    let mut state = self.state.lock().await;
                    state.pending_steering.push(PendingSteering {
                        message: message.clone(),
                        when,
                    });
                }
                self.emit(RuntimeEvent::SteeringQueued { message, when });
            }
            ControlEvent::Pause => {
                let phase = self.state.lock().await.phase;
                if matches!(phase, SessionPhase::Idle) {
                    self.set_phase_and_boundary(
                        SessionPhase::Paused,
                        Some(SessionBoundary::Paused),
                    )
                    .await?;
                } else {
                    let mut state = self.state.lock().await;
                    state.pending_interrupt = Some(InterruptMode::PauseAtBoundary);
                }
            }
            ControlEvent::Resume => {
                let was_paused = {
                    let state = self.state.lock().await;
                    matches!(state.phase, SessionPhase::Paused)
                };
                if was_paused {
                    self.set_phase_and_boundary(SessionPhase::Idle, None)
                        .await?;
                }
            }
        }
        Ok(())
    }

    async fn handle_approval(&mut self, decision: ApprovalDecision) -> Result<(), RuntimeError> {
        let pending_request = self.state.lock().await.pending_approval.clone();
        let Some(request) = pending_request else {
            return Ok(());
        };

        let request_id = match &decision {
            ApprovalDecision::Allow { request_id } | ApprovalDecision::Deny { request_id, .. } => {
                request_id.as_str()
            }
        };
        if request.id != request_id {
            return Ok(());
        }

        self.emit(RuntimeEvent::ApprovalResolved {
            decision: decision.clone(),
        });

        match decision {
            ApprovalDecision::Allow { .. } => {
                {
                    let mut state = self.state.lock().await;
                    state.pending_approval = None;
                }
                self.set_phase_and_boundary(SessionPhase::Idle, None)
                    .await?;
            }
            ApprovalDecision::Deny { reason, .. } => {
                {
                    let mut state = self.state.lock().await;
                    state.pending_approval = None;
                    state.pending_tool_calls.clear();
                    state.pending_completion = true;
                    state.last_finish_reason = Some(FinishReason::Error);
                }
                self.set_phase_and_boundary(SessionPhase::Idle, None)
                    .await?;
                self.emit(RuntimeEvent::Error {
                    message: reason.unwrap_or_else(|| "approval denied".into()),
                    recoverable: false,
                });
            }
        }
        Ok(())
    }

    async fn handle_inbound_envelope(&mut self, envelope: Envelope) -> Result<(), RuntimeError> {
        {
            let mut state = self.state.lock().await;
            state.inbox.push_back(envelope.clone());
        }
        self.emit(RuntimeEvent::EnvelopeReceived {
            envelope: envelope.clone(),
        });

        match envelope.kind {
            EnvelopeKind::Input => {
                let routed = serde_json::from_value::<RoutedAgentInput>(envelope.payload.clone())
                    .or_else(|_| {
                        serde_json::from_value::<Vec<Message>>(envelope.payload.clone()).map(
                            |input| RoutedAgentInput {
                                input,
                                delivery: InputDelivery::NextSafeBoundary,
                            },
                        )
                    })
                    .map_err(|error| {
                        RuntimeError::Internal(format!("invalid input envelope: {error}"))
                    })?;
                self.queue_agent_input(envelope.from, routed.input, routed.delivery)
                    .await?;
            }
            EnvelopeKind::Steer => {
                let message: Message =
                    serde_json::from_value(envelope.payload.clone()).map_err(|error| {
                        RuntimeError::Internal(format!("invalid steer envelope: {error}"))
                    })?;
                let mut state = self.state.lock().await;
                state.pending_steering.push(PendingSteering {
                    message,
                    when: SteerWhen::NextSafeBoundary,
                });
            }
            EnvelopeKind::Interrupt => {
                let mode: InterruptMode = serde_json::from_value(envelope.payload.clone())
                    .map_err(|error| {
                        RuntimeError::Internal(format!("invalid interrupt envelope: {error}"))
                    })?;
                self.handle_control(ControlEvent::Interrupt { mode })
                    .await?;
            }
            EnvelopeKind::Pause => {
                self.handle_control(ControlEvent::Pause).await?;
            }
            EnvelopeKind::Resume => {
                self.handle_control(ControlEvent::Resume).await?;
            }
            EnvelopeKind::ApprovalDecision => {
                let decision: ApprovalDecision = serde_json::from_value(envelope.payload.clone())
                    .map_err(|error| {
                    RuntimeError::Internal(format!("invalid approval decision envelope: {error}"))
                })?;
                self.handle_approval(decision).await?;
            }
            EnvelopeKind::ApprovalRequest
            | EnvelopeKind::Progress
            | EnvelopeKind::Result
            | EnvelopeKind::Mail => {
                if matches!(envelope.kind, EnvelopeKind::Mail) {
                    if let Ok(message) =
                        serde_json::from_value::<AgentMessage>(envelope.payload.clone())
                    {
                        {
                            let mut state = self.state.lock().await;
                            state.mailbox.push_back(message.clone());
                            if message.delivery_mode == AgentMessageDelivery::Direct {
                                state.pending_direct_messages.push(message.clone());
                            }
                        }
                        self.emit(RuntimeEvent::AgentMessageDelivered { message });
                    }
                }
                if let Some(report) = self.build_child_report_from_envelope(&envelope).await? {
                    {
                        let mut state = self.state.lock().await;
                        state.pending_child_reports.push(report.clone());
                    }
                    self.emit(RuntimeEvent::ChildReportReceived { report });
                }
            }
        }

        self.drive().await
    }

    async fn handle_child_update(&mut self, update: ChildUpdate) -> Result<(), RuntimeError> {
        match update {
            ChildUpdate::Status { child_id, status } => {
                {
                    let mut state = self.state.lock().await;
                    if let Some(child) = state.children.get_mut(&child_id) {
                        if !child.status.is_terminal() {
                            child.status = status;
                        }
                    }
                }
                self.emit(RuntimeEvent::ChildStatusChanged {
                    parent_id: self.runtime_id,
                    child_id,
                    status,
                });
            }
            ChildUpdate::Completed { child_id, result } => {
                let result_envelope = Envelope::new(
                    child_id,
                    self.runtime_id,
                    EnvelopeKind::Result,
                    serde_json::to_value(&result).map_err(|error| {
                        RuntimeError::Internal(format!("invalid result value: {error}"))
                    })?,
                );
                let child_label = {
                    let state = self.state.lock().await;
                    state
                        .children
                        .get(&child_id)
                        .and_then(|child| child.label.clone())
                };
                {
                    let mut state = self.state.lock().await;
                    if let Some(child) = state.children.get_mut(&child_id) {
                        child.status = ChildStatus::Completed;
                        child.last_result = Some(result.clone());
                    }
                    state.inbox.push_back(result_envelope.clone());
                    state.pending_child_reports.push(ChildReport {
                        child_id,
                        child_label,
                        kind: ChildReportKind::Result,
                        payload: serde_json::to_value(&result).map_err(|error| {
                            RuntimeError::Internal(format!("invalid child result value: {error}"))
                        })?,
                        is_final: true,
                    });
                }
                self.emit(RuntimeEvent::EnvelopeReceived {
                    envelope: result_envelope,
                });
                if let Some(report) = self
                    .state
                    .lock()
                    .await
                    .pending_child_reports
                    .last()
                    .cloned()
                {
                    self.emit(RuntimeEvent::ChildReportReceived { report });
                }
                self.emit(RuntimeEvent::ChildCompleted {
                    parent_id: self.runtime_id,
                    child_id,
                    result,
                });
            }
            ChildUpdate::Failed { child_id, error } => {
                let child_label = {
                    let state = self.state.lock().await;
                    state
                        .children
                        .get(&child_id)
                        .and_then(|child| child.label.clone())
                };
                {
                    let mut state = self.state.lock().await;
                    if let Some(child) = state.children.get_mut(&child_id) {
                        child.status = ChildStatus::Failed;
                    }
                    state.pending_child_reports.push(ChildReport {
                        child_id,
                        child_label,
                        kind: ChildReportKind::Failure,
                        payload: serde_json::json!({ "error": error }),
                        is_final: true,
                    });
                }
                if let Some(report) = self
                    .state
                    .lock()
                    .await
                    .pending_child_reports
                    .last()
                    .cloned()
                {
                    self.emit(RuntimeEvent::ChildReportReceived { report });
                }
                self.emit(RuntimeEvent::ChildFailed {
                    parent_id: self.runtime_id,
                    child_id,
                    error: error.clone(),
                });
            }
        }

        self.resume_from_wait_if_ready().await?;

        self.drive().await
    }

    async fn handle_provider_finished(
        &mut self,
        result: Result<InferenceStep, RuntimeError>,
    ) -> Result<(), RuntimeError> {
        self.active_provider_cancel = None;
        self.set_phase_and_boundary(SessionPhase::Idle, None)
            .await?;

        match result {
            Ok(step) => {
                if let Some(message) = step.message {
                    self.commit_messages(vec![message]).await?;
                }

                let approval_request = self.build_approval_request(&step.tool_calls);
                {
                    let mut state = self.state.lock().await;
                    state.pending_tool_calls = step.tool_calls.clone();
                    state.pending_approval = approval_request.clone();
                    state.pending_completion = step.tool_calls.is_empty();
                    state.last_finish_reason = step.finish_reason;
                }

                for call in step.tool_calls {
                    self.emit(RuntimeEvent::ToolCallPending { call });
                }
            }
            Err(RuntimeError::Cancelled) => {}
            Err(error) => {
                {
                    let mut state = self.state.lock().await;
                    state.pending_tool_calls.clear();
                    state.pending_completion = true;
                    state.last_finish_reason = Some(FinishReason::Error);
                }
                self.emit(RuntimeEvent::Error {
                    message: error.to_string(),
                    recoverable: error.recoverable(),
                });
            }
        }

        self.drive().await
    }

    async fn handle_tool_finished(
        &mut self,
        call: ToolCall,
        result: Result<ToolExecutionResult, RuntimeError>,
    ) -> Result<(), RuntimeError> {
        {
            let mut state = self.state.lock().await;
            state.active_tool_call = None;
        }
        self.set_phase_and_boundary(SessionPhase::Idle, None)
            .await?;

        match result {
            Ok(result) => {
                let result = ToolExecutionResult {
                    output: truncate_json_value(&result.output, self.config.tool_output_max_bytes),
                    is_error: result.is_error,
                };
                let tool_result_message = Message {
                    role: MessageRole::User,
                    content: vec![provider::ContentBlock::ToolResult {
                        call_id: call.id.clone(),
                        output: result.output.clone(),
                        is_error: Some(result.is_error),
                    }],
                };
                self.commit_messages(vec![tool_result_message]).await?;
                self.emit(RuntimeEvent::ToolCallFinished {
                    call,
                    result: result.clone(),
                });
            }
            Err(error) => {
                {
                    let mut state = self.state.lock().await;
                    state.pending_tool_calls.clear();
                    state.pending_completion = true;
                    state.last_finish_reason = Some(FinishReason::Error);
                }
                self.emit(RuntimeEvent::Error {
                    message: error.to_string(),
                    recoverable: error.recoverable(),
                });
            }
        }

        self.drive().await
    }

    async fn drive(&mut self) -> Result<(), RuntimeError> {
        loop {
            self.begin_turn_if_needed().await?;

            if self.apply_pending_interrupt_if_ready().await? {
                break;
            }
            if self.apply_queued_steering().await? {
                continue;
            }
            if self.apply_pending_direct_messages().await? {
                continue;
            }
            if self.apply_pending_child_reports().await? {
                continue;
            }

            let snapshot = self.state.lock().await.clone();
            match snapshot.phase {
                SessionPhase::RunningProvider
                | SessionPhase::RunningTool
                | SessionPhase::AwaitingApproval
                | SessionPhase::AwaitingChildren
                | SessionPhase::Paused => break,
                SessionPhase::Idle => {}
            }

            let decision = self
                .strategy
                .decide(LoopContext::new(
                    self.provider.info(),
                    self.config.clone(),
                    snapshot,
                ))
                .await?;

            match decision {
                LoopDecision::RunProvider => {
                    self.spawn_provider_step().await?;
                    break;
                }
                LoopDecision::ExecuteToolBatch => {
                    if self.spawn_next_tool_call().await? {
                        break;
                    }
                }
                LoopDecision::RequestToolApproval { request } => {
                    {
                        let mut state = self.state.lock().await;
                        if state.pending_approval.is_none() {
                            state.pending_approval = Some(request.clone());
                        }
                    }
                    self.emit(RuntimeEvent::ApprovalRequested { request });
                    self.set_phase_and_boundary(
                        SessionPhase::AwaitingApproval,
                        Some(SessionBoundary::AwaitingApproval),
                    )
                    .await?;
                    break;
                }
                LoopDecision::WaitForInput => {
                    self.set_phase_and_boundary(
                        SessionPhase::Idle,
                        Some(SessionBoundary::AwaitingInput),
                    )
                    .await?;
                    break;
                }
                LoopDecision::FinishTurn => {
                    self.finish_turn().await?;
                    continue;
                }
                LoopDecision::SpawnAgent { request, wait } => {
                    let child_id = self.spawn_child_runtime(request).await?;
                    if let Some(wait) = wait {
                        self.start_wait(vec![child_id], wait).await?;
                        break;
                    }
                    continue;
                }
                LoopDecision::WaitForAgents { ids, wait } => {
                    if self.start_wait(ids, wait).await? {
                        break;
                    }
                }
                LoopDecision::SendAgentInput {
                    runtime_id,
                    input,
                    delivery,
                } => {
                    self.send_agent_input(runtime_id, input, delivery).await?;
                    continue;
                }
                LoopDecision::InterruptAgent { runtime_id, mode } => {
                    self.handle_agent_command(AgentCommand::InterruptAgent { runtime_id, mode })
                        .await?;
                    continue;
                }
                LoopDecision::PauseAgent { runtime_id } => {
                    self.handle_agent_command(AgentCommand::PauseAgent { runtime_id })
                        .await?;
                    continue;
                }
                LoopDecision::ResumeAgent { runtime_id } => {
                    self.handle_agent_command(AgentCommand::ResumeAgent { runtime_id })
                        .await?;
                    continue;
                }
                LoopDecision::CompactContext => {
                    let original_messages = self.state.lock().await.transcript.len();
                    self.emit(RuntimeEvent::Compaction {
                        original_messages,
                        summary_tokens: 0,
                    });
                    break;
                }
            }
        }
        Ok(())
    }

    async fn spawn_child_runtime(
        &mut self,
        request: SpawnRequest,
    ) -> Result<RuntimeId, RuntimeError> {
        self.emit(RuntimeEvent::ChildSpawnRequested {
            parent_id: self.runtime_id,
            label: request.label.clone(),
        });
        let child_id = SessionEngine {
            session_id: self.runtime_id,
            provider: self.provider.clone(),
            tools: self.tools.clone(),
            config: self.config.clone(),
            state: self.state.clone(),
            command_tx: self.command_tx.clone(),
            event_tx: self.event_tx.clone(),
            registry: self.registry.clone(),
        }
        .spawn_child_runtime_from_handle(request)
        .await?;
        let child = self.registry.handle(child_id)?;
        let child_snapshot = {
            let state = self.state.lock().await;
            state.children.get(&child_id).cloned().ok_or_else(|| {
                RuntimeError::Internal(format!("missing child snapshot: {child_id}"))
            })?
        };
        self.emit(RuntimeEvent::ChildSpawned {
            parent_id: self.runtime_id,
            child: child_snapshot.clone(),
        });
        self.watch_child(child, child_id);
        Ok(child_id)
    }

    fn watch_child(&self, child: RuntimeHandle, child_id: RuntimeId) {
        let parent_command = self.command_tx.clone();
        tokio::spawn(async move {
            let mut events = child.event_tx.subscribe();
            loop {
                let event = match events.recv().await {
                    Ok(event) => event,
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                };

                let update = match event {
                    RuntimeEvent::TurnStarted { .. } => Some(ChildUpdate::Status {
                        child_id,
                        status: ChildStatus::Running,
                    }),
                    RuntimeEvent::BoundaryReached {
                        boundary: SessionBoundary::AwaitingInput,
                    } => Some(ChildUpdate::Status {
                        child_id,
                        status: ChildStatus::AwaitingInput,
                    }),
                    RuntimeEvent::BoundaryReached {
                        boundary: SessionBoundary::Paused,
                    } => Some(ChildUpdate::Status {
                        child_id,
                        status: ChildStatus::Paused,
                    }),
                    RuntimeEvent::TurnFinished { .. } => {
                        let snapshot = child.state.lock().await.clone();
                        Some(ChildUpdate::Completed {
                            child_id,
                            result: ChildResult {
                                output: snapshot.transcript.clone(),
                                success: true,
                            },
                        })
                    }
                    RuntimeEvent::Error {
                        message,
                        recoverable: false,
                    } => Some(ChildUpdate::Failed {
                        child_id,
                        error: message,
                    }),
                    _ => None,
                };

                let Some(update) = update else {
                    continue;
                };
                if parent_command
                    .send(EngineCommand::ChildUpdate(update))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        });
    }

    async fn send_agent_input(
        &mut self,
        runtime_id: RuntimeId,
        input: Vec<Message>,
        delivery: InputDelivery,
    ) -> Result<(), RuntimeError> {
        let message_count = input.len();
        self.emit(RuntimeEvent::AgentInputQueued {
            from: self.runtime_id,
            to: runtime_id,
            delivery,
            message_count,
        });
        let payload = serde_json::to_value(RoutedAgentInput { input, delivery })
            .map_err(|error| RuntimeError::Internal(format!("invalid agent input: {error}")))?;
        let envelope = self
            .send_runtime_control(runtime_id, EnvelopeKind::Input, payload)
            .await?;
        self.emit(RuntimeEvent::AgentInputDelivered {
            from: envelope.from,
            to: envelope.to,
            delivery,
            message_count,
        });
        Ok(())
    }

    async fn send_agent_message(
        &mut self,
        runtime_id: RuntimeId,
        mut message: AgentMessage,
    ) -> Result<(), RuntimeError> {
        message.from_runtime_id = self.runtime_id;
        message.to_runtime_id = runtime_id;
        self.emit(RuntimeEvent::AgentMessageQueued {
            message: message.clone(),
        });
        let envelope = self
            .send_runtime_control(
                runtime_id,
                EnvelopeKind::Mail,
                serde_json::to_value(&message).map_err(|error| {
                    RuntimeError::Internal(format!("invalid agent message payload: {error}"))
                })?,
            )
            .await?;
        let delivered: AgentMessage =
            serde_json::from_value(envelope.payload).map_err(|error| {
                RuntimeError::Internal(format!("invalid delivered agent message payload: {error}"))
            })?;
        self.emit(RuntimeEvent::AgentMessageDelivered { message: delivered });
        Ok(())
    }

    async fn send_runtime_control(
        &mut self,
        runtime_id: RuntimeId,
        kind: EnvelopeKind,
        payload: serde_json::Value,
    ) -> Result<Envelope, RuntimeError> {
        let envelope = Envelope::new(self.runtime_id, runtime_id, kind, payload);
        self.emit(RuntimeEvent::EnvelopeQueued {
            envelope: envelope.clone(),
        });
        self.registry.route_envelope(envelope.clone()).await?;
        self.emit(RuntimeEvent::EnvelopeDelivered {
            envelope: envelope.clone(),
        });
        Ok(envelope)
    }

    async fn queue_agent_input(
        &mut self,
        from: RuntimeId,
        input: Vec<Message>,
        delivery: InputDelivery,
    ) -> Result<(), RuntimeError> {
        let message_count = input.len();
        let when = match delivery {
            InputDelivery::NextSafeBoundary => SteerWhen::NextSafeBoundary,
            InputDelivery::AfterCurrentTool => SteerWhen::AfterCurrentTool,
        };
        let mut state = self.state.lock().await;
        for message in input {
            state
                .pending_steering
                .push(PendingSteering { message, when });
        }
        self.emit(RuntimeEvent::AgentInputDelivered {
            from,
            to: self.runtime_id,
            delivery,
            message_count,
        });
        Ok(())
    }

    async fn build_child_report_from_envelope(
        &self,
        envelope: &Envelope,
    ) -> Result<Option<ChildReport>, RuntimeError> {
        let kind = match envelope.kind {
            EnvelopeKind::Progress => ChildReportKind::Progress,
            EnvelopeKind::Result => ChildReportKind::Result,
            EnvelopeKind::Mail
                if serde_json::from_value::<AgentMessage>(envelope.payload.clone()).is_err() =>
            {
                ChildReportKind::Observation
            }
            _ => return Ok(None),
        };
        let child_label = {
            let state = self.state.lock().await;
            state
                .children
                .get(&envelope.from)
                .and_then(|child| child.label.clone())
        };
        Ok(Some(ChildReport {
            child_id: envelope.from,
            child_label,
            kind,
            payload: envelope.payload.clone(),
            is_final: matches!(envelope.kind, EnvelopeKind::Result),
        }))
    }

    async fn read_agent_messages(&self, filter: AgentMessageFilter) -> Vec<AgentMessage> {
        let mut state = self.state.lock().await;
        let mut messages = state
            .mailbox
            .iter_mut()
            .filter(|message| message_matches_filter(message, &filter))
            .map(|message| {
                if message.read_at.is_none() {
                    message.read_at = Some(now_millis());
                }
                message.clone()
            })
            .collect::<Vec<_>>();
        if let Some(limit) = filter.limit {
            messages.truncate(limit);
        }
        messages
    }

    async fn apply_pending_child_reports(&mut self) -> Result<bool, RuntimeError> {
        let reports = {
            let mut state = self.state.lock().await;
            if !matches!(state.phase, SessionPhase::Idle) || state.pending_child_reports.is_empty()
            {
                return Ok(false);
            }
            std::mem::take(&mut state.pending_child_reports)
        };

        for report in reports {
            let message = render_child_report_message(&report)?;
            self.commit_messages(vec![message.clone()]).await?;
            self.emit(RuntimeEvent::ChildReportInjected { report, message });
        }
        Ok(true)
    }

    async fn apply_pending_direct_messages(&mut self) -> Result<bool, RuntimeError> {
        let messages = {
            let mut state = self.state.lock().await;
            if !matches!(state.phase, SessionPhase::Idle)
                || state.pending_direct_messages.is_empty()
            {
                return Ok(false);
            }
            std::mem::take(&mut state.pending_direct_messages)
        };

        for message in messages {
            let rendered = render_agent_message_message(&message)?;
            self.commit_messages(vec![rendered]).await?;
        }
        Ok(true)
    }

    async fn start_wait(
        &mut self,
        ids: Vec<RuntimeId>,
        wait: WaitRequest,
    ) -> Result<bool, RuntimeError> {
        let ids = ids
            .into_iter()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        if ids.is_empty() {
            return Ok(false);
        }

        let should_wait = {
            let state = self.state.lock().await;
            ids.iter().any(|id| {
                state
                    .children
                    .get(id)
                    .map(|child| !child.status.is_terminal())
                    .unwrap_or(false)
            })
        };
        if !should_wait {
            return Ok(false);
        }

        {
            let mut state = self.state.lock().await;
            state.active_wait = Some(ActiveWait {
                ids: ids.clone(),
                request: wait.clone(),
            });
        }
        self.set_phase_and_boundary(
            SessionPhase::AwaitingChildren,
            Some(SessionBoundary::AwaitingChildren),
        )
        .await?;

        if let Some(timeout_ms) = wait.timeout_ms {
            let command_tx = self.command_tx.clone();
            let state = self.state.clone();
            let runtime_id = self.runtime_id;
            let wait_request = wait.clone();
            tokio::spawn(async move {
                sleep(Duration::from_millis(timeout_ms)).await;
                let active_wait = state.lock().await.active_wait.clone();
                let Some(active_wait) = active_wait else {
                    return;
                };
                if active_wait.ids != ids || active_wait.request != wait_request {
                    return;
                }
                let result = WaitResult {
                    outcome: WaitOutcome::TimedOut,
                    targets: ids
                        .iter()
                        .copied()
                        .map(|runtime_id| WaitTargetOutcome {
                            runtime_id,
                            outcome: WaitOutcome::TimedOut,
                        })
                        .collect(),
                };
                let _ = command_tx.send(EngineCommand::WaitTimedOut(result)).await;
                let _ = runtime_id;
            });
        }

        Ok(true)
    }

    async fn resume_from_wait_if_ready(&mut self) -> Result<(), RuntimeError> {
        let should_resume = {
            let state = self.state.lock().await;
            let Some(wait) = &state.active_wait else {
                return Ok(());
            };
            wait.ids.iter().all(|id| {
                state
                    .children
                    .get(id)
                    .map(|child| child.status.is_terminal())
                    .unwrap_or(true)
            })
        };

        if should_resume {
            {
                let mut state = self.state.lock().await;
                state.active_wait = None;
            }
            self.set_phase_and_boundary(SessionPhase::Idle, None)
                .await?;
        }

        Ok(())
    }

    async fn handle_wait_timed_out(&mut self, result: WaitResult) -> Result<(), RuntimeError> {
        let active_wait = {
            let mut state = self.state.lock().await;
            state.active_wait.take()
        };
        let Some(active_wait) = active_wait else {
            return Ok(());
        };

        if active_wait.request.on_timeout == WaitTimeoutAction::InterruptChild {
            for target in &active_wait.ids {
                let _ = self
                    .send_runtime_control(
                        *target,
                        EnvelopeKind::Interrupt,
                        serde_json::to_value(InterruptMode::ImmediateCancel).map_err(|error| {
                            RuntimeError::Internal(format!(
                                "invalid timeout interrupt payload: {error}"
                            ))
                        })?,
                    )
                    .await?;
                self.emit(RuntimeEvent::AgentInterrupted {
                    from: self.runtime_id,
                    to: *target,
                    mode: InterruptMode::ImmediateCancel,
                });
            }
        }

        self.set_phase_and_boundary(SessionPhase::Idle, None)
            .await?;
        self.emit(RuntimeEvent::AgentWaitTimedOut {
            runtime_id: self.runtime_id,
            wait: active_wait.request,
            result,
        });
        Ok(())
    }

    async fn list_visible_agents(&self, scope: AgentListScope) -> Vec<ChildRuntimeState> {
        match scope {
            AgentListScope::DirectChildren => self.children_snapshot().await,
        }
    }

    async fn children_snapshot(&self) -> Vec<ChildRuntimeState> {
        let state = self.state.lock().await;
        state.children.values().cloned().collect()
    }

    async fn begin_turn_if_needed(&mut self) -> Result<(), RuntimeError> {
        let pending_inputs = {
            let mut state = self.state.lock().await;
            if state.active_turn
                || state.pending_inputs.is_empty()
                || matches!(
                    state.phase,
                    SessionPhase::Paused
                        | SessionPhase::RunningProvider
                        | SessionPhase::RunningTool
                        | SessionPhase::AwaitingApproval
                        | SessionPhase::AwaitingChildren
                )
            {
                return Ok(());
            }

            state.turn_index = state.turn_index.saturating_add(1);
            state.iteration_count = 0;
            state.active_turn = true;
            state.pending_completion = false;
            state.last_finish_reason = None;
            state.pending_tool_calls.clear();
            state.pending_approval = None;

            let session_id = state.session_id;
            let turn_index = state.turn_index;
            let pending_inputs = std::mem::take(&mut state.pending_inputs);
            state.boundary = None;

            self.emit(RuntimeEvent::TurnStarted {
                session_id,
                turn_index,
            });
            pending_inputs
        };

        let messages = pending_inputs
            .into_iter()
            .flat_map(|pending| pending.input)
            .collect::<Vec<_>>();
        if !messages.is_empty() {
            self.commit_messages(messages).await?;
        }

        Ok(())
    }

    async fn apply_queued_steering(&mut self) -> Result<bool, RuntimeError> {
        let steering = {
            let mut state = self.state.lock().await;
            if !matches!(state.phase, SessionPhase::Idle) || state.pending_steering.is_empty() {
                return Ok(false);
            }

            let should_delay_for_tools =
                state.active_tool_call.is_some() || !state.pending_tool_calls.is_empty();
            let mut apply = Vec::new();
            let mut retain = Vec::new();
            for item in std::mem::take(&mut state.pending_steering) {
                let ready = match item.when {
                    SteerWhen::NextSafeBoundary => true,
                    SteerWhen::AfterCurrentTool => !should_delay_for_tools,
                };
                if ready {
                    apply.push(item);
                } else {
                    retain.push(item);
                }
            }
            state.pending_steering = retain;
            apply
        };

        if steering.is_empty() {
            return Ok(false);
        }

        for item in steering {
            self.commit_messages(vec![item.message.clone()]).await?;
            self.emit(RuntimeEvent::SteeringApplied {
                message: item.message,
                when: item.when,
            });
        }
        Ok(true)
    }

    async fn apply_pending_interrupt_if_ready(&mut self) -> Result<bool, RuntimeError> {
        let mode = {
            let mut state = self.state.lock().await;
            if !matches!(state.phase, SessionPhase::Idle) {
                return Ok(false);
            }

            let Some(mode) = state.pending_interrupt else {
                return Ok(false);
            };
            if matches!(mode, InterruptMode::AfterCurrentTool) && state.active_tool_call.is_some() {
                return Ok(false);
            }

            state.pending_interrupt = None;
            Some(mode)
        };

        let Some(mode) = mode else {
            return Ok(false);
        };

        self.emit(RuntimeEvent::Interrupted { mode });
        self.set_phase_and_boundary(SessionPhase::Paused, Some(SessionBoundary::Paused))
            .await?;
        Ok(true)
    }

    async fn spawn_provider_step(&mut self) -> Result<(), RuntimeError> {
        let request = {
            let mut state = self.state.lock().await;
            state.iteration_count = state.iteration_count.saturating_add(1);
            Request {
                model: self.config.model.clone(),
                messages: state.transcript.clone(),
                tools: merge_tool_definitions(
                    self.tools.definitions(),
                    native_agent_tool_definitions(),
                ),
                options: self.config.request.clone(),
            }
        };

        let cancel = CancellationToken::new();
        self.active_provider_cancel = Some(cancel.clone());
        self.set_phase_and_boundary(SessionPhase::RunningProvider, None)
            .await?;

        let provider = self.provider.clone();
        let config = self.config.clone();
        let event_tx = self.event_tx.clone();
        let command_tx = self.command_tx.clone();
        tokio::spawn(async move {
            let result = run_provider_step(provider, config, request, Some(event_tx), cancel).await;
            let _ = command_tx
                .send(EngineCommand::ProviderFinished(result))
                .await;
        });

        Ok(())
    }

    async fn spawn_next_tool_call(&mut self) -> Result<bool, RuntimeError> {
        let call = {
            let mut state = self.state.lock().await;
            if state.pending_tool_calls.is_empty() {
                return Ok(false);
            }

            let call = state.pending_tool_calls.remove(0);
            state.active_tool_call = Some(call.clone());
            call
        };

        self.emit(RuntimeEvent::ToolCallStarted { call: call.clone() });
        self.set_phase_and_boundary(SessionPhase::RunningTool, None)
            .await?;

        let tools = self.tools.clone();
        let engine = SessionEngine {
            session_id: self.runtime_id,
            provider: self.provider.clone(),
            tools: self.tools.clone(),
            config: self.config.clone(),
            state: self.state.clone(),
            command_tx: self.command_tx.clone(),
            event_tx: self.event_tx.clone(),
            registry: self.registry.clone(),
        };
        let command_tx = self.command_tx.clone();
        tokio::spawn(async move {
            let result = if is_native_agent_tool(&call.name) {
                execute_native_agent_tool(engine, call.clone()).await
            } else {
                tools.execute(call.clone()).await
            };
            let _ = command_tx
                .send(EngineCommand::ToolFinished { call, result })
                .await;
        });
        Ok(true)
    }

    async fn finish_turn(&mut self) -> Result<(), RuntimeError> {
        let finished = {
            let mut state = self.state.lock().await;
            if !state.active_turn {
                return Ok(());
            }

            let finished = (
                state.session_id,
                state.turn_index,
                state.last_finish_reason.clone(),
            );
            state.active_turn = false;
            state.iteration_count = 0;
            state.pending_completion = false;
            state.pending_tool_calls.clear();
            state.active_tool_call = None;
            state.pending_approval = None;
            finished
        };

        self.set_phase_and_boundary(SessionPhase::Idle, Some(SessionBoundary::TurnFinished))
            .await?;
        self.emit(RuntimeEvent::TurnFinished {
            session_id: finished.0,
            turn_index: finished.1,
            finish_reason: finished.2,
        });
        Ok(())
    }

    async fn commit_messages(&self, messages: Vec<Message>) -> Result<(), RuntimeError> {
        {
            let mut state = self.state.lock().await;
            state.transcript.extend(messages.iter().cloned());
        }
        for message in messages {
            self.emit(RuntimeEvent::MessageCommitted { message });
        }
        Ok(())
    }

    fn build_approval_request(&self, calls: &[ToolCall]) -> Option<ApprovalRequest> {
        let mut gated_calls = Vec::new();
        let mut reason = None;

        for call in calls {
            let approval = self.tools.approval(call);
            let Some(ToolApproval {
                reason: approval_reason,
            }) = approval
            else {
                continue;
            };
            if reason.is_none() {
                reason = approval_reason;
            }
            gated_calls.push(call.clone());
        }

        (!gated_calls.is_empty()).then(|| ApprovalRequest {
            id: Ulid::new().to_string(),
            calls: gated_calls,
            reason,
        })
    }

    async fn set_phase_and_boundary(
        &self,
        phase: SessionPhase,
        boundary: Option<SessionBoundary>,
    ) -> Result<(), RuntimeError> {
        let (phase_changed, boundary_changed) = {
            let mut state = self.state.lock().await;
            let phase_changed = (state.phase != phase).then(|| {
                state.phase = phase;
                phase
            });
            let boundary_changed = (state.boundary != boundary).then(|| {
                state.boundary = boundary;
                boundary
            });
            (phase_changed, boundary_changed)
        };

        if let Some(phase) = phase_changed {
            self.emit(RuntimeEvent::PhaseChanged { phase });
        }
        if let Some(Some(boundary)) = boundary_changed {
            self.emit(RuntimeEvent::BoundaryReached { boundary });
        }

        Ok(())
    }

    fn emit(&self, event: RuntimeEvent) {
        let _ = self.event_tx.send(event);
    }
}

const SPAWN_AGENT_TOOL: &str = "spawn_agent";
const MESSAGE_AGENT_TOOL: &str = "message_agent";
const READ_AGENT_MAIL_TOOL: &str = "read_agent_mail";
const INTERRUPT_AGENT_TOOL: &str = "interrupt_agent";
const LIST_AGENTS_TOOL: &str = "list_agents";
const WAIT_AGENT_TOOL: &str = "wait_agent";

#[derive(Debug, Deserialize)]
struct SpawnAgentToolInput {
    #[serde(default)]
    label: Option<String>,
    task: String,
    #[serde(default)]
    spawn_mode: Option<SpawnMode>,
    #[serde(default)]
    result_mode: Option<ResultMode>,
    #[serde(default)]
    history_mode: Option<HistoryMode>,
    #[serde(default)]
    profile: Option<String>,
    #[serde(default)]
    model_override: Option<String>,
    #[serde(default)]
    wait: Option<WaitRequest>,
}

#[derive(Debug, Deserialize)]
struct MessageAgentToolInput {
    agent_id: RuntimeId,
    message: String,
    #[serde(default)]
    delivery_mode: Option<AgentMessageDelivery>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    kind: Option<AgentMessageKind>,
}

#[derive(Debug, Deserialize)]
struct InterruptAgentToolInput {
    agent_id: RuntimeId,
    #[serde(default)]
    mode: Option<InterruptMode>,
}

#[derive(Debug, Deserialize)]
struct WaitAgentToolInput {
    agent_ids: Vec<RuntimeId>,
    #[serde(default)]
    timeout_ms: Option<u64>,
    #[serde(default)]
    on_timeout: Option<WaitTimeoutAction>,
}

#[derive(Debug, Deserialize)]
struct ReadAgentMailToolInput {
    #[serde(default)]
    from_agent_id: Option<RuntimeId>,
    #[serde(default)]
    delivery_mode: Option<AgentMessageDelivery>,
    #[serde(default)]
    kind: Option<AgentMessageKind>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    unread_only: bool,
    #[serde(default)]
    since: Option<u64>,
    #[serde(default)]
    limit: Option<usize>,
}

fn native_agent_tool_definitions() -> Vec<provider::ToolDefinition> {
    vec![
        provider::ToolDefinition::new(
            SPAWN_AGENT_TOOL,
            "Spawn a new agent runtime. By default this is background and non-blocking. Optionally include a wait object to hold until completion or timeout.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "task": { "type": "string" },
                    "label": { "type": "string" },
                    "spawn_mode": { "type": "string", "enum": ["await_completion", "concurrent", "background"] },
                    "result_mode": { "type": "string", "enum": ["final_result_only", "mailbox"] },
                    "history_mode": { "type": "string", "enum": ["empty", "fork_parent_transcript"] },
                    "profile": { "type": "string" },
                    "model_override": { "type": "string" },
                    "wait": {
                        "type": "object",
                        "properties": {
                            "timeout_ms": { "type": "integer", "minimum": 0 },
                            "on_timeout": { "type": "string", "enum": ["release_hold", "interrupt_child"] }
                        }
                    }
                },
                "required": ["task"]
            }),
        ),
        provider::ToolDefinition::new(
            MESSAGE_AGENT_TOOL,
            "Send a direct or mailbox-style message to an existing agent runtime by runtime id.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "agent_id": { "type": "string" },
                    "message": { "type": "string" },
                    "delivery_mode": { "type": "string", "enum": ["direct", "mail"] },
                    "title": { "type": "string" },
                    "tags": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "kind": { "type": "string", "enum": ["message", "question", "observation", "result", "progress"] }
                },
                "required": ["agent_id", "message"]
            }),
        ),
        provider::ToolDefinition::new(
            INTERRUPT_AGENT_TOOL,
            "Interrupt an existing agent runtime by runtime id.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "agent_id": { "type": "string" },
                    "mode": { "type": "string", "enum": ["immediate_cancel", "after_current_tool", "pause_at_boundary"] }
                },
                "required": ["agent_id"]
            }),
        ),
        provider::ToolDefinition::new(
            LIST_AGENTS_TOOL,
            "List visible direct child agents for the current runtime.",
            serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        ),
        provider::ToolDefinition::new(
            READ_AGENT_MAIL_TOOL,
            "Read stored direct or mailbox-style agent messages using optional filters.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "from_agent_id": { "type": "string" },
                    "delivery_mode": { "type": "string", "enum": ["direct", "mail"] },
                    "kind": { "type": "string", "enum": ["message", "question", "observation", "result", "progress"] },
                    "tags": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "unread_only": { "type": "boolean" },
                    "since": { "type": "integer", "minimum": 0 },
                    "limit": { "type": "integer", "minimum": 1 }
                }
            }),
        ),
        provider::ToolDefinition::new(
            WAIT_AGENT_TOOL,
            "Wait on one or more agent runtime ids using non-destructive defaults unless on_timeout says otherwise.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "agent_ids": {
                        "type": "array",
                        "items": { "type": "string" },
                        "minItems": 1
                    },
                    "timeout_ms": { "type": "integer", "minimum": 0 },
                    "on_timeout": { "type": "string", "enum": ["release_hold", "interrupt_child"] }
                },
                "required": ["agent_ids"]
            }),
        ),
    ]
}

fn merge_tool_definitions(
    mut application_tools: Vec<provider::ToolDefinition>,
    native_tools: Vec<provider::ToolDefinition>,
) -> Vec<provider::ToolDefinition> {
    application_tools.extend(native_tools);
    application_tools
}

fn is_native_agent_tool(name: &str) -> bool {
    matches!(
        name,
        SPAWN_AGENT_TOOL
            | MESSAGE_AGENT_TOOL
            | READ_AGENT_MAIL_TOOL
            | INTERRUPT_AGENT_TOOL
            | LIST_AGENTS_TOOL
            | WAIT_AGENT_TOOL
    )
}

async fn execute_native_agent_tool(
    engine: SessionEngine,
    call: ToolCall,
) -> Result<ToolExecutionResult, RuntimeError> {
    match call.name.as_str() {
        SPAWN_AGENT_TOOL => {
            let input: SpawnAgentToolInput =
                serde_json::from_value(call.input).map_err(|error| {
                    RuntimeError::Tool(format!("invalid spawn_agent input: {error}"))
                })?;
            let request = SpawnRequest {
                label: input.label,
                initial_input: vec![Message::user_text(input.task)],
                spawn_mode: input.spawn_mode.unwrap_or(SpawnMode::Background),
                result_mode: input.result_mode.unwrap_or(ResultMode::FinalResultOnly),
                history_mode: input.history_mode.unwrap_or(HistoryMode::Empty),
                profile: input.profile,
                model_override: input.model_override,
                ..SpawnRequest::default()
            };
            let child_id = engine.spawn_child_tool(request).await?;
            let wait_result = if let Some(wait) = input.wait {
                Some(wait_for_runtime_targets(engine.clone(), vec![child_id], wait).await?)
            } else {
                None
            };
            Ok(ToolExecutionResult::success(serde_json::json!({
                "agent_id": child_id,
                "status": "spawned",
                "wait": wait_result,
            })))
        }
        MESSAGE_AGENT_TOOL => {
            let input: MessageAgentToolInput =
                serde_json::from_value(call.input).map_err(|error| {
                    RuntimeError::Tool(format!("invalid message_agent input: {error}"))
                })?;
            let mut message = AgentMessage::new(
                engine.session_id(),
                input.agent_id,
                input.delivery_mode.unwrap_or(AgentMessageDelivery::Mail),
                input.kind.unwrap_or(AgentMessageKind::Message),
                serde_json::json!({ "message": input.message }),
            );
            message.title = input.title;
            message.tags = input.tags;
            engine
                .submit(SessionCommand::Agent(AgentCommand::SendAgentMessage {
                    runtime_id: input.agent_id,
                    message,
                }))
                .await?;
            Ok(ToolExecutionResult::success(serde_json::json!({
                "agent_id": input.agent_id,
                "status": "queued"
            })))
        }
        READ_AGENT_MAIL_TOOL => {
            let input: ReadAgentMailToolInput =
                serde_json::from_value(call.input).map_err(|error| {
                    RuntimeError::Tool(format!("invalid read_agent_mail input: {error}"))
                })?;
            let messages = read_messages_from_engine(
                &engine,
                AgentMessageFilter {
                    from_runtime_id: input.from_agent_id,
                    delivery_mode: input.delivery_mode,
                    kind: input.kind,
                    tags: input.tags,
                    unread_only: input.unread_only,
                    since: input.since,
                    limit: input.limit,
                },
            )
            .await;
            Ok(ToolExecutionResult::success(
                serde_json::to_value(messages).map_err(|error| {
                    RuntimeError::Tool(format!("invalid read_agent_mail output: {error}"))
                })?,
            ))
        }
        INTERRUPT_AGENT_TOOL => {
            let input: InterruptAgentToolInput =
                serde_json::from_value(call.input).map_err(|error| {
                    RuntimeError::Tool(format!("invalid interrupt_agent input: {error}"))
                })?;
            let mode = input.mode.unwrap_or(InterruptMode::PauseAtBoundary);
            engine
                .submit(SessionCommand::Agent(AgentCommand::InterruptAgent {
                    runtime_id: input.agent_id,
                    mode,
                }))
                .await?;
            Ok(ToolExecutionResult::success(serde_json::json!({
                "agent_id": input.agent_id,
                "status": "queued",
                "mode": mode,
            })))
        }
        LIST_AGENTS_TOOL => Ok(ToolExecutionResult::success(
            serde_json::to_value(engine.children().await).map_err(|error| {
                RuntimeError::Tool(format!("invalid list_agents output: {error}"))
            })?,
        )),
        WAIT_AGENT_TOOL => {
            let input: WaitAgentToolInput =
                serde_json::from_value(call.input).map_err(|error| {
                    RuntimeError::Tool(format!("invalid wait_agent input: {error}"))
                })?;
            let wait = WaitRequest {
                timeout_ms: input.timeout_ms,
                on_timeout: input.on_timeout.unwrap_or(WaitTimeoutAction::ReleaseHold),
            };
            let result = wait_for_runtime_targets(engine, input.agent_ids, wait).await?;
            Ok(ToolExecutionResult::success(
                serde_json::to_value(result)
                    .map_err(|error| RuntimeError::Tool(format!("invalid wait result: {error}")))?,
            ))
        }
        _ => Err(RuntimeError::Tool(format!(
            "unknown native runtime tool: {}",
            call.name
        ))),
    }
}

async fn wait_for_runtime_targets(
    engine: SessionEngine,
    ids: Vec<RuntimeId>,
    wait: WaitRequest,
) -> Result<WaitResult, RuntimeError> {
    let deadline = wait
        .timeout_ms
        .map(|timeout_ms| tokio::time::Instant::now() + Duration::from_millis(timeout_ms));

    loop {
        let snapshot = engine.snapshot().await;
        let mut targets = Vec::new();
        let mut all_terminal = true;
        let mut any_failed = false;
        for runtime_id in &ids {
            let outcome = match snapshot.children.get(runtime_id).map(|child| child.status) {
                Some(ChildStatus::Completed) => WaitOutcome::Completed,
                Some(ChildStatus::Failed | ChildStatus::Cancelled) => {
                    any_failed = true;
                    WaitOutcome::Failed
                }
                Some(ChildStatus::Paused) => {
                    all_terminal = false;
                    WaitOutcome::Interrupted
                }
                Some(_) => {
                    all_terminal = false;
                    WaitOutcome::TimedOut
                }
                None => {
                    any_failed = true;
                    WaitOutcome::Failed
                }
            };
            targets.push(WaitTargetOutcome {
                runtime_id: *runtime_id,
                outcome,
            });
        }

        if all_terminal {
            let outcome = if any_failed {
                WaitOutcome::Failed
            } else {
                WaitOutcome::Completed
            };
            return Ok(WaitResult { outcome, targets });
        }

        if let Some(deadline) = deadline {
            if tokio::time::Instant::now() >= deadline {
                if wait.on_timeout == WaitTimeoutAction::InterruptChild {
                    for runtime_id in &ids {
                        engine
                            .submit(SessionCommand::Agent(AgentCommand::InterruptAgent {
                                runtime_id: *runtime_id,
                                mode: InterruptMode::ImmediateCancel,
                            }))
                            .await?;
                    }
                    return Ok(WaitResult {
                        outcome: WaitOutcome::Interrupted,
                        targets: ids
                            .into_iter()
                            .map(|runtime_id| WaitTargetOutcome {
                                runtime_id,
                                outcome: WaitOutcome::Interrupted,
                            })
                            .collect(),
                    });
                }
                return Ok(WaitResult {
                    outcome: WaitOutcome::TimedOut,
                    targets: ids
                        .into_iter()
                        .map(|runtime_id| WaitTargetOutcome {
                            runtime_id,
                            outcome: WaitOutcome::TimedOut,
                        })
                        .collect(),
                });
            }
        }

        sleep(Duration::from_millis(25)).await;
    }
}

async fn read_messages_from_engine(
    engine: &SessionEngine,
    filter: AgentMessageFilter,
) -> Vec<AgentMessage> {
    let mut state = engine.state.lock().await;
    let mut messages = state
        .mailbox
        .iter_mut()
        .filter(|message| message_matches_filter(message, &filter))
        .map(|message| {
            if message.read_at.is_none() {
                message.read_at = Some(now_millis());
            }
            message.clone()
        })
        .collect::<Vec<_>>();
    if let Some(limit) = filter.limit {
        messages.truncate(limit);
    }
    messages
}

fn render_child_report_message(report: &ChildReport) -> Result<Message, RuntimeError> {
    let payload = serde_json::json!({
        "child_id": report.child_id,
        "child_label": report.child_label,
        "kind": report.kind,
        "is_final": report.is_final,
        "payload": report.payload,
    });
    Ok(Message::new(
        MessageRole::Developer,
        vec![provider::ContentBlock::Text {
            text: format!(
                "<subagent_report>{}</subagent_report>",
                serde_json::to_string(&payload).map_err(|error| {
                    RuntimeError::Internal(format!("invalid child report payload: {error}"))
                })?
            ),
        }],
    ))
}

fn render_agent_message_message(message: &AgentMessage) -> Result<Message, RuntimeError> {
    let payload = serde_json::json!({
        "message_id": message.message_id,
        "from_runtime_id": message.from_runtime_id,
        "delivery_mode": message.delivery_mode,
        "kind": message.kind,
        "title": message.title,
        "tags": message.tags,
        "payload": message.payload,
        "thread_id": message.thread_id,
        "reply_to": message.reply_to,
    });
    Ok(Message::new(
        MessageRole::Developer,
        vec![provider::ContentBlock::Text {
            text: format!(
                "<agent_message>{}</agent_message>",
                serde_json::to_string(&payload).map_err(|error| {
                    RuntimeError::Internal(format!("invalid agent message payload: {error}"))
                })?
            ),
        }],
    ))
}

fn message_matches_filter(message: &AgentMessage, filter: &AgentMessageFilter) -> bool {
    if filter
        .from_runtime_id
        .is_some_and(|from_runtime_id| message.from_runtime_id != from_runtime_id)
    {
        return false;
    }
    if filter
        .delivery_mode
        .is_some_and(|delivery_mode| message.delivery_mode != delivery_mode)
    {
        return false;
    }
    if filter.kind.is_some_and(|kind| message.kind != kind) {
        return false;
    }
    if filter.unread_only && message.read_at.is_some() {
        return false;
    }
    if filter.since.is_some_and(|since| message.sent_at < since) {
        return false;
    }
    if !filter.tags.is_empty()
        && !message
            .tags
            .iter()
            .any(|tag| filter.tags.iter().any(|candidate| candidate == tag))
    {
        return false;
    }
    true
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

async fn run_provider_step(
    provider: Arc<dyn Provider>,
    config: Arc<RuntimeConfig>,
    request: Request,
    tx: Option<broadcast::Sender<RuntimeEvent>>,
    cancel: CancellationToken,
) -> Result<InferenceStep, RuntimeError> {
    let max_attempts = config.max_retries.saturating_add(1);

    'attempt: for attempt in 1..=max_attempts {
        if cancel.is_cancelled() {
            return Err(RuntimeError::Cancelled);
        }

        let mut stream = match provider.stream(&request).await {
            Ok(stream) => stream,
            Err(error) if attempt < max_attempts && is_retryable_provider_error(&error) => {
                if let Some(tx) = &tx {
                    let _ = tx.send(RuntimeEvent::Retry {
                        attempt,
                        max: max_attempts,
                        error: error.to_string(),
                    });
                }
                sleep(Duration::from_millis(config.retry_backoff_ms)).await;
                continue;
            }
            Err(error) => return Err(error.into()),
        };

        let mut accumulator = AssistantAccumulator::default();
        let mut usage = None;
        let mut finish_reason = None;
        let mut saw_output = false;

        while let Some(event) = stream.next().await {
            if cancel.is_cancelled() {
                return Err(RuntimeError::Cancelled);
            }

            match event {
                Ok(Event::ResponseStart { .. }) => {}
                Ok(Event::BlockStart { block }) => {
                    saw_output = true;
                    accumulator.block_start(block.clone());
                    if let Some(tx) = &tx {
                        let _ = tx.send(RuntimeEvent::OutputBlockStart { block });
                    }
                }
                Ok(Event::BlockDelta { id, delta }) => {
                    saw_output = true;
                    accumulator.block_delta(&id, &delta);
                    if let Some(tx) = &tx {
                        let _ = tx.send(RuntimeEvent::OutputBlockDelta {
                            id,
                            delta: delta.clone(),
                        });
                    }
                }
                Ok(Event::BlockStop { id }) => {
                    saw_output = true;
                    accumulator.block_stop(&id)?;
                    if let Some(tx) = &tx {
                        let _ = tx.send(RuntimeEvent::OutputBlockStop { id });
                    }
                }
                Ok(Event::Usage { usage: value }) => {
                    saw_output = true;
                    usage = Some(value.clone());
                    if let Some(tx) = &tx {
                        let _ = tx.send(RuntimeEvent::Usage { usage: value });
                    }
                }
                Ok(Event::Completed {
                    finish_reason: reason,
                    ..
                }) => {
                    finish_reason = reason;
                    break;
                }
                Err(error)
                    if !saw_output
                        && attempt < max_attempts
                        && is_retryable_provider_error(&error) =>
                {
                    if let Some(tx) = &tx {
                        let _ = tx.send(RuntimeEvent::Retry {
                            attempt,
                            max: max_attempts,
                            error: error.to_string(),
                        });
                    }
                    sleep(Duration::from_millis(config.retry_backoff_ms)).await;
                    continue 'attempt;
                }
                Err(error) => return Err(error.into()),
            }
        }

        let (message, tool_calls) = accumulator.finish()?;
        return Ok(InferenceStep {
            message,
            tool_calls,
            usage,
            finish_reason,
        });
    }

    Err(RuntimeError::Internal(
        "provider retry loop exhausted unexpectedly".into(),
    ))
}

#[derive(Default)]
struct AssistantAccumulator {
    order: usize,
    active: Vec<ActiveBlock>,
    finalized: Vec<(u32, usize, provider::ContentBlock)>,
    tool_calls: Vec<ToolCall>,
}

struct ActiveBlock {
    id: String,
    output_index: u32,
    sequence: usize,
    kind: BlockKind,
    text: String,
    json: String,
}

impl AssistantAccumulator {
    fn block_start(&mut self, block: Block) {
        let sequence = self.order;
        self.order = self.order.saturating_add(1);
        self.active.push(ActiveBlock {
            id: block.id,
            output_index: block.output_index,
            sequence,
            kind: block.kind,
            text: String::new(),
            json: String::new(),
        });
    }

    fn block_delta(&mut self, id: &str, delta: &BlockDelta) {
        if let Some(block) = self.active.iter_mut().find(|block| block.id == id) {
            match delta {
                BlockDelta::Text { text }
                | BlockDelta::Reasoning { text }
                | BlockDelta::Refusal { text } => block.text.push_str(text),
                BlockDelta::Json { partial_json } => block.json.push_str(partial_json),
                BlockDelta::Signature { signature } => block.text.push_str(signature),
                BlockDelta::Unknown { raw } => {
                    let _ = write!(&mut block.text, "{raw}");
                }
            }
        }
    }

    fn block_stop(&mut self, id: &str) -> Result<(), RuntimeError> {
        let Some(index) = self.active.iter().position(|block| block.id == id) else {
            return Ok(());
        };
        let block = self.active.remove(index);
        self.finalize_block(block)
    }

    fn finish(mut self) -> Result<(Option<Message>, Vec<ToolCall>), RuntimeError> {
        while let Some(block) = self.active.pop() {
            self.finalize_block(block)?;
        }
        self.finalized
            .sort_by_key(|(output_index, sequence, _)| (*output_index, *sequence));
        let content = self
            .finalized
            .into_iter()
            .map(|(_, _, block)| block)
            .collect::<Vec<_>>();

        let message = (!content.is_empty()).then_some(Message {
            role: MessageRole::Assistant,
            content,
        });

        Ok((message, self.tool_calls))
    }

    fn finalize_block(&mut self, block: ActiveBlock) -> Result<(), RuntimeError> {
        match block.kind {
            BlockKind::Text => {
                if !block.text.is_empty() {
                    self.finalized.push((
                        block.output_index,
                        block.sequence,
                        provider::ContentBlock::Text { text: block.text },
                    ));
                }
            }
            BlockKind::Reasoning => {
                if !block.text.is_empty() {
                    self.finalized.push((
                        block.output_index,
                        block.sequence,
                        provider::ContentBlock::Reasoning { text: block.text },
                    ));
                }
            }
            BlockKind::Refusal => {
                if !block.text.is_empty() {
                    self.finalized.push((
                        block.output_index,
                        block.sequence,
                        provider::ContentBlock::Refusal { text: block.text },
                    ));
                }
            }
            BlockKind::ToolCall { name, call_id } => {
                let id = call_id.unwrap_or(block.id);
                let name = name.unwrap_or_else(|| "tool".into());
                let input = if block.json.trim().is_empty() {
                    serde_json::Value::Object(serde_json::Map::new())
                } else {
                    serde_json::from_str(&block.json)
                        .unwrap_or_else(|_| serde_json::Value::String(block.json.clone()))
                };
                self.tool_calls.push(ToolCall {
                    id: id.clone(),
                    name: name.clone(),
                    input: input.clone(),
                });
                self.finalized.push((
                    block.output_index,
                    block.sequence,
                    provider::ContentBlock::ToolCall { id, name, input },
                ));
            }
            BlockKind::Unknown { kind } => {
                let mut text = String::new();
                if !block.text.is_empty() {
                    text.push_str(&block.text);
                }
                if !block.json.is_empty() {
                    if !text.is_empty() {
                        text.push('\n');
                    }
                    text.push_str(&block.json);
                }
                if !text.is_empty() {
                    self.finalized.push((
                        block.output_index,
                        block.sequence,
                        provider::ContentBlock::Text {
                            text: format!("[{kind}] {text}"),
                        },
                    ));
                }
            }
        }
        Ok(())
    }
}

fn is_retryable_provider_error(error: &provider::Error) -> bool {
    matches!(
        error,
        provider::Error::Inference(_) | provider::Error::Remote(_)
    )
}

fn truncate_json_value(value: &serde_json::Value, max_bytes: usize) -> serde_json::Value {
    let rendered = value.to_string();
    if rendered.len() <= max_bytes {
        return value.clone();
    }

    serde_json::Value::String(truncate_utf8(&rendered, max_bytes))
}

fn truncate_utf8(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_owned();
    }

    let mut cut = max_bytes.min(value.len());
    while cut > 0 && !value.is_char_boundary(cut) {
        cut -= 1;
    }
    value[..cut].to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_stream::stream;
    use futures::future::BoxFuture;
    use provider::{EventStream, MockProvider, ProviderCapabilities, ToolDefinition, Usage};
    use tokio::time::{Duration, timeout};

    use crate::ResultMode;

    struct TestTools;

    impl ToolExecutor for TestTools {
        fn definitions(&self) -> Vec<ToolDefinition> {
            vec![ToolDefinition::new(
                "echo",
                "Echo tool",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "message": { "type": "string" }
                    },
                    "required": ["message"]
                }),
            )]
        }

        fn execute<'a>(
            &'a self,
            call: ToolCall,
        ) -> BoxFuture<'a, Result<ToolExecutionResult, RuntimeError>> {
            Box::pin(async move {
                Ok(ToolExecutionResult::success(serde_json::json!({
                    "echoed": call.input
                })))
            })
        }
    }

    struct PassiveLoop;

    impl LoopStrategy for PassiveLoop {
        fn name(&self) -> &'static str {
            "passive"
        }

        fn decide<'a>(
            &'a self,
            ctx: LoopContext,
        ) -> futures::future::BoxFuture<'a, Result<LoopDecision, RuntimeError>> {
            Box::pin(async move {
                if ctx.state().pending_completion {
                    return Ok(LoopDecision::FinishTurn);
                }
                if !ctx.state().active_turn {
                    return Ok(LoopDecision::WaitForInput);
                }
                Ok(LoopDecision::RunProvider)
            })
        }
    }

    struct SlowTextProvider;

    impl Provider for SlowTextProvider {
        fn stream<'a>(
            &'a self,
            _request: &'a Request,
        ) -> BoxFuture<'a, Result<provider::EventStream<'a>, provider::Error>> {
            Box::pin(async move {
                let stream = stream! {
                    yield Ok(Event::ResponseStart {
                        response_id: Some("slow".into()),
                        model: Some("slow-model".into()),
                    });
                    yield Ok(Event::BlockStart {
                        block: Block {
                            id: "text".into(),
                            output_index: 0,
                            kind: BlockKind::Text,
                            item_id: None,
                        },
                    });
                    sleep(Duration::from_millis(30)).await;
                    yield Ok(Event::BlockDelta {
                        id: "text".into(),
                        delta: BlockDelta::Text {
                            text: "hello".into(),
                        },
                    });
                    sleep(Duration::from_millis(50)).await;
                    yield Ok(Event::BlockStop {
                        id: "text".into(),
                    });
                    yield Ok(Event::Usage {
                        usage: Usage::with_totals(Some(1), Some(1)),
                    });
                    yield Ok(Event::Completed {
                        response_id: Some("slow".into()),
                        finish_reason: Some(FinishReason::Stop),
                    });
                };
                Ok(Box::pin(stream) as EventStream<'a>)
            })
        }

        fn info(&self) -> ProviderInfo {
            ProviderInfo {
                name: "slow".into(),
                default_model_id: Some("slow-model".into()),
                capabilities: ProviderCapabilities::text_only(),
                models: Vec::new(),
            }
        }
    }

    fn new_engine(provider: impl Provider + 'static) -> SessionEngine {
        SessionEngine::new(
            Arc::new(provider),
            Arc::new(TestTools),
            Arc::new(PassiveLoop),
            RuntimeConfig::default(),
            SessionState::new(Ulid::new()),
        )
    }

    async fn spawn_child(engine: &SessionEngine, request: SpawnRequest) -> RuntimeId {
        let mut events = engine.subscribe();
        engine
            .submit(SessionCommand::Agent(AgentCommand::SpawnAgent {
                request,
                wait: None,
            }))
            .await
            .unwrap();

        loop {
            let event = timeout(Duration::from_millis(500), events.recv())
                .await
                .unwrap()
                .unwrap();
            if let RuntimeEvent::ChildSpawned { child, .. } = event {
                return child.runtime_id;
            }
        }
    }

    async fn wait_for_child_status(
        engine: &SessionEngine,
        child_id: RuntimeId,
        predicate: impl Fn(ChildStatus) -> bool,
    ) -> ChildStatus {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
        loop {
            let snapshot = engine.snapshot().await;
            let status = snapshot.children.get(&child_id).unwrap().status;
            if predicate(status) {
                return status;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "timed out waiting for child status, last status: {status:?}"
            );
            sleep(Duration::from_millis(10)).await;
        }
    }

    #[tokio::test]
    async fn spawn_child_tracks_lineage_and_completion() {
        let engine = new_engine(MockProvider::new());
        let mut events = engine.subscribe();

        engine
            .submit(SessionCommand::Agent(AgentCommand::SpawnAgent {
                request: SpawnRequest {
                    label: Some("child".into()),
                    initial_input: vec![Message::user_text("hello child")],
                    ..SpawnRequest::default()
                },
                wait: None,
            }))
            .await
            .unwrap();

        let mut saw_completed = false;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
        while tokio::time::Instant::now() < deadline {
            let event = timeout(Duration::from_millis(250), events.recv())
                .await
                .unwrap()
                .unwrap();
            if matches!(event, RuntimeEvent::ChildCompleted { .. }) {
                saw_completed = true;
                break;
            }
        }

        assert!(saw_completed);
        let snapshot = engine.snapshot().await;
        assert_eq!(snapshot.children.len(), 1);
        let child = snapshot.children.values().next().unwrap();
        assert_eq!(child.status, ChildStatus::Completed);
    }

    #[tokio::test]
    async fn message_agent_routes_to_known_runtime_id() {
        let engine = new_engine(SlowTextProvider);
        let mut events = engine.subscribe();

        engine
            .submit(SessionCommand::Agent(AgentCommand::SpawnAgent {
                request: SpawnRequest {
                    label: Some("mail-child".into()),
                    result_mode: ResultMode::Mailbox,
                    ..SpawnRequest::default()
                },
                wait: None,
            }))
            .await
            .unwrap();

        let child_id = loop {
            let event = timeout(Duration::from_millis(250), events.recv())
                .await
                .unwrap()
                .unwrap();
            if let RuntimeEvent::ChildSpawned { child, .. } = event {
                break child.runtime_id;
            }
        };

        engine
            .submit(SessionCommand::Agent(AgentCommand::SendAgentInput {
                runtime_id: child_id,
                input: vec![Message::user_text("hello child")],
                delivery: InputDelivery::NextSafeBoundary,
            }))
            .await
            .unwrap();

        sleep(Duration::from_millis(50)).await;
        let child_state = engine
            .registry
            .handle(child_id)
            .unwrap()
            .state
            .lock()
            .await
            .clone();
        assert_eq!(child_state.inbox.len(), 1);
        assert_eq!(child_state.inbox.front().unwrap().from, engine.session_id());
    }

    #[tokio::test]
    async fn registry_can_route_to_any_known_runtime_id() {
        let engine = new_engine(MockProvider::new());

        engine
            .submit(SessionCommand::Agent(AgentCommand::SpawnAgent {
                request: SpawnRequest::default(),
                wait: None,
            }))
            .await
            .unwrap();
        engine
            .submit(SessionCommand::Agent(AgentCommand::SpawnAgent {
                request: SpawnRequest::default(),
                wait: None,
            }))
            .await
            .unwrap();
        sleep(Duration::from_millis(50)).await;

        let snapshot = engine.snapshot().await;
        let ids = snapshot.children.keys().copied().collect::<Vec<_>>();
        assert_eq!(ids.len(), 2);

        let left_handle = engine.registry.handle(ids[0]).unwrap();
        let result = left_handle
            .command_tx
            .send(EngineCommand::External(SessionCommand::Agent(
                AgentCommand::SendAgentInput {
                    runtime_id: ids[1],
                    input: vec![Message::user_text("sibling")],
                    delivery: InputDelivery::NextSafeBoundary,
                },
            )))
            .await;
        assert!(result.is_ok());
        sleep(Duration::from_millis(50)).await;

        let right_state = engine
            .registry
            .handle(ids[1])
            .unwrap()
            .state
            .lock()
            .await
            .clone();
        assert_eq!(right_state.inbox.len(), 1);
    }

    #[tokio::test]
    async fn direct_agent_messages_are_injected_at_safe_boundary() {
        let engine = new_engine(MockProvider::new());
        engine
            .submit(SessionCommand::Agent(AgentCommand::SpawnAgent {
                request: SpawnRequest::default(),
                wait: None,
            }))
            .await
            .unwrap();
        sleep(Duration::from_millis(50)).await;

        let child_id = engine
            .snapshot()
            .await
            .children
            .keys()
            .copied()
            .next()
            .unwrap();
        let child_handle = engine.registry.handle(child_id).unwrap();
        let message = AgentMessage::new(
            child_id,
            engine.session_id(),
            AgentMessageDelivery::Direct,
            AgentMessageKind::Message,
            serde_json::json!({ "message": "hello parent" }),
        );
        child_handle
            .command_tx
            .send(EngineCommand::External(SessionCommand::Agent(
                AgentCommand::SendAgentMessage {
                    runtime_id: engine.session_id(),
                    message,
                },
            )))
            .await
            .unwrap();

        sleep(Duration::from_millis(50)).await;
        let snapshot = engine.snapshot().await;
        assert!(
            snapshot
                .transcript
                .iter()
                .any(|message| message.plain_text_lossy().contains("agent_message"))
        );
    }

    #[tokio::test]
    async fn read_agent_mail_tool_returns_filtered_messages() {
        let engine = new_engine(MockProvider::new());
        engine
            .submit(SessionCommand::Agent(AgentCommand::SpawnAgent {
                request: SpawnRequest::default(),
                wait: None,
            }))
            .await
            .unwrap();
        sleep(Duration::from_millis(50)).await;

        let child_id = engine
            .snapshot()
            .await
            .children
            .keys()
            .copied()
            .next()
            .unwrap();
        let child_handle = engine.registry.handle(child_id).unwrap();
        let mut message = AgentMessage::new(
            child_id,
            engine.session_id(),
            AgentMessageDelivery::Mail,
            AgentMessageKind::Observation,
            serde_json::json!({ "message": "mailbox note" }),
        );
        message.tags = vec!["research".into()];
        child_handle
            .command_tx
            .send(EngineCommand::External(SessionCommand::Agent(
                AgentCommand::SendAgentMessage {
                    runtime_id: engine.session_id(),
                    message,
                },
            )))
            .await
            .unwrap();
        sleep(Duration::from_millis(50)).await;

        let result = execute_native_agent_tool(
            engine.clone(),
            ToolCall {
                id: "read-mail".into(),
                name: READ_AGENT_MAIL_TOOL.into(),
                input: serde_json::json!({
                    "delivery_mode": "mail",
                    "kind": "observation",
                    "tags": ["research"]
                }),
            },
        )
        .await
        .unwrap();

        let output = result.output.as_array().unwrap();
        assert_eq!(output.len(), 1);
        assert_eq!(output[0]["delivery_mode"], "mail");
    }

    #[tokio::test]
    async fn wait_for_runtime_targets_returns_completed_for_finished_child() {
        let engine = new_engine(SlowTextProvider);
        let child_id = spawn_child(
            &engine,
            SpawnRequest {
                initial_input: vec![Message::user_text("hello child")],
                ..SpawnRequest::default()
            },
        )
        .await;

        let result = wait_for_runtime_targets(
            engine,
            vec![child_id],
            WaitRequest {
                timeout_ms: Some(500),
                ..WaitRequest::default()
            },
        )
        .await
        .unwrap();

        assert_eq!(result.outcome, WaitOutcome::Completed);
        assert_eq!(result.targets.len(), 1);
        assert_eq!(result.targets[0].runtime_id, child_id);
        assert_eq!(result.targets[0].outcome, WaitOutcome::Completed);
    }

    #[tokio::test]
    async fn wait_for_runtime_targets_times_out_without_interrupting_child() {
        let engine = new_engine(SlowTextProvider);
        let child_id = spawn_child(
            &engine,
            SpawnRequest {
                initial_input: vec![Message::user_text("slow child")],
                ..SpawnRequest::default()
            },
        )
        .await;

        let result = wait_for_runtime_targets(
            engine.clone(),
            vec![child_id],
            WaitRequest {
                timeout_ms: Some(10),
                on_timeout: WaitTimeoutAction::ReleaseHold,
            },
        )
        .await
        .unwrap();

        assert_eq!(result.outcome, WaitOutcome::TimedOut);
        assert_eq!(result.targets[0].outcome, WaitOutcome::TimedOut);

        let final_status =
            wait_for_child_status(&engine, child_id, |status| status == ChildStatus::Completed)
                .await;
        assert_eq!(final_status, ChildStatus::Completed);
    }

    #[tokio::test]
    async fn wait_for_runtime_targets_interrupts_child_on_timeout() {
        let engine = new_engine(SlowTextProvider);
        let child_id = spawn_child(
            &engine,
            SpawnRequest {
                initial_input: vec![Message::user_text("interruptible child")],
                ..SpawnRequest::default()
            },
        )
        .await;

        let result = wait_for_runtime_targets(
            engine.clone(),
            vec![child_id],
            WaitRequest {
                timeout_ms: Some(10),
                on_timeout: WaitTimeoutAction::InterruptChild,
            },
        )
        .await
        .unwrap();

        assert_eq!(result.outcome, WaitOutcome::Interrupted);
        assert_eq!(result.targets[0].outcome, WaitOutcome::Interrupted);

        let final_status =
            wait_for_child_status(&engine, child_id, |status| status == ChildStatus::Paused).await;
        assert_eq!(final_status, ChildStatus::Paused);
    }

    #[tokio::test]
    async fn wait_for_agents_command_sets_and_clears_active_wait() {
        let engine = new_engine(SlowTextProvider);
        let child_id = spawn_child(
            &engine,
            SpawnRequest {
                initial_input: vec![Message::user_text("slow child")],
                ..SpawnRequest::default()
            },
        )
        .await;

        let mut events = engine.subscribe();
        engine
            .submit(SessionCommand::Agent(AgentCommand::WaitForAgents {
                ids: vec![child_id],
                wait: WaitRequest {
                    timeout_ms: Some(10),
                    on_timeout: WaitTimeoutAction::ReleaseHold,
                },
            }))
            .await
            .unwrap();

        let waiting_snapshot = loop {
            let snapshot = engine.snapshot().await;
            if snapshot.active_wait.is_some() {
                break snapshot;
            }
            let _ = timeout(Duration::from_millis(200), events.recv())
                .await
                .unwrap()
                .unwrap();
        };
        assert_eq!(waiting_snapshot.phase, SessionPhase::AwaitingChildren);
        assert_eq!(
            waiting_snapshot.boundary,
            Some(SessionBoundary::AwaitingChildren)
        );
        assert!(waiting_snapshot.active_wait.is_some());

        sleep(Duration::from_millis(40)).await;

        let resumed_snapshot = engine.snapshot().await;
        assert_eq!(resumed_snapshot.phase, SessionPhase::Idle);
        assert_eq!(resumed_snapshot.active_wait, None);
    }

    #[tokio::test]
    async fn spawn_agent_tool_can_wait_inline() {
        let engine = new_engine(SlowTextProvider);
        let result = execute_native_agent_tool(
            engine,
            ToolCall {
                id: "spawn-inline-wait".into(),
                name: SPAWN_AGENT_TOOL.into(),
                input: serde_json::json!({
                    "task": "hello child",
                    "wait": {
                        "timeout_ms": 500,
                        "on_timeout": "release_hold"
                    }
                }),
            },
        )
        .await
        .unwrap();

        assert_eq!(result.output["status"], "spawned");
        assert_eq!(result.output["wait"]["outcome"], "completed");
    }

    #[tokio::test]
    async fn list_agents_tool_returns_all_direct_children() {
        let engine = new_engine(MockProvider::new());
        let left = spawn_child(
            &engine,
            SpawnRequest {
                label: Some("left".into()),
                ..SpawnRequest::default()
            },
        )
        .await;
        let right = spawn_child(
            &engine,
            SpawnRequest {
                label: Some("right".into()),
                ..SpawnRequest::default()
            },
        )
        .await;

        let result = execute_native_agent_tool(
            engine,
            ToolCall {
                id: "list-agents".into(),
                name: LIST_AGENTS_TOOL.into(),
                input: serde_json::json!({}),
            },
        )
        .await
        .unwrap();

        let output = result.output.as_array().unwrap();
        assert_eq!(output.len(), 2);
        let ids = output
            .iter()
            .map(|child| child["runtime_id"].as_str().unwrap().to_owned())
            .collect::<Vec<_>>();
        assert!(ids.iter().any(|id| id == &left.to_string()));
        assert!(ids.iter().any(|id| id == &right.to_string()));
    }

    #[tokio::test]
    async fn child_completion_reports_are_injected_at_safe_boundary() {
        let engine = new_engine(SlowTextProvider);
        let child_id = spawn_child(
            &engine,
            SpawnRequest {
                label: Some("reporting-child".into()),
                initial_input: vec![Message::user_text("report back")],
                ..SpawnRequest::default()
            },
        )
        .await;

        wait_for_child_status(&engine, child_id, |status| status == ChildStatus::Completed).await;

        let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
        loop {
            let snapshot = engine.snapshot().await;
            if snapshot
                .transcript
                .iter()
                .any(|message| message.plain_text_lossy().contains("<subagent_report>"))
            {
                break;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "timed out waiting for child report injection"
            );
            sleep(Duration::from_millis(10)).await;
        }
    }

    #[tokio::test]
    async fn mail_messages_are_not_injected_into_parent_transcript() {
        let engine = new_engine(MockProvider::new());
        let child_id = spawn_child(&engine, SpawnRequest::default()).await;
        let child_handle = engine.registry.handle(child_id).unwrap();
        let message = AgentMessage::new(
            child_id,
            engine.session_id(),
            AgentMessageDelivery::Mail,
            AgentMessageKind::Observation,
            serde_json::json!({ "message": "save for later" }),
        );

        child_handle
            .command_tx
            .send(EngineCommand::External(SessionCommand::Agent(
                AgentCommand::SendAgentMessage {
                    runtime_id: engine.session_id(),
                    message,
                },
            )))
            .await
            .unwrap();

        sleep(Duration::from_millis(50)).await;

        let snapshot = engine.snapshot().await;
        assert_eq!(snapshot.mailbox.len(), 1);
        assert!(
            snapshot
                .transcript
                .iter()
                .all(|message| !message.plain_text_lossy().contains("<agent_message>"))
        );
    }

    #[tokio::test]
    async fn reading_messages_marks_them_as_read_and_unread_filter_excludes_them() {
        let engine = new_engine(MockProvider::new());
        let child_id = spawn_child(&engine, SpawnRequest::default()).await;
        let child_handle = engine.registry.handle(child_id).unwrap();
        let message = AgentMessage::new(
            child_id,
            engine.session_id(),
            AgentMessageDelivery::Mail,
            AgentMessageKind::Observation,
            serde_json::json!({ "message": "mailbox note" }),
        );

        child_handle
            .command_tx
            .send(EngineCommand::External(SessionCommand::Agent(
                AgentCommand::SendAgentMessage {
                    runtime_id: engine.session_id(),
                    message,
                },
            )))
            .await
            .unwrap();
        sleep(Duration::from_millis(50)).await;

        let unread = read_messages_from_engine(
            &engine,
            AgentMessageFilter {
                unread_only: true,
                ..AgentMessageFilter::default()
            },
        )
        .await;
        assert_eq!(unread.len(), 1);
        assert!(unread[0].read_at.is_some());

        let unread_again = read_messages_from_engine(
            &engine,
            AgentMessageFilter {
                unread_only: true,
                ..AgentMessageFilter::default()
            },
        )
        .await;
        assert!(unread_again.is_empty());

        let snapshot = engine.snapshot().await;
        assert!(snapshot.mailbox.front().unwrap().read_at.is_some());
    }

    #[tokio::test]
    async fn sending_input_to_unknown_runtime_returns_error_without_side_effects() {
        let engine = new_engine(MockProvider::new());
        let mut events = engine.subscribe();
        let unknown_runtime_id = Ulid::new();

        engine
            .submit(SessionCommand::Agent(AgentCommand::SendAgentInput {
                runtime_id: unknown_runtime_id,
                input: vec![Message::user_text("ghost message")],
                delivery: InputDelivery::NextSafeBoundary,
            }))
            .await
            .unwrap();

        let error_message = loop {
            let event = timeout(Duration::from_millis(500), events.recv())
                .await
                .unwrap()
                .unwrap();
            if let RuntimeEvent::Error { message, .. } = event {
                break message;
            }
        };

        let snapshot = engine.snapshot().await;
        assert!(snapshot.children.is_empty());
        assert!(snapshot.mailbox.is_empty());
        assert!(snapshot.transcript.is_empty());
        assert!(error_message.contains(&unknown_runtime_id.to_string()));
    }

    #[test]
    fn message_matches_filter_honors_sender_mode_kind_tags_unread_and_since() {
        let from_runtime_id = Ulid::new();
        let to_runtime_id = Ulid::new();
        let mut message = AgentMessage::new(
            from_runtime_id,
            to_runtime_id,
            AgentMessageDelivery::Mail,
            AgentMessageKind::Observation,
            serde_json::json!({ "message": "mailbox note" }),
        );
        message.sent_at = 500;
        message.tags = vec!["research".into(), "urgent".into()];

        assert!(message_matches_filter(
            &message,
            &AgentMessageFilter {
                from_runtime_id: Some(from_runtime_id),
                delivery_mode: Some(AgentMessageDelivery::Mail),
                kind: Some(AgentMessageKind::Observation),
                tags: vec!["research".into()],
                unread_only: true,
                since: Some(400),
                limit: None,
            }
        ));

        assert!(!message_matches_filter(
            &message,
            &AgentMessageFilter {
                from_runtime_id: Some(Ulid::new()),
                ..AgentMessageFilter::default()
            }
        ));
        assert!(!message_matches_filter(
            &message,
            &AgentMessageFilter {
                delivery_mode: Some(AgentMessageDelivery::Direct),
                ..AgentMessageFilter::default()
            }
        ));
        assert!(!message_matches_filter(
            &message,
            &AgentMessageFilter {
                kind: Some(AgentMessageKind::Result),
                ..AgentMessageFilter::default()
            }
        ));
        assert!(!message_matches_filter(
            &message,
            &AgentMessageFilter {
                tags: vec!["missing".into()],
                ..AgentMessageFilter::default()
            }
        ));
        assert!(!message_matches_filter(
            &message,
            &AgentMessageFilter {
                since: Some(600),
                ..AgentMessageFilter::default()
            }
        ));

        message.read_at = Some(700);
        assert!(!message_matches_filter(
            &message,
            &AgentMessageFilter {
                unread_only: true,
                ..AgentMessageFilter::default()
            }
        ));
    }

    #[test]
    fn render_helpers_wrap_payloads_in_developer_messages() {
        let report = ChildReport {
            child_id: Ulid::new(),
            child_label: Some("researcher".into()),
            kind: ChildReportKind::Observation,
            payload: serde_json::json!({ "summary": "found it" }),
            is_final: false,
        };
        let report_message = render_child_report_message(&report).unwrap();
        assert_eq!(report_message.role, MessageRole::Developer);
        assert!(
            report_message
                .plain_text_lossy()
                .contains("<subagent_report>")
        );

        let mut agent_message = AgentMessage::new(
            Ulid::new(),
            Ulid::new(),
            AgentMessageDelivery::Direct,
            AgentMessageKind::Question,
            serde_json::json!({ "message": "can you review this?" }),
        );
        agent_message.title = Some("Review request".into());
        let rendered_agent_message = render_agent_message_message(&agent_message).unwrap();
        assert_eq!(rendered_agent_message.role, MessageRole::Developer);
        assert!(
            rendered_agent_message
                .plain_text_lossy()
                .contains("<agent_message>")
        );
    }
}
