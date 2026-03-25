//! Application-facing runtime SDK that assembles stores, providers, loops, and
//! the per-session `agent-runtime` engine.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;

use agent_loops::SimpleLoop;
use agent_runtime::{
    LoopStrategy, Message, RuntimeConfig, RuntimeError, RuntimeEvent, SessionCommand,
    SessionEngine, SessionState, ToolExecutor,
};
use agent_store::{
    CredentialEntry, Project, ProjectConfig, ProjectId, Session, SessionId, SessionUpdate, Store,
    StoreError, StoreEvent, StoredMessage, normalize_project_root,
};
use agent_tools::native_tools as native_tool_executor;
use futures::{Stream, StreamExt};
use provider::{ModelInfo, Provider, RequestOptions};
use provider_openai::{OpenAiConfigPreset, OpenAiProvider};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, broadcast, mpsc};
use tokio_stream::wrappers::{BroadcastStream, ReceiverStream};
use tokio_util::sync::CancellationToken;

const CORE_EVENT_CAPACITY: usize = 1024;

/// Stream of events emitted by the application-facing core boundary.
pub type CoreEventStream = Pin<Box<dyn Stream<Item = CoreEvent> + Send>>;

/// Aggregated event emitted by `AgentCore`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CoreEvent {
    /// A store mutation or lifecycle update occurred.
    Store { event: StoreEvent },
    /// A runtime event occurred while driving a session turn.
    Turn {
        session_id: SessionId,
        event: RuntimeEvent,
    },
    /// A turn was cancelled by the outer application.
    TurnCancelled { session_id: SessionId },
}

/// Flattened model metadata surfaced by `AgentCore`.
#[derive(Debug, Clone)]
pub struct ProviderModelInfo {
    pub provider_name: String,
    pub model: ModelInfo,
}

/// Error returned by `AgentCore`.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error("provider not registered: {0}")]
    ProviderNotRegistered(String),
    #[error("loop not registered: {0}")]
    LoopNotRegistered(String),
    #[error("a turn is already active for session {0}")]
    TurnActive(SessionId),
    #[error("no providers are registered")]
    NoProvidersRegistered,
    #[error("no loops are registered")]
    NoLoopsRegistered,
}

/// Builder for the application-facing `AgentCore` SDK surface.
pub struct AgentCoreBuilder {
    store: Arc<dyn Store>,
    providers: BTreeMap<String, Arc<dyn Provider>>,
    loops: BTreeMap<String, Arc<dyn LoopStrategy>>,
    tools: Option<Arc<dyn ToolExecutor>>,
    default_provider_name: Option<String>,
    default_loop_name: Option<String>,
    discover_openai_from_credentials: bool,
}

impl AgentCoreBuilder {
    /// Starts building a new `AgentCore` around the provided store.
    pub fn new(store: Arc<dyn Store>) -> Self {
        Self {
            store,
            providers: BTreeMap::new(),
            loops: BTreeMap::new(),
            tools: None,
            default_provider_name: None,
            default_loop_name: None,
            discover_openai_from_credentials: true,
        }
    }

    /// Registers a provider under a stable name.
    pub fn with_provider(mut self, name: impl Into<String>, provider: Arc<dyn Provider>) -> Self {
        self.providers.insert(name.into(), provider);
        self
    }

    /// Registers a loop strategy under a stable name.
    pub fn with_loop(mut self, name: impl Into<String>, strategy: Arc<dyn LoopStrategy>) -> Self {
        self.loops.insert(name.into(), strategy);
        self
    }

