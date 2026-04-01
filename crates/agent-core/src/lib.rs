//! Application-facing runtime SDK that assembles stores, providers, loops, and
//! the per-session `agent-runtime` engine.

mod bootstrap;

use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use agent_loops::{RobustLoop, SimpleLoop, Terminus2Loop, TerminusKiraLoop};
use agent_runtime::{
    LoopStrategy, Message, RuntimeConfig, RuntimeError, RuntimeEvent, SessionCommand,
    SessionEngine, SessionState, ToolExecutor,
};
use agent_store::{
    CredentialEntry, Project, ProjectConfig, ProjectId, Session, SessionId, SessionUpdate, Store,
    StoreError, StoreEvent, StoredMessage, normalize_project_root,
};
use agent_tool::{EchoTool, RegistryToolExecutor};
use agent_tool_files::register_native_tools as register_native_file_tools;
use agent_tool_process::register_native_tools as register_native_process_tools;
use chrono::{DateTime, Utc};
use futures::{Stream, StreamExt, future::BoxFuture};
use provider::{
    CredentialEntry as ProviderCredentialEntry,
    CredentialFailureRecord as ProviderCredentialFailureRecord,
    CredentialHealth as ProviderCredentialHealth, CredentialPool as SharedCredentialPool,
    ModelInfo, Provider, RequestOptions, StickyRoundRobin,
};
use provider_openai::{
    OpenAiConfigPreset, OpenAiOAuthCredentials, OpenAiOAuthPreset, OpenAiOAuthProvider,
    OpenAiProvider, refresh_access_token,
};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, broadcast, mpsc};
use tokio_stream::wrappers::{BroadcastStream, ReceiverStream};
use tokio_util::sync::CancellationToken;

use crate::bootstrap::resolve_project_config;

const CORE_EVENT_CAPACITY: usize = 1024;

fn native_tool_executor() -> RegistryToolExecutor {
    let mut registry = RegistryToolExecutor::new();
    registry
        .register(Arc::new(EchoTool))
        .expect("builtin core tool name should be unique");
    register_native_file_tools(&mut registry).expect("builtin file tool names should be unique");
    register_native_process_tools(&mut registry)
        .expect("builtin process tool names should be unique");
    registry
}

/// Stream of events emitted by the application-facing core boundary.
pub type CoreEventStream = Pin<Box<dyn Stream<Item = CoreEvent> + Send>>;