    /// Overrides the tool executor used for all sessions.
    pub fn with_tools(mut self, tools: Arc<dyn ToolExecutor>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Sets the default provider name.
    pub fn default_provider(mut self, name: impl Into<String>) -> Self {
        self.default_provider_name = Some(name.into());
        self
    }

    /// Sets the default loop name.
    pub fn default_loop(mut self, name: impl Into<String>) -> Self {
        self.default_loop_name = Some(name.into());
        self
    }

    /// Disables provider discovery from stored credentials.
    pub fn without_credential_discovery(mut self) -> Self {
        self.discover_openai_from_credentials = false;
        self
    }

    /// Builds the assembled `AgentCore` surface.
    pub async fn build(mut self) -> Result<AgentCore, CoreError> {
        if self.loops.is_empty() {
            self.loops.insert("simple".into(), Arc::new(SimpleLoop));
        }
        if self.default_loop_name.is_none() {
            self.default_loop_name = Some("simple".into());
        }

        if self.discover_openai_from_credentials {
            discover_openai_providers(&self.store, &mut self.providers).await?;
        }
        if self.default_provider_name.is_none() {
            let first = self
                .providers
                .keys()
                .next()
                .cloned()
                .ok_or(CoreError::NoProvidersRegistered)?;
            self.default_provider_name = Some(first);
        }

        let tools = self
            .tools
            .unwrap_or_else(|| Arc::new(native_tool_executor()) as Arc<dyn ToolExecutor>);

        let (events, _) = broadcast::channel(CORE_EVENT_CAPACITY);
        let runtime = AgentCore {
            store: self.store.clone(),
            providers: self.providers,
            loops: self.loops,
            tools,
            default_provider_name: self
                .default_provider_name
                .ok_or(CoreError::NoProvidersRegistered)?,
            default_loop_name: self.default_loop_name.ok_or(CoreError::NoLoopsRegistered)?,
            active_turns: Arc::new(Mutex::new(HashMap::new())),
            events,
        };
        runtime.spawn_store_forwarder();
        Ok(runtime)
    }
}

/// Multi-session orchestration facade built on top of `agent-runtime`.
#[derive(Clone)]
pub struct AgentCore {
    store: Arc<dyn Store>,
    providers: BTreeMap<String, Arc<dyn Provider>>,
    loops: BTreeMap<String, Arc<dyn LoopStrategy>>,
    tools: Arc<dyn ToolExecutor>,
    default_provider_name: String,
    default_loop_name: String,
    active_turns: Arc<Mutex<HashMap<SessionId, CancellationToken>>>,
    events: broadcast::Sender<CoreEvent>,
}

impl AgentCore {
    /// Returns a builder for the new core SDK surface.
    pub fn builder(store: Arc<dyn Store>) -> AgentCoreBuilder {
        AgentCoreBuilder::new(store)
    }

    /// Builds a default local core surface with native `agent-tools`,
    /// `SimpleLoop`, and provider discovery from stored credentials.
    pub async fn build_default_local(store: Arc<dyn Store>) -> Result<Self, CoreError> {
        Self::builder(store).build().await
    }

    /// Returns the backing store used by this core.
    pub fn store(&self) -> &dyn Store {
        self.store.as_ref()
    }

    /// Returns the registered provider names.
    pub fn provider_names(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }

    /// Returns the registered loop names.
    pub fn loop_names(&self) -> Vec<String> {
        self.loops.keys().cloned().collect()
    }

    /// Returns the configured default provider name.
    pub fn default_provider_name(&self) -> &str {
        &self.default_provider_name
    }

    /// Returns the configured default loop name.
    pub fn default_loop_name(&self) -> &str {
        &self.default_loop_name
    }

    /// Returns a unified event subscription stream combining store and turn
    /// events.
    pub fn subscribe(&self) -> CoreEventStream {
        Box::pin(
            BroadcastStream::new(self.events.subscribe())
                .filter_map(|item| async move { item.ok() }),
        )
    }

    /// Resolves a project by normalized root, creating it when absent.
    pub async fn resolve_or_create_project(
        &self,
        root: impl AsRef<Path>,
    ) -> Result<Project, CoreError> {
        let normalized = normalize_project_root(root.as_ref());
        if let Some(project) = self.store.projects().find_by_root(&normalized).await? {
            return Ok(project);
        }
        let name = normalized
            .file_name()
            .and_then(|segment| segment.to_str())
            .map(|segment| segment.to_owned());
        Ok(self
            .store
            .projects()
            .create(Project::new(
                name,
                Some(normalized),
                ProjectConfig::default(),
            ))
            .await?)
    }

    /// Creates a new session under a project.
    pub async fn create_session(&self, project_id: ProjectId) -> Result<Session, CoreError> {
        Ok(self
            .store
            .sessions()
            .create(Session::new(project_id))
            .await?)
    }

    /// Returns one persisted project.
    pub async fn project(&self, project_id: ProjectId) -> Result<Project, CoreError> {
        Ok(self.store.projects().get(project_id).await?)
    }

    /// Returns one persisted session.
    pub async fn session(&self, session_id: SessionId) -> Result<Session, CoreError> {
        Ok(self.store.sessions().get(session_id).await?)
    }

    /// Lists sessions under a project.
    pub async fn sessions_for_project(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<Session>, CoreError> {
        Ok(self.store.sessions().list_for_project(project_id).await?)
    }

    /// Applies a partial update to a persisted session.
    pub async fn update_session(
        &self,
        session_id: SessionId,
        update: SessionUpdate,
    ) -> Result<Session, CoreError> {
        Ok(self.store.sessions().patch(session_id, update).await?)
    }

    /// Deletes a persisted session and its transcript data.
    pub async fn delete_session(&self, session_id: SessionId) -> Result<(), CoreError> {
        Ok(self.store.sessions().delete(session_id).await?)
    }

    /// Lists the canonical stored transcript for a session.
    pub async fn messages(&self, session_id: SessionId) -> Result<Vec<StoredMessage>, CoreError> {
        Ok(self.store.messages().list_for_session(session_id).await?)
    }

    /// Returns the persisted ATIF trajectory for a session when present.
    pub async fn trajectory(
        &self,
        session_id: SessionId,
    ) -> Result<Option<atif::Trajectory>, CoreError> {
        Ok(self
            .store
            .trajectories()
            .get_for_session(session_id)
            .await?)
    }

    /// Upserts a validated ATIF trajectory for a session.
    pub async fn upsert_trajectory(
        &self,
        session_id: SessionId,
        trajectory: atif::Trajectory,
    ) -> Result<atif::Trajectory, CoreError> {
        Ok(self
            .store
            .trajectories()
            .upsert(session_id, trajectory)
            .await?)
    }

    /// Returns the effective runtime configuration for a session.
    pub async fn effective_runtime_config(
        &self,
        session_id: SessionId,
    ) -> Result<RuntimeConfig, CoreError> {
        let session = self.store.sessions().get(session_id).await?;
        let project = self.store.projects().get(session.project_id).await?;
        Ok(resolve_runtime_config(&project.config, &session))
    }

    /// Returns the effective loop name for a session.
    pub async fn current_loop_name_for_session(
        &self,
        session_id: SessionId,
    ) -> Result<String, CoreError> {
        let session = self.store.sessions().get(session_id).await?;
        Ok(session
            .loop_name
            .unwrap_or_else(|| self.default_loop_name.clone()))
    }

    /// Returns the effective model identifier for a session when configured.
    pub async fn current_model_id_for_session(
        &self,
        session_id: SessionId,
    ) -> Result<Option<String>, CoreError> {
        let session = self.store.sessions().get(session_id).await?;
        let project = self.store.projects().get(session.project_id).await?;
        Ok(session
            .model
            .or_else(|| project.config.default_model)
            .or_else(|| project.config.runtime.model))
    }

    /// Lists all registered models across all providers.
    pub fn list_models(&self) -> Vec<ProviderModelInfo> {
        let mut models = self
            .providers
            .iter()
            .flat_map(|(provider_name, provider)| {
                provider
                    .info()
                    .models
                    .into_iter()
                    .map(|model| ProviderModelInfo {
                        provider_name: provider_name.clone(),
                        model,
                    })
            })
            .collect::<Vec<_>>();
        models.sort_by(|left, right| {
            left.provider_name
                .cmp(&right.provider_name)
                .then_with(|| left.model.id.cmp(&right.model.id))
        });
        models
    }

    /// Starts one store-backed turn for a session and returns an event stream
    /// for that turn only.
    pub async fn turn(
        &self,
        session_id: SessionId,
        input: Vec<Message>,
    ) -> Result<CoreEventStream, CoreError> {
        let cancel = CancellationToken::new();
        {
            let mut active_turns = self.active_turns.lock().await;
            if active_turns.contains_key(&session_id) {
                return Err(CoreError::TurnActive(session_id));
            }
            active_turns.insert(session_id, cancel.clone());
        }

        let session = self.store.sessions().get(session_id).await?;
        let project = self.store.projects().get(session.project_id).await?;
        let resolved = self.resolve_session_runtime(&project, &session)?;
        let transcript = self
            .store
            .messages()
            .list_for_session(session_id)
            .await?
            .into_iter()
            .map(|stored| stored.message)
            .collect::<Vec<_>>();
        let initial_state = SessionState::with_transcript(session_id, transcript);
        let engine = SessionEngine::new(
            resolved.provider,
            self.tools.clone(),
            resolved.loop_strategy,
            resolved.runtime_config,
            initial_state,
        );
        let mut receiver = engine.subscribe();
        engine
            .submit(SessionCommand::SubmitInput {
                input,
                source: None,
            })
            .await?;

        let (turn_tx, turn_rx) = mpsc::channel(256);
        let core = self.clone();
        tokio::spawn(async move {
            let mut terminal_seen = false;
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        core.emit(CoreEvent::TurnCancelled { session_id });
                        let _ = turn_tx.send(CoreEvent::TurnCancelled { session_id }).await;
                        core.persist_engine_snapshot(session_id, &engine).await;
                        let _ = engine.submit(SessionCommand::Shutdown).await;
                        break;
                    }
                    event = receiver.recv() => {
                        let event = match event {
                            Ok(event) => event,
                            Err(broadcast::error::RecvError::Lagged(_)) => continue,
                            Err(broadcast::error::RecvError::Closed) => break,
                        };
                        let is_terminal = matches!(
                            event,
                            RuntimeEvent::TurnFinished { .. }
                                | RuntimeEvent::Error {
                                    recoverable: false,
                                    ..
                                }
                        );
                        if is_terminal {
                            terminal_seen = true;
                            core.persist_engine_snapshot(session_id, &engine).await;
                            let core_event = CoreEvent::Turn {
                                session_id,
                                event: event.clone(),
                            };
                            core.emit(core_event.clone());
                            if turn_tx.send(core_event).await.is_err() {
                                break;
                            }
                            let _ = engine.submit(SessionCommand::Shutdown).await;
                            break;
                        } else {
                            let core_event = CoreEvent::Turn {
                                session_id,
                                event: event.clone(),
                            };
                            core.emit(core_event.clone());
                            if turn_tx.send(core_event).await.is_err() {
                                break;
                            }
                        }
                    }
                }
            }

            if !terminal_seen && !cancel.is_cancelled() {
                core.persist_engine_snapshot(session_id, &engine).await;
                let _ = engine.submit(SessionCommand::Shutdown).await;
            }

            core.active_turns.lock().await.remove(&session_id);
        });