/// Aggregated event emitted by the shared `AgentCore` boundary.
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
#[derive(Debug, Clone, Serialize)]
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
    #[error(transparent)]
    Bootstrap(#[from] bootstrap::BootstrapError),
    #[error(transparent)]
    ProviderOpenAi(#[from] provider_openai::Error),
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
    #[error("internal: {0}")]
    Internal(String),
}

struct CredentialActivation {
    store: Arc<dyn Store>,
    pool: Arc<SharedCredentialPool>,
    client: reqwest::Client,
}

#[derive(Clone)]
struct ActiveTurn {
    id: u64,
    cancel: CancellationToken,
}

impl CredentialActivation {
    fn new(store: Arc<dyn Store>, pool: Arc<SharedCredentialPool>) -> Self {
        Self {
            store,
            pool,
            client: reqwest::Client::new(),
        }
    }

    async fn sync_provider(&self, provider_name: &str) -> Result<(), CoreError> {
        let stored = self
            .store
            .credentials()
            .list_for_provider(provider_name)
            .await?;
        let mut entries = Vec::with_capacity(stored.len());
        for (key, entry) in stored {
            let credential_id = key.1.clone();
            let refreshed = self
                .refresh_entry_if_needed(provider_name, key, entry)
                .await?;
            entries.push(store_entry_to_provider_entry(&credential_id, &refreshed));
        }
        self.pool
            .set_entries(provider_name.to_owned(), entries)
            .await;
        Ok(())
    }

    async fn persist_provider_health(&self, provider_name: &str) -> Result<(), CoreError> {
        let entries = self.pool.entries(provider_name).await;
        for entry in entries {
            let health = provider_health_to_store_health(&entry.health);
            self.store
                .credentials()
                .update_health(provider_name, &entry.id, &health)
                .await?;
        }
        Ok(())
    }

    async fn refresh_entry_if_needed(
        &self,
        provider_name: &str,
        key: (String, String),
        entry: CredentialEntry,
    ) -> Result<CredentialEntry, CoreError> {
        let agent_store::ProviderCredential::OAuth(credentials) = &entry.credential else {
            return Ok(entry);
        };

        if OpenAiOAuthPreset::by_name(provider_name).is_none() || !credentials.needs_refresh() {
            return Ok(entry);
        }

        let refreshed = refresh_access_token(
            &self.client,
            &OpenAiOAuthCredentials {
                access_token: credentials.access_token.clone(),
                refresh_token: credentials.refresh_token.clone(),
                expires_at: credentials.expires_at,
                client_id: credentials.client_id.clone(),
                token_endpoint: credentials.token_endpoint.clone(),
                account_id: credentials.account_id.clone(),
                token_type: credentials.token_type.clone(),
                scopes: credentials.scopes.clone(),
            },
        )
        .await?;

        let mut updated = entry;
        updated.credential =
            agent_store::ProviderCredential::OAuth(agent_store::OAuthCredentials {
                access_token: refreshed.access_token,
                refresh_token: refreshed.refresh_token,
                expires_at: refreshed.expires_at,
                client_id: refreshed.client_id,
                token_endpoint: refreshed.token_endpoint,
                account_id: refreshed.account_id,
                token_type: refreshed.token_type,
                scopes: refreshed.scopes,
            });

        Ok(self.store.credentials().update(key, updated).await?)
    }
}

fn store_entry_to_provider_entry(
    credential_id: &str,
    entry: &CredentialEntry,
) -> ProviderCredentialEntry {
    let mut resolved = match &entry.credential {
        agent_store::ProviderCredential::ApiKey { api_key } => {
            ProviderCredentialEntry::bearer(credential_id.to_owned(), api_key.clone())
        }
        agent_store::ProviderCredential::OAuth(credentials) => {
            ProviderCredentialEntry::openai_oauth(
                credential_id.to_owned(),
                credentials.access_token.clone(),
                credentials.account_id.clone(),
            )
        }
    };
    resolved.enabled = entry.enabled;
    resolved.health = store_health_to_provider_health(&entry.health);
    resolved
}

async fn release_active_turn(
    active_turns: &Mutex<HashMap<SessionId, ActiveTurn>>,
    session_id: SessionId,
    active_turn_id: u64,
) {
    let mut active_turns = active_turns.lock().await;
    if active_turns
        .get(&session_id)
        .is_some_and(|active_turn| active_turn.id == active_turn_id)
    {
        active_turns.remove(&session_id);
    }
}

fn store_health_to_provider_health(
    health: &agent_store::CredentialHealth,
) -> ProviderCredentialHealth {
    ProviderCredentialHealth {
        last_ok_ms: health.last_ok.and_then(datetime_to_ms),
        last_error: health.last_error.as_ref().and_then(|error| {
            Some(ProviderCredentialFailureRecord {
                message: error.message.clone(),
                code: error.code.clone(),
                recorded_at_ms: datetime_to_ms(error.recorded_at)?,
            })
        }),
        consecutive_errors: health.consecutive_errors,
    }
}

fn provider_health_to_store_health(
    health: &ProviderCredentialHealth,
) -> agent_store::CredentialHealth {
    let last_ok = health.last_ok_ms.and_then(ms_to_datetime);
    let last_error = health.last_error.as_ref().and_then(|error| {
        Some(agent_store::CredentialError {
            message: error.message.clone(),
            code: error.code.clone(),
            recorded_at: ms_to_datetime(error.recorded_at_ms)?,
        })
    });
    let updated_at = last_error
        .as_ref()
        .map(|error| error.recorded_at)
        .or(last_ok)
        .unwrap_or_else(Utc::now);

    agent_store::CredentialHealth {
        last_ok,
        last_error,
        consecutive_errors: health.consecutive_errors,
        updated_at,
    }
}

fn datetime_to_ms(datetime: DateTime<Utc>) -> Option<u128> {
    u128::try_from(datetime.timestamp_millis()).ok()
}

fn ms_to_datetime(milliseconds: u128) -> Option<DateTime<Utc>> {
    let milliseconds = i64::try_from(milliseconds).ok()?;
    DateTime::from_timestamp_millis(milliseconds)
}

/// Consumer-facing core boundary shared by native and remote implementations.
///
/// Embedded clients, remote clients, and hosted servers should depend on this
/// trait rather than on a specific locality-aware implementation. Native and
/// remote construction are intentionally separate concerns.
pub trait AgentCore: Send + Sync {
    /// Returns the backing store or a remote proxy implementing the same store
    /// trait surface.
    fn store(&self) -> &dyn Store;

    /// Returns the registered provider names.
    fn provider_names(&self) -> Vec<String>;

    /// Returns the registered loop names.
    fn loop_names(&self) -> Vec<String>;

    /// Returns the configured default provider name.
    fn default_provider_name(&self) -> &str;

    /// Returns the configured default loop name.
    fn default_loop_name(&self) -> &str;

    /// Returns a unified live event stream for store and turn activity.
    fn subscribe(&self) -> CoreEventStream;

    /// Resolves a project by normalized root, creating it when absent.
    fn resolve_or_create_project(
        &self,
        root: std::path::PathBuf,
    ) -> BoxFuture<'_, Result<Project, CoreError>>;

    /// Creates a new session under a project.
    fn create_session(&self, project_id: ProjectId) -> BoxFuture<'_, Result<Session, CoreError>>;

    /// Returns one persisted project.
    fn project(&self, project_id: ProjectId) -> BoxFuture<'_, Result<Project, CoreError>>;

    /// Returns one persisted session.
    fn session(&self, session_id: SessionId) -> BoxFuture<'_, Result<Session, CoreError>>;

    /// Lists sessions under a project.
    fn sessions_for_project(
        &self,
        project_id: ProjectId,
    ) -> BoxFuture<'_, Result<Vec<Session>, CoreError>>;

    /// Applies a partial update to a persisted session.
    fn update_session(
        &self,
        session_id: SessionId,
        update: SessionUpdate,
    ) -> BoxFuture<'_, Result<Session, CoreError>>;

    /// Deletes a persisted session and its transcript data.
    fn delete_session(&self, session_id: SessionId) -> BoxFuture<'_, Result<(), CoreError>>;

    /// Lists the canonical stored transcript for a session.
    fn messages(
        &self,
        session_id: SessionId,
    ) -> BoxFuture<'_, Result<Vec<StoredMessage>, CoreError>>;

    /// Returns the persisted ATIF trajectory for a session when present.
    fn trajectory(
        &self,
        session_id: SessionId,
    ) -> BoxFuture<'_, Result<Option<atif::Trajectory>, CoreError>>;

    /// Upserts a validated ATIF trajectory for a session.
    fn upsert_trajectory(
        &self,
        session_id: SessionId,
        trajectory: atif::Trajectory,
    ) -> BoxFuture<'_, Result<atif::Trajectory, CoreError>>;

    /// Returns the effective runtime configuration for a session.
    fn effective_runtime_config(
        &self,
        session_id: SessionId,
    ) -> BoxFuture<'_, Result<RuntimeConfig, CoreError>>;

    /// Returns the effective loop name for a session.
    fn current_loop_name_for_session(
        &self,
        session_id: SessionId,
    ) -> BoxFuture<'_, Result<String, CoreError>>;

    /// Returns the effective model identifier for a session when configured.
    fn current_model_id_for_session(
        &self,
        session_id: SessionId,
    ) -> BoxFuture<'_, Result<Option<String>, CoreError>>;

    /// Lists all registered models across all providers.
    fn list_models(&self) -> Vec<ProviderModelInfo>;

    /// Starts one store-backed turn for a session and returns an event stream
    /// for that turn only.
    fn turn(
        &self,
        session_id: SessionId,
        input: Vec<Message>,
    ) -> BoxFuture<'_, Result<CoreEventStream, CoreError>>;

    /// Cancels a currently active turn for a session.
    fn cancel_turn(&self, session_id: SessionId) -> BoxFuture<'_, Result<(), CoreError>>;
}

/// Builder for the embedded `AgentCoreNative` implementation.
pub struct AgentCoreNativeBuilder {
    store: Arc<dyn Store>,
    providers: BTreeMap<String, Arc<dyn Provider>>,
    loops: BTreeMap<String, Arc<dyn LoopStrategy>>,
    tools: Option<Arc<dyn ToolExecutor>>,
    default_provider_name: Option<String>,
    default_loop_name: Option<String>,
    discover_openai_from_credentials: bool,
}

impl AgentCoreNativeBuilder {
    /// Starts building a new embedded `AgentCoreNative` around the provided
    /// store.
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

    /// Builds the assembled embedded `AgentCoreNative`.
    pub async fn build(mut self) -> Result<AgentCoreNative, CoreError> {
        if self.loops.is_empty() {
            self.loops.insert("simple".into(), Arc::new(SimpleLoop));
            self.loops.insert("robust".into(), Arc::new(RobustLoop));
            self.loops
                .insert("terminus2".into(), Arc::new(Terminus2Loop));
            self.loops
                .insert("terminus_kira".into(), Arc::new(TerminusKiraLoop));
        }
        if self.default_loop_name.is_none() {
            self.default_loop_name = Some("simple".into());
        }

        let credential_activation = if self.discover_openai_from_credentials {
            let pool = Arc::new(SharedCredentialPool::new(Arc::new(StickyRoundRobin::new())));
            discover_openai_providers(&self.store, &mut self.providers, pool.clone()).await?;
            Some(Arc::new(CredentialActivation::new(
                self.store.clone(),
                pool,
            )))
        } else {
            None
        };

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
        let runtime = AgentCoreNative {
            store: self.store.clone(),
            providers: self.providers,
            loops: self.loops,
            tools,
            default_provider_name: self
                .default_provider_name
                .ok_or(CoreError::NoProvidersRegistered)?,
            default_loop_name: self.default_loop_name.ok_or(CoreError::NoLoopsRegistered)?,
            credential_activation,
            active_turns: Arc::new(Mutex::new(HashMap::new())),
            next_active_turn_id: Arc::new(AtomicU64::new(1)),
            events,
        };
        runtime.spawn_store_forwarder();
        Ok(runtime)
    }
}

/// Native in-process implementation of the shared `AgentCore` boundary.
#[derive(Clone)]
pub struct AgentCoreNative {
    store: Arc<dyn Store>,
    providers: BTreeMap<String, Arc<dyn Provider>>,
    loops: BTreeMap<String, Arc<dyn LoopStrategy>>,
    tools: Arc<dyn ToolExecutor>,
    default_provider_name: String,
    default_loop_name: String,
    credential_activation: Option<Arc<CredentialActivation>>,
    active_turns: Arc<Mutex<HashMap<SessionId, ActiveTurn>>>,
    next_active_turn_id: Arc<AtomicU64>,
    events: broadcast::Sender<CoreEvent>,
}

impl AgentCoreNative {
    /// Returns a builder for the embedded native core implementation.
    pub fn builder(store: Arc<dyn Store>) -> AgentCoreNativeBuilder {
        AgentCoreNativeBuilder::new(store)
    }

    /// Builds a default local core surface with the native `agent-tool`
    /// family crates, `SimpleLoop`, and provider discovery from stored
    /// credentials.
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
        let config = resolve_project_config(&normalized)?;
        Ok(self
            .store
            .projects()
            .create(Project::new(name, Some(normalized), config))
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
        let project = self.store.projects().get(session.project_id).await?;
        let project_default_loop = project.config.default_loop.clone();
        Ok(session
            .loop_name
            .or(project_default_loop)
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
            .or(project.config.default_model)
            .or(project.config.runtime.model))
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
        let active_turn_id = self.next_active_turn_id.fetch_add(1, Ordering::Relaxed);
        let cancel = CancellationToken::new();
        {
            let mut active_turns = self.active_turns.lock().await;
            if let Some(active_turn) = active_turns.get(&session_id) {
                if active_turn.cancel.is_cancelled() {
                    active_turns.remove(&session_id);
                }
            }
            if active_turns.contains_key(&session_id) {
                return Err(CoreError::TurnActive(session_id));
            }
            active_turns.insert(
                session_id,
                ActiveTurn {
                    id: active_turn_id,
                    cancel: cancel.clone(),
                },
            );
        }

        let session = self.store.sessions().get(session_id).await?;
        let project = self.store.projects().get(session.project_id).await?;
        let resolved = self.resolve_session_runtime(&project, &session)?;
        if let Some(credential_activation) = &self.credential_activation {
            credential_activation
                .sync_provider(&resolved.provider_name)
                .await?;
        }
        let mut runtime_config = resolved.runtime_config;
        runtime_config.request.metadata.insert(
            "session_id".into(),
            serde_json::Value::String(session_id.to_string()),
        );
        let transcript = self
            .store
            .messages()
            .list_for_session(session_id)
            .await?
            .into_iter()
            .map(|stored| stored.message)
            .collect::<Vec<_>>();
        let transcript =
            seed_project_prompt(project.config.system_prompt.as_deref(), transcript, &input);
        let initial_state = SessionState::with_transcript(session_id, transcript);
        let engine = SessionEngine::new(
            resolved.provider,
            self.tools.clone(),
            resolved.loop_strategy,
            runtime_config,
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
        let provider_name = resolved.provider_name.clone();
        tokio::spawn(async move {
            let mut terminal_seen = false;
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        core.emit(CoreEvent::TurnCancelled { session_id });
                        let _ = turn_tx.send(CoreEvent::TurnCancelled { session_id }).await;
                        release_active_turn(&core.active_turns, session_id, active_turn_id).await;
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
                        core.persist_completed_trajectory(session_id, &event).await;
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
                            release_active_turn(&core.active_turns, session_id, active_turn_id)
                                .await;
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

            core.persist_provider_health(session_id, &provider_name)
                .await;
            release_active_turn(&core.active_turns, session_id, active_turn_id).await;
        });

        Ok(Box::pin(ReceiverStream::new(turn_rx)))
    }

    /// Cancels a currently active turn for a session.
    pub async fn cancel_turn(&self, session_id: SessionId) -> Result<(), CoreError> {
        if let Some(cancel) = self
            .active_turns
            .lock()
            .await
            .get(&session_id)
            .map(|active_turn| active_turn.cancel.clone())
        {
            cancel.cancel();
        }
        Ok(())
    }

    #[allow(clippy::result_large_err)]
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
            .or_else(|| project.config.default_loop.clone())
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
            provider_name,
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

    async fn persist_completed_trajectory(&self, session_id: SessionId, event: &RuntimeEvent) {
        let RuntimeEvent::AtifTrajectoryCompleted { trajectory } = event else {
            return;
        };
        if let Err(error) = self.upsert_trajectory(session_id, trajectory.clone()).await {
            self.emit(CoreEvent::Turn {
                session_id,
                event: RuntimeEvent::Error {
                    message: format!("failed to persist trajectory: {error}"),
                    recoverable: false,
                },
            });
        }
    }

    async fn persist_provider_health(&self, session_id: SessionId, provider_name: &str) {
        let Some(credential_activation) = &self.credential_activation else {
            return;
        };
        if let Err(error) = credential_activation
            .persist_provider_health(provider_name)
            .await
        {
            self.emit(CoreEvent::Turn {
                session_id,
                event: RuntimeEvent::Error {
                    message: format!("failed to persist credential health: {error}"),
                    recoverable: false,
                },
            });
        }
    }
}

impl AgentCore for AgentCoreNative {
    fn store(&self) -> &dyn Store {
        AgentCoreNative::store(self)
    }

    fn provider_names(&self) -> Vec<String> {
        AgentCoreNative::provider_names(self)
    }

    fn loop_names(&self) -> Vec<String> {
        AgentCoreNative::loop_names(self)
    }

    fn default_provider_name(&self) -> &str {
        AgentCoreNative::default_provider_name(self)
    }

    fn default_loop_name(&self) -> &str {
        AgentCoreNative::default_loop_name(self)
    }

    fn subscribe(&self) -> CoreEventStream {
        AgentCoreNative::subscribe(self)
    }

    fn resolve_or_create_project(
        &self,
        root: std::path::PathBuf,
    ) -> BoxFuture<'_, Result<Project, CoreError>> {
        Box::pin(AgentCoreNative::resolve_or_create_project(self, root))
    }

    fn create_session(&self, project_id: ProjectId) -> BoxFuture<'_, Result<Session, CoreError>> {
        Box::pin(AgentCoreNative::create_session(self, project_id))
    }

    fn project(&self, project_id: ProjectId) -> BoxFuture<'_, Result<Project, CoreError>> {
        Box::pin(AgentCoreNative::project(self, project_id))
    }

    fn session(&self, session_id: SessionId) -> BoxFuture<'_, Result<Session, CoreError>> {
        Box::pin(AgentCoreNative::session(self, session_id))
    }

    fn sessions_for_project(
        &self,
        project_id: ProjectId,
    ) -> BoxFuture<'_, Result<Vec<Session>, CoreError>> {
        Box::pin(AgentCoreNative::sessions_for_project(self, project_id))
    }

    fn update_session(
        &self,
        session_id: SessionId,
        update: SessionUpdate,
    ) -> BoxFuture<'_, Result<Session, CoreError>> {
        Box::pin(AgentCoreNative::update_session(self, session_id, update))
    }

    fn delete_session(&self, session_id: SessionId) -> BoxFuture<'_, Result<(), CoreError>> {
        Box::pin(AgentCoreNative::delete_session(self, session_id))
    }

    fn messages(
        &self,
        session_id: SessionId,
    ) -> BoxFuture<'_, Result<Vec<StoredMessage>, CoreError>> {
        Box::pin(AgentCoreNative::messages(self, session_id))
    }

    fn trajectory(
        &self,
        session_id: SessionId,
    ) -> BoxFuture<'_, Result<Option<atif::Trajectory>, CoreError>> {
        Box::pin(AgentCoreNative::trajectory(self, session_id))
    }

    fn upsert_trajectory(
        &self,
        session_id: SessionId,
        trajectory: atif::Trajectory,
    ) -> BoxFuture<'_, Result<atif::Trajectory, CoreError>> {
        Box::pin(AgentCoreNative::upsert_trajectory(
            self, session_id, trajectory,
        ))
    }

    fn effective_runtime_config(
        &self,
        session_id: SessionId,
    ) -> BoxFuture<'_, Result<RuntimeConfig, CoreError>> {
        Box::pin(AgentCoreNative::effective_runtime_config(self, session_id))
    }

    fn current_loop_name_for_session(
        &self,
        session_id: SessionId,
    ) -> BoxFuture<'_, Result<String, CoreError>> {
        Box::pin(AgentCoreNative::current_loop_name_for_session(
            self, session_id,
        ))
    }

    fn current_model_id_for_session(
        &self,
        session_id: SessionId,
    ) -> BoxFuture<'_, Result<Option<String>, CoreError>> {
        Box::pin(AgentCoreNative::current_model_id_for_session(
            self, session_id,
        ))
    }

    fn list_models(&self) -> Vec<ProviderModelInfo> {
        AgentCoreNative::list_models(self)
    }

    fn turn(
        &self,
        session_id: SessionId,
        input: Vec<Message>,
    ) -> BoxFuture<'_, Result<CoreEventStream, CoreError>> {
        Box::pin(AgentCoreNative::turn(self, session_id, input))
    }

    fn cancel_turn(&self, session_id: SessionId) -> BoxFuture<'_, Result<(), CoreError>> {
        Box::pin(AgentCoreNative::cancel_turn(self, session_id))
    }
}

struct ResolvedSessionRuntime {
    provider_name: String,
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

fn seed_project_prompt(
    system_prompt: Option<&str>,
    mut transcript: Vec<Message>,
    input: &[Message],
) -> Vec<Message> {
    let Some(system_prompt) = system_prompt else {
        return transcript;
    };
    let has_explicit_prompt = transcript.iter().chain(input.iter()).any(|message| {
        matches!(
            message.role,
            provider::MessageRole::System | provider::MessageRole::Developer
        )
    });
    if !has_explicit_prompt {
        transcript.insert(0, Message::system_text(system_prompt));
    }
    transcript
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
    pool: Arc<SharedCredentialPool>,
) -> Result<(), CoreError> {
    let credentials = store.credentials().list().await?;
    for ((provider_name, _credential_id), entry) in credentials {
        if providers.contains_key(&provider_name) {
            continue;
        }
        match entry.credential {
            agent_store::ProviderCredential::ApiKey { .. } => {
                let Some(preset) = OpenAiConfigPreset::by_name(&provider_name) else {
                    continue;
                };
                providers.insert(
                    provider_name.clone(),
                    Arc::new(OpenAiProvider::from_pool(
                        preset.into_config(String::new()),
                        pool.clone(),
                    )),
                );
            }
            agent_store::ProviderCredential::OAuth(_) => {
                let Some(preset) = OpenAiOAuthPreset::by_name(&provider_name) else {
                    continue;
                };
                providers.insert(
                    provider_name.clone(),
                    Arc::new(OpenAiOAuthProvider::from_pool(preset, pool.clone())),
                );
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::Duration;

    use async_stream::stream;
    use chrono::Duration as ChronoDuration;
    use futures::StreamExt;
    use provider::{
        Block, BlockDelta, BlockKind, CredentialFailure, Event, EventStream, FinishReason,
        MockProvider, ProviderInfo, Request, Usage,
    };
    use tempfile::TempDir;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::sync::oneshot;
    use tokio::time::timeout;

    use super::*;
    use agent_store::InMemoryStore;

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

    async fn spawn_refresh_server(body: String) -> (String, oneshot::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = oneshot::channel();

        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 8 * 1024];
            let mut request = Vec::new();
            let mut content_length = 0usize;

            loop {
                let n = socket.read(&mut buf).await.unwrap();
                if n == 0 {
                    break;
                }
                request.extend_from_slice(&buf[..n]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&request);
                    content_length = headers
                        .lines()
                        .find_map(|line| {
                            line.strip_prefix("content-length: ")
                                .or_else(|| line.strip_prefix("Content-Length: "))
                        })
                        .and_then(|value| value.trim().parse::<usize>().ok())
                        .unwrap_or(0);
                    break;
                }
            }

            if let Some(header_end) = request.windows(4).position(|window| window == b"\r\n\r\n") {
                let body_start = header_end + 4;
                while request.len().saturating_sub(body_start) < content_length {
                    let n = socket.read(&mut buf).await.unwrap();
                    if n == 0 {
                        break;
                    }
                    request.extend_from_slice(&buf[..n]);
                }
            }

            let _ = tx.send(String::from_utf8_lossy(&request).into_owned());

            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(response.as_bytes()).await.unwrap();
            socket.shutdown().await.unwrap();
        });

        (format!("http://{addr}/token"), rx)
    }