        Ok(Box::pin(ReceiverStream::new(turn_rx)))
    }

    /// Cancels a currently active turn for a session.
    pub async fn cancel_turn(&self, session_id: SessionId) -> Result<(), CoreError> {
        if let Some(cancel) = self.active_turns.lock().await.get(&session_id).cloned() {
            cancel.cancel();
        }
        Ok(())
    }

    fn resolve_session_runtime(
        &self,
        project: &Project,
        session: &Session,
    ) -> Result<ResolvedSessionRuntime, CoreError> {
        let provider_name = session
            .provider
            .clone()
            .or_else(|| project.config.default_provider.clone())
            .unwrap_or_else(|| self.default_provider_name.clone());
        let loop_name = session
            .loop_name
            .clone()
            .unwrap_or_else(|| self.default_loop_name.clone());
        let provider = self
            .providers
            .get(&provider_name)
            .cloned()
            .ok_or_else(|| CoreError::ProviderNotRegistered(provider_name.clone()))?;
        let loop_strategy = self
            .loops
            .get(&loop_name)
            .cloned()
            .ok_or_else(|| CoreError::LoopNotRegistered(loop_name.clone()))?;

        Ok(ResolvedSessionRuntime {
            provider,
            loop_strategy,
            runtime_config: resolve_runtime_config(&project.config, session),
        })
    }

    fn emit(&self, event: CoreEvent) {
        let _ = self.events.send(event);
    }

    fn spawn_store_forwarder(&self) {
        let mut store_events = self.store.subscribe();
        let bus = self.events.clone();
        tokio::spawn(async move {
            while let Some(event) = store_events.next().await {
                let _ = bus.send(CoreEvent::Store { event });
            }
        });
    }

    async fn persist_engine_snapshot(&self, session_id: SessionId, engine: &SessionEngine) {
        let snapshot = engine.snapshot().await;
        let stored_messages = snapshot
            .transcript
            .into_iter()
            .enumerate()
            .map(|(idx, message)| StoredMessage::new(session_id, idx as u64, message))
            .collect::<Vec<_>>();
        if let Err(error) = self
            .store
            .messages()
            .replace_for_session(session_id, stored_messages)
            .await
        {
            self.emit(CoreEvent::Turn {
                session_id,
                event: RuntimeEvent::Error {
                    message: format!("failed to persist transcript: {error}"),
                    recoverable: false,
                },
            });
        }
        if let Err(error) = self
            .store
            .sessions()
            .patch(session_id, SessionUpdate::default())
            .await
        {
            self.emit(CoreEvent::Turn {
                session_id,
                event: RuntimeEvent::Error {
                    message: format!("failed to update session timestamp: {error}"),
                    recoverable: false,
                },
            });
        }
    }
}