    #[tokio::test]
    async fn build_default_local_requires_explicit_provider_registration() {
        let store = Arc::new(InMemoryStore::new());
        let result = AgentCoreNative::build_default_local(store).await;

        assert!(matches!(result, Err(CoreError::NoProvidersRegistered)));
    }

    #[tokio::test]
    async fn trait_object_core_supports_projects_turns_and_events() {
        let store = Arc::new(InMemoryStore::new());
        let core: Arc<dyn AgentCore> = Arc::new(
            AgentCoreNative::builder(store)
                .with_provider("mock", Arc::new(MockProvider::new()))
                .build()
                .await
                .unwrap(),
        );

        let mut bus = core.subscribe();
        let project = core
            .resolve_or_create_project("/tmp/trait-object-core".into())
            .await
            .unwrap();
        let session = core.create_session(project.id).await.unwrap();
        let mut turn = core
            .turn(session.id, vec![Message::user_text("hello from dyn core")])
            .await
            .unwrap();

        let bus_event = timeout(Duration::from_secs(1), bus.next())
            .await
            .unwrap()
            .expect("expected a core bus event");
        assert!(matches!(
            bus_event,
            CoreEvent::Store { .. } | CoreEvent::Turn { .. }
        ));

        let finished = timeout(Duration::from_secs(1), async {
            while let Some(event) = turn.next().await {
                if matches!(
                    event,
                    CoreEvent::Turn {
                        event: RuntimeEvent::TurnFinished { .. },
                        ..
                    }
                ) {
                    return true;
                }
            }
            false
        })
        .await
        .unwrap();

        assert!(finished);
    }

    #[tokio::test]
    async fn resolve_or_create_project_normalizes_roots() {
        let store = Arc::new(InMemoryStore::new());
        let core = AgentCoreNative::builder(store)
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
    async fn resolve_or_create_project_loads_project_local_defaults() {
        let temp = TempDir::new().unwrap();
        let agents_dir = temp.path().join(".agents");
        std::fs::create_dir_all(&agents_dir).unwrap();
        std::fs::write(
            agents_dir.join("config.toml"),
            r#"[agent]
loop_name = "planner"
max_iterations = 64
max_retries = 2

[agent.inference]
provider = "mock"
model = "mock-model"
reasoning = "high"
max_tokens = 512
"#,
        )
        .unwrap();
        std::fs::write(agents_dir.join("AGENTS.md"), "root prompt").unwrap();

        let store = Arc::new(InMemoryStore::new());
        let core = AgentCoreNative::builder(store)
            .with_provider("mock", Arc::new(MockProvider::new()))
            .with_loop("planner", Arc::new(SimpleLoop))
            .build()
            .await
            .unwrap();

        let project = core.resolve_or_create_project(temp.path()).await.unwrap();

        assert_eq!(project.config.default_loop.as_deref(), Some("planner"));
        assert_eq!(project.config.default_provider.as_deref(), Some("mock"));
        assert_eq!(project.config.default_model.as_deref(), Some("mock-model"));
        assert_eq!(project.config.runtime.max_iterations, 64);
        assert_eq!(project.config.runtime.max_retries, 2);
        assert_eq!(project.config.runtime.request.max_output_tokens, Some(512));
        assert_eq!(project.config.system_prompt.as_deref(), Some("root prompt"));
        assert_eq!(
            project
                .config
                .runtime
                .request
                .reasoning
                .as_ref()
                .and_then(|reasoning| reasoning.effort.as_deref()),
            Some("high")
        );
    }

    #[tokio::test]
    async fn resolve_or_create_project_prefers_explicit_prompt_over_agents() {
        let temp = TempDir::new().unwrap();
        let agents_dir = temp.path().join(".agents");
        std::fs::create_dir_all(&agents_dir).unwrap();
        std::fs::write(
            agents_dir.join("config.toml"),
            "[agent]\nsystem_prompt = \"config prompt\"\n",
        )
        .unwrap();
        std::fs::write(agents_dir.join("AGENTS.md"), "root prompt").unwrap();

        let store = Arc::new(InMemoryStore::new());
        let core = AgentCoreNative::builder(store)
            .with_provider("mock", Arc::new(MockProvider::new()))
            .build()
            .await
            .unwrap();

        let project = core.resolve_or_create_project(temp.path()).await.unwrap();
        assert_eq!(
            project.config.system_prompt.as_deref(),
            Some("config prompt")
        );
    }

    #[tokio::test]
    async fn current_loop_name_uses_project_default_loop() {
        let store = Arc::new(InMemoryStore::new());
        let core = AgentCoreNative::builder(store.clone())
            .with_provider("mock", Arc::new(MockProvider::new()))
            .with_loop("planner", Arc::new(SimpleLoop))
            .build()
            .await
            .unwrap();

        let project = store
            .projects()
            .create(Project::new(
                Some("demo".into()),
                Some("/tmp/demo".into()),
                ProjectConfig {
                    default_loop: Some("planner".into()),
                    ..ProjectConfig::default()
                },
            ))
            .await
            .unwrap();
        let session = core.create_session(project.id).await.unwrap();

        let loop_name = core
            .current_loop_name_for_session(session.id)
            .await
            .unwrap();
        assert_eq!(loop_name, "planner");
    }

    #[tokio::test]
    async fn turn_persists_transcript_back_to_store() {
        let store = Arc::new(InMemoryStore::new());
        let core = AgentCoreNative::builder(store.clone())
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
    async fn turn_seeds_project_prompt_once_and_preserves_original_prompt() {
        let store = Arc::new(InMemoryStore::new());
        let core = AgentCoreNative::builder(store.clone())
            .with_provider("mock", Arc::new(MockProvider::new()))
            .build()
            .await
            .unwrap();
        let project = store
            .projects()
            .create(Project::new(
                Some("demo".into()),
                Some("/tmp/prompt-demo".into()),
                ProjectConfig {
                    system_prompt: Some("prompt a".into()),
                    ..ProjectConfig::default()
                },
            ))
            .await
            .unwrap();
        let session = core.create_session(project.id).await.unwrap();

        let mut first = core
            .turn(session.id, vec![Message::user_text("hello there")])
            .await
            .unwrap();
        while let Some(event) = timeout(Duration::from_secs(1), first.next()).await.unwrap() {
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

        let mut updated_project = project.clone();
        updated_project.config.system_prompt = Some("prompt b".into());
        store
            .projects()
            .update(project.id, updated_project)
            .await
            .unwrap();

        let mut second = core
            .turn(session.id, vec![Message::user_text("follow up")])
            .await
            .unwrap();
        while let Some(event) = timeout(Duration::from_secs(1), second.next())
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
        assert_eq!(messages[0].message.role, provider::MessageRole::System);
        assert_eq!(messages[0].message.plain_text_lossy(), "prompt a");
        let system_messages = messages
            .iter()
            .filter(|message| message.message.role == provider::MessageRole::System)
            .count();
        assert_eq!(system_messages, 1);
    }

    #[tokio::test]
    async fn cancel_turn_emits_turn_cancelled() {
        let store = Arc::new(InMemoryStore::new());
        let core = AgentCoreNative::builder(store.clone())
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
                CredentialEntry::api_key("Primary", "sk-test"),
            )
            .await
            .unwrap();

        let core = AgentCoreNative::build_default_local(store).await.unwrap();
        assert!(core.provider_names().contains(&"openai".to_string()));
    }

    #[tokio::test]
    async fn credential_activation_mirrors_store_entries_into_shared_pool() {
        let store = Arc::new(InMemoryStore::new());
        let mut entry = CredentialEntry::api_key("Primary", "sk-test");
        entry.health.record_error("previous failure", None);
        store
            .credentials()
            .create(("openai".into(), "store-key".into()), entry)
            .await
            .unwrap();

        let pool = Arc::new(SharedCredentialPool::new(Arc::new(StickyRoundRobin::new())));
        let activation = CredentialActivation::new(store, pool.clone());
        activation.sync_provider("openai").await.unwrap();

        let entries = pool.entries("openai").await;
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "store-key");
        assert_eq!(entries[0].health.consecutive_errors, 1);
        assert_eq!(
            entries[0].health.last_error.as_ref().unwrap().message,
            "previous failure"
        );
    }

    #[tokio::test]
    async fn credential_activation_persists_pool_health_back_to_store() {
        let store = Arc::new(InMemoryStore::new());
        store
            .credentials()
            .create(
                ("openai".into(), "store-key".into()),
                CredentialEntry::api_key("Primary", "sk-test"),
            )
            .await
            .unwrap();

        let pool = Arc::new(SharedCredentialPool::new(Arc::new(StickyRoundRobin::new())));
        let activation = CredentialActivation::new(store.clone(), pool.clone());
        activation.sync_provider("openai").await.unwrap();
        pool.mark_error(
            "openai",
            "store-key",
            CredentialFailure::new("rate limited").with_code("429"),
        )
        .await;
        activation.persist_provider_health("openai").await.unwrap();

        let stored = store
            .credentials()
            .get(("openai".into(), "store-key".into()))
            .await
            .unwrap();
        assert_eq!(stored.health.consecutive_errors, 1);
        assert_eq!(
            stored.health.last_error.as_ref().unwrap().message,
            "rate limited"
        );
        assert_eq!(
            stored.health.last_error.as_ref().unwrap().code.as_deref(),
            Some("429")
        );
    }

    #[tokio::test]
    async fn credential_activation_refreshes_expiring_openai_oauth_credentials() {
        let store = Arc::new(InMemoryStore::new());
        let (token_endpoint, request_rx) = spawn_refresh_server(
            serde_json::json!({
                "access_token": "new-token",
                "refresh_token": "new-refresh",
                "expires_in": 3600,
                "token_type": "Bearer",
                "scope": "openid profile offline_access"
            })
            .to_string(),
        )
        .await;
        let oauth = agent_store::OAuthCredentials {
            access_token: "old-token".into(),
            refresh_token: "old-refresh".into(),
            client_id: "client-id".into(),
            token_endpoint,
            account_id: Some("acct-123".into()),
            token_type: Some("Bearer".into()),
            expires_at: Utc::now() - ChronoDuration::minutes(5),
            scopes: vec!["openid".into()],
        };
        store
            .credentials()
            .create(
                ("openai-oauth".into(), "oauth-key".into()),
                CredentialEntry::oauth("OAuth", oauth),
            )
            .await
            .unwrap();

        let pool = Arc::new(SharedCredentialPool::new(Arc::new(StickyRoundRobin::new())));
        let activation = CredentialActivation::new(store.clone(), pool.clone());
        activation.sync_provider("openai-oauth").await.unwrap();

        let request = request_rx.await.unwrap();
        assert!(request.contains("POST /token HTTP/1.1"));
        assert!(request.contains("grant_type=refresh_token"));
        assert!(request.contains("client_id=client-id"));
        assert!(request.contains("refresh_token=old-refresh"));

        let stored = store
            .credentials()
            .get(("openai-oauth".into(), "oauth-key".into()))
            .await
            .unwrap();
        let agent_store::ProviderCredential::OAuth(updated) = stored.credential else {
            panic!("expected oauth credential");
        };
        assert_eq!(updated.access_token, "new-token");
        assert_eq!(updated.refresh_token, "new-refresh");

        let pooled = pool.entries("openai-oauth").await;
        assert_eq!(pooled.len(), 1);
        assert_eq!(
            pooled[0]
                .material
                .headers
                .get("Authorization")
                .map(String::as_str),
            Some("Bearer new-token")
        );
        assert_eq!(
            pooled[0]
                .material
                .metadata
                .get("account_id")
                .and_then(|value| value.as_str()),
            Some("acct-123")
        );
    }

    #[tokio::test]
    async fn trajectories_roundtrip_through_core() {
        let store = Arc::new(InMemoryStore::new());
        let core = AgentCoreNative::builder(store.clone())
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
    async fn completed_trajectory_event_persists_through_core() {
        let store = Arc::new(InMemoryStore::new());
        let core = AgentCoreNative::builder(store.clone())
            .with_provider("mock", Arc::new(MockProvider::new()))
            .build()
            .await
            .unwrap();
        let project = core
            .resolve_or_create_project("/tmp/trajectory-event-core")
            .await
            .unwrap();
        let session = core.create_session(project.id).await.unwrap();
        let trajectory = sample_trajectory(session.id);

        core.persist_completed_trajectory(
            session.id,
            &RuntimeEvent::AtifTrajectoryCompleted {
                trajectory: trajectory.clone(),
            },
        )
        .await;

        let loaded = core.trajectory(session.id).await.unwrap().unwrap();
        assert_eq!(loaded, trajectory);
    }

    #[tokio::test]
    async fn persist_completed_trajectory_ignores_non_trajectory_events() {
        let store = Arc::new(InMemoryStore::new());
        let core = AgentCoreNative::builder(store.clone())
            .with_provider("mock", Arc::new(MockProvider::new()))
            .build()
            .await
            .unwrap();
        let project = core
            .resolve_or_create_project("/tmp/trajectory-persist-core")
            .await
            .unwrap();
        let session = core.create_session(project.id).await.unwrap();
        let trajectory = sample_trajectory(session.id);

        core.upsert_trajectory(session.id, trajectory.clone())
            .await
            .unwrap();

        let event = RuntimeEvent::TurnFinished {
            session_id: session.id,
            turn_index: 0,
            finish_reason: Some(FinishReason::Stop),
        };
        core.persist_completed_trajectory(session.id, &event).await;

        let loaded = core.trajectory(session.id).await.unwrap().unwrap();
        assert_eq!(loaded, trajectory);
    }

    #[tokio::test]
    async fn file_store_default_core_roundtrips() {
        let temp = TempDir::new().unwrap();
        let store = Arc::new(agent_store::FileStore::new(temp.path()).await.unwrap());
        let core = AgentCoreNative::builder(store.clone())
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
        let core = AgentCoreNative::builder(store.clone())
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