struct ResolvedSessionRuntime {
    provider: Arc<dyn Provider>,
    loop_strategy: Arc<dyn LoopStrategy>,
    runtime_config: RuntimeConfig,
}

fn resolve_runtime_config(project: &ProjectConfig, session: &Session) -> RuntimeConfig {
    let mut config = project.runtime.clone();
    config.model = session
        .model
        .clone()
        .or_else(|| project.default_model.clone())
        .or_else(|| config.model.clone());
    config.request = merge_request_options(&project.runtime.request, &session.request);
    config
}

fn merge_request_options(base: &RequestOptions, overlay: &RequestOptions) -> RequestOptions {
    RequestOptions {
        max_output_tokens: overlay.max_output_tokens.or(base.max_output_tokens),
        temperature: overlay.temperature.or(base.temperature),
        top_p: overlay.top_p.or(base.top_p),
        top_k: overlay.top_k.or(base.top_k),
        stop_sequences: if overlay.stop_sequences.is_empty() {
            base.stop_sequences.clone()
        } else {
            overlay.stop_sequences.clone()
        },
        tool_choice: overlay
            .tool_choice
            .clone()
            .or_else(|| base.tool_choice.clone()),
        parallel_tool_calls: overlay.parallel_tool_calls.or(base.parallel_tool_calls),
        reasoning: overlay.reasoning.clone().or_else(|| base.reasoning.clone()),
        metadata: if overlay.metadata.is_empty() {
            base.metadata.clone()
        } else {
            overlay.metadata.clone()
        },
    }
}

async fn discover_openai_providers(
    store: &Arc<dyn Store>,
    providers: &mut BTreeMap<String, Arc<dyn Provider>>,
) -> Result<(), CoreError> {
    let credentials = store.credentials().list().await?;
    for ((provider_name, _credential_id), entry) in credentials {
        if providers.contains_key(&provider_name) {
            continue;
        }
        let Some(preset) = OpenAiConfigPreset::by_name(&provider_name) else {
            continue;
        };
        let CredentialEntry { credential, .. } = entry;
        let agent_store::ProviderCredential::ApiKey { api_key } = credential else {
            continue;
        };
        providers.insert(
            provider_name,
            Arc::new(OpenAiProvider::new(preset.into_config(api_key))),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::Duration;

    use async_stream::stream;
    use futures::StreamExt;
    use provider::{
        Block, BlockDelta, BlockKind, Event, EventStream, FinishReason, MockProvider, ProviderInfo,
        Request, Usage,
    };
    use tempfile::TempDir;
    use tokio::time::timeout;

    use super::*;
    use agent_store::{CredentialHealth, InMemoryStore, ProviderCredential};

    struct SlowProvider;

    impl Provider for SlowProvider {
        fn stream<'a>(
            &'a self,
            _request: &'a Request,
        ) -> futures::future::BoxFuture<'a, Result<EventStream<'a>, provider::Error>> {
            Box::pin(async move {
                let stream = stream! {
                    yield Ok(Event::ResponseStart {
                        response_id: Some("slow".into()),
                        model: Some("slow-model".into()),
                    });
                    yield Ok(Event::BlockStart {
                        block: Block {
                            id: "assistant-text".into(),
                            output_index: 0,
                            kind: BlockKind::Text,
                            item_id: None,
                        },
                    });
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    yield Ok(Event::BlockDelta {
                        id: "assistant-text".into(),
                        delta: BlockDelta::Text {
                            text: "done".into(),
                        },
                    });
                    yield Ok(Event::BlockStop {
                        id: "assistant-text".into(),
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
            MockProvider::new().info()
        }
    }

    fn sample_trajectory(session_id: SessionId) -> atif::Trajectory {
        atif::Trajectory {
            schema_version: atif::SchemaVersion::default(),
            session_id: session_id.to_string(),
            agent: atif::Agent {
                name: "brain".into(),
                version: "0.1.0".into(),
                model_name: Some("mock-echo".into()),
                tool_definitions: None,
                extra: None,
            },
            steps: vec![atif::Step {
                step_id: 1,
                timestamp: None,
                source: atif::StepSource::User,
                model_name: None,
                reasoning_effort: None,
                message: "hello".into(),
                reasoning_content: None,
                tool_calls: None,
                observation: None,
                metrics: None,
                is_copied_context: None,
                extra: None,
            }],
            notes: None,
            final_metrics: None,
            continued_trajectory_ref: None,
            extra: None,
        }
    }

    #[tokio::test]
    async fn build_default_local_requires_explicit_provider_registration() {
        let store = Arc::new(InMemoryStore::new());
        let result = AgentCore::build_default_local(store).await;

        assert!(matches!(result, Err(CoreError::NoProvidersRegistered)));
    }

    #[tokio::test]
    async fn resolve_or_create_project_normalizes_roots() {
        let store = Arc::new(InMemoryStore::new());
        let core = AgentCore::builder(store)
            .with_provider("mock", Arc::new(MockProvider::new()))
            .build()
            .await
            .unwrap();

        let first = core
            .resolve_or_create_project("/tmp/work/./repo")
            .await
            .unwrap();
        let second = core
            .resolve_or_create_project(PathBuf::from("/tmp/work/repo"))
            .await
            .unwrap();

        assert_eq!(first.id, second.id);
    }

    #[tokio::test]
    async fn turn_persists_transcript_back_to_store() {
        let store = Arc::new(InMemoryStore::new());
        let core = AgentCore::builder(store.clone())
            .with_provider("mock", Arc::new(MockProvider::new()))
            .build()
            .await
            .unwrap();
        let project = core
            .resolve_or_create_project("/tmp/demo-core")
            .await
            .unwrap();
        let session = core.create_session(project.id).await.unwrap();

        let mut events = core
            .turn(session.id, vec![Message::user_text("hello there")])
            .await
            .unwrap();
        let mut finished = false;
        while let Some(event) = timeout(Duration::from_secs(1), events.next())
            .await
            .unwrap()
        {
            if matches!(
                event,
                CoreEvent::Turn {
                    event: RuntimeEvent::TurnFinished { .. },
                    ..
                }
            ) {
                finished = true;
                break;
            }
        }

        assert!(finished);
        let messages = core.messages(session.id).await.unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].message.plain_text_lossy(), "hello there");
    }

    #[tokio::test]
    async fn cancel_turn_emits_turn_cancelled() {
        let store = Arc::new(InMemoryStore::new());
        let core = AgentCore::builder(store.clone())
            .with_provider("slow", Arc::new(SlowProvider))
            .build()
            .await
            .unwrap();
        let project = core
            .resolve_or_create_project("/tmp/slow-core")
            .await
            .unwrap();
        let session = core
            .update_session(
                core.create_session(project.id).await.unwrap().id,
                SessionUpdate {
                    provider: Some(Some("slow".into())),
                    ..SessionUpdate::default()
                },
            )
            .await
            .unwrap();

        let mut events = core
            .turn(session.id, vec![Message::user_text("wait")])
            .await
            .unwrap();
        core.cancel_turn(session.id).await.unwrap();

        let cancelled = timeout(Duration::from_secs(1), async {
            while let Some(event) = events.next().await {
                if matches!(event, CoreEvent::TurnCancelled { session_id } if session_id == session.id) {
                    return true;
                }
            }
            false
        })
        .await
        .unwrap();

        assert!(cancelled);
    }

    #[tokio::test]
    async fn builder_discovers_openai_compatible_providers_from_credentials() {
        let store = Arc::new(InMemoryStore::new());
        let project = store
            .projects()
            .create(Project::new(None, None, ProjectConfig::default()))
            .await
            .unwrap();
        let _session = store
            .sessions()
            .create(Session::new(project.id))
            .await
            .unwrap();
        store
            .credentials()
            .create(
                ("openai".into(), "primary".into()),
                CredentialEntry {
                    label: "Primary".into(),
                    credential: ProviderCredential::ApiKey {
                        api_key: "sk-test".into(),
                    },
                    health: CredentialHealth::default(),
                },
            )
            .await
            .unwrap();

        let core = AgentCore::build_default_local(store).await.unwrap();
        assert!(core.provider_names().contains(&"openai".to_string()));
    }

    #[tokio::test]
    async fn trajectories_roundtrip_through_core() {
        let store = Arc::new(InMemoryStore::new());
        let core = AgentCore::builder(store.clone())
            .with_provider("mock", Arc::new(MockProvider::new()))
            .build()
            .await
            .unwrap();
        let project = core
            .resolve_or_create_project("/tmp/trajectory-core")
            .await
            .unwrap();
        let session = core.create_session(project.id).await.unwrap();
        let trajectory = sample_trajectory(session.id);

        core.upsert_trajectory(session.id, trajectory.clone())
            .await
            .unwrap();

        let loaded = core.trajectory(session.id).await.unwrap().unwrap();
        assert_eq!(loaded.session_id, trajectory.session_id);
    }

    #[tokio::test]
    async fn file_store_default_core_roundtrips() {
        let temp = TempDir::new().unwrap();
        let store = Arc::new(agent_store::FileStore::new(temp.path()).await.unwrap());
        let core = AgentCore::builder(store.clone())
            .with_provider("mock", Arc::new(MockProvider::new()))
            .build()
            .await
            .unwrap();
        let project = core
            .resolve_or_create_project(std::path::PathBuf::from(temp.path()))
            .await
            .unwrap();
        let session = core.create_session(project.id).await.unwrap();

        let mut events = core
            .turn(session.id, vec![Message::user_text("persist")])
            .await
            .unwrap();
        while let Some(event) = timeout(Duration::from_secs(1), events.next())
            .await
            .unwrap()
        {
            if matches!(
                event,
                CoreEvent::Turn {
                    event: RuntimeEvent::TurnFinished { .. },
                    ..
                }
            ) {
                break;
            }
        }

        let reopened = agent_store::FileStore::new(temp.path()).await.unwrap();
        let messages = reopened
            .messages()
            .list_for_session(session.id)
            .await
            .unwrap();
        assert_eq!(messages.len(), 2);
    }

    struct ToolCallingProvider;

    impl Provider for ToolCallingProvider {
        fn stream<'a>(
            &'a self,
            request: &'a Request,
        ) -> futures::future::BoxFuture<'a, Result<EventStream<'a>, provider::Error>> {
            Box::pin(async move {
                let saw_tool_result = request.messages.iter().any(|message| {
                    message
                        .content
                        .iter()
                        .any(|block| matches!(block, provider::ContentBlock::ToolResult { .. }))
                });

                let stream = stream! {
                    yield Ok(Event::ResponseStart {
                        response_id: Some("tool-provider".into()),
                        model: Some("tool-model".into()),
                    });

                    if saw_tool_result {
                        yield Ok(Event::BlockStart {
                            block: Block {
                                id: "assistant-text".into(),
                                output_index: 0,
                                kind: BlockKind::Text,
                                item_id: Some("assistant-item".into()),
                            },
                        });
                        yield Ok(Event::BlockDelta {
                            id: "assistant-text".into(),
                            delta: BlockDelta::Text {
                                text: "tool finished".into(),
                            },
                        });
                        yield Ok(Event::BlockStop {
                            id: "assistant-text".into(),
                        });
                    } else {
                        yield Ok(Event::BlockStart {
                            block: Block {
                                id: "assistant-tool".into(),
                                output_index: 0,
                                kind: BlockKind::ToolCall {
                                    name: Some("echo".into()),
                                    call_id: Some("tool-call-1".into()),
                                },
                                item_id: Some("tool-item".into()),
                            },
                        });
                        yield Ok(Event::BlockDelta {
                            id: "assistant-tool".into(),
                            delta: BlockDelta::Json {
                                partial_json: serde_json::json!({
                                    "message": "hello from tool"
                                }).to_string(),
                            },
                        });
                        yield Ok(Event::BlockStop {
                            id: "assistant-tool".into(),
                        });
                    }

                    yield Ok(Event::Usage {
                        usage: Usage::with_totals(Some(1), Some(1)),
                    });
                    yield Ok(Event::Completed {
                        response_id: Some("tool-provider".into()),
                        finish_reason: Some(FinishReason::Stop),
                    });
                };

                Ok(Box::pin(stream) as EventStream<'a>)
            })
        }

        fn info(&self) -> ProviderInfo {
            MockProvider::new().info()
        }
    }

    #[tokio::test]
    async fn default_tools_execute_without_legacy_bridge() {
        let store = Arc::new(InMemoryStore::new());
        let core = AgentCore::builder(store.clone())
            .with_provider("tool", Arc::new(ToolCallingProvider))
            .build()
            .await
            .unwrap();
        let project = core
            .resolve_or_create_project("/tmp/tool-core")
            .await
            .unwrap();
        let session = core
            .update_session(
                core.create_session(project.id).await.unwrap().id,
                SessionUpdate {
                    provider: Some(Some("tool".into())),
                    ..SessionUpdate::default()
                },
            )
            .await
            .unwrap();

        let mut events = core
            .turn(session.id, vec![Message::user_text("use the echo tool")])
            .await
            .unwrap();
        while let Some(event) = timeout(Duration::from_secs(1), events.next())
            .await
            .unwrap()
        {
            if matches!(
                event,
                CoreEvent::Turn {
                    event: RuntimeEvent::TurnFinished { .. },
                    ..
                }
            ) {
                break;
            }
        }

        let messages = core.messages(session.id).await.unwrap();
        assert!(messages.iter().any(|message| {
            message
                .message
                .content
                .iter()
                .any(|block| matches!(block, provider::ContentBlock::ToolResult { .. }))
        }));
    }
}
