use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use futures::{StreamExt, future::BoxFuture};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_util::sync::CancellationToken;
use ulid::Ulid;

use brain_types::*;

use crate::brain::{
    load_agent_config_for_session, resolve_or_create_project_with_store_status,
    run_turn_with_config, send_error_event,
};

const RUNTIME_BUS_CAPACITY: usize = 1024;

pub struct BrainRuntimeNative {
    store: Arc<dyn Store>,
    providers: Arc<RegistryProviderHashMap>,
    tools: Arc<RegistryToolHashMap>,
    loops: Arc<RegistryLoopHashMap>,
    default_provider_name: String,
    default_loop_name: String,
    active_turns: Arc<Mutex<HashMap<Ulid, CancellationToken>>>,
    runtime_bus: broadcast::Sender<RuntimeBusEvent>,
}

impl BrainRuntimeNative {
    pub fn new(
        store: Arc<dyn Store>,
        default_provider_name: impl Into<String>,
        default_loop_name: impl Into<String>,
    ) -> Self {
        let (runtime_bus, _) = broadcast::channel(RUNTIME_BUS_CAPACITY);
        Self {
            store,
            providers: Arc::new(RegistryProviderHashMap::default()),
            tools: Arc::new(RegistryToolHashMap::default()),
            loops: Arc::new(RegistryLoopHashMap::default()),
            default_provider_name: default_provider_name.into(),
            default_loop_name: default_loop_name.into(),
            active_turns: Arc::new(Mutex::new(HashMap::new())),
            runtime_bus,
        }
    }

    pub fn set_provider(
        &self,
        name: impl Into<String>,
        provider: Arc<dyn Provider>,
    ) -> Result<Option<Arc<dyn Provider>>, BrainError> {
        self.providers.set(name.into(), provider)
    }

    pub fn set_tool(
        &self,
        name: impl Into<String>,
        tool: Arc<dyn Tool>,
    ) -> Result<Option<Arc<dyn Tool>>, BrainError> {
        self.tools.set(name.into(), tool)
    }

    pub fn set_loop(
        &self,
        name: impl Into<String>,
        agent_loop: Arc<dyn AgentLoop>,
    ) -> Result<Option<Arc<dyn AgentLoop>>, BrainError> {
        self.loops.set(name.into(), agent_loop)
    }
}

impl BrainRuntime for BrainRuntimeNative {
    fn store(&self) -> &dyn Store {
        self.store.as_ref()
    }

    fn providers(&self) -> &dyn RegistryProvider {
        self.providers.as_ref()
    }

    fn tools(&self) -> &dyn RegistryTool {
        self.tools.as_ref()
    }

    fn loops(&self) -> &dyn RegistryLoop {
        self.loops.as_ref()
    }

    fn resolve_or_create_project(
        &self,
        root: std::path::PathBuf,
    ) -> BoxFuture<'_, Result<Project, BrainError>> {
        let store = self.store.clone();
        Box::pin(async move {
            resolve_or_create_project_with_store_status(store, root)
                .await
                .map(|(project, _)| project)
        })
    }

    fn subscribe(&self) -> RuntimeBusStream {
        let turn_events =
            BroadcastStream::new(self.runtime_bus.subscribe()).filter_map(|result| async move {
                match result {
                    Ok(event) => Some(event),
                    Err(error) => {
                        tracing::warn!("runtime bus receive error: {error}");
                        None
                    }
                }
            });
        let store_events = self.store.subscribe().map(map_store_event);
        Box::pin(futures::stream::select(store_events, turn_events))
    }

    fn turn(&self, session_id: Ulid, input: &str, cancel: CancellationToken) -> EventStream {
        {
            let mut active_turns = self.active_turns.lock().unwrap();
            if active_turns.contains_key(&session_id) {
                return error_stream(BrainError::TurnActive(session_id));
            }
            active_turns.insert(session_id, cancel.clone());
        }

        let store = self.store.clone();
        let providers = self.providers.clone();
        let tools = self.tools.clone();
        let loops = self.loops.clone();
        let active_turns = self.active_turns.clone();
        let default_provider_name = self.default_provider_name.clone();
        let default_loop_name = self.default_loop_name.clone();
        let runtime_bus = self.runtime_bus.clone();
        let user_msg = Message::user(input);

        let (raw_tx, mut raw_rx) = tokio::sync::mpsc::channel::<Event>(256);
        let (client_tx, client_rx) = tokio::sync::mpsc::channel::<Event>(256);

        tokio::spawn(async move {
            while let Some(event) = raw_rx.recv().await {
                let _ = runtime_bus.send(RuntimeBusEvent::Turn {
                    session_id,
                    event: event.clone(),
                });
                if client_tx.send(event).await.is_err() {
                    break;
                }
            }
        });

        tokio::spawn(async move {
            let result = async {
                let config = load_agent_config_for_session(store.clone(), session_id).await?;
                let provider =
                    resolve_provider(&providers, &default_provider_name, &config.inference)?;
                let agent_loop = resolve_loop(&loops, &default_loop_name, &config)?;
                let tools = resolve_tools(&tools)?;

                run_turn_with_config(
                    store,
                    provider,
                    agent_loop,
                    tools,
                    config,
                    cancel,
                    session_id,
                    user_msg,
                    raw_tx.clone(),
                )
                .await
            }
            .await;

            active_turns.lock().unwrap().remove(&session_id);

            if let Err(error) = result {
                send_error_event(raw_tx, error).await;
            }
        });

        Box::pin(tokio_stream::wrappers::ReceiverStream::new(client_rx))
    }

    fn cancel_turn(&self, session_id: Ulid) -> BoxFuture<'_, Result<(), BrainError>> {
        Box::pin(async move {
            if let Some(cancel) = self.active_turns.lock().unwrap().get(&session_id).cloned() {
                cancel.cancel();
            }
            Ok(())
        })
    }

    fn effective_inference_for_session(
        &self,
        session_id: Ulid,
    ) -> BoxFuture<'_, Result<InferenceConfig, BrainError>> {
        let store = self.store.clone();
        Box::pin(async move {
            let config = load_agent_config_for_session(store, session_id).await?;
            Ok(config.inference)
        })
    }

    fn effective_agent_config_for_session(
        &self,
        session_id: Ulid,
    ) -> BoxFuture<'_, Result<AgentConfig, BrainError>> {
        Box::pin(load_agent_config_for_session(
            self.store.clone(),
            session_id,
        ))
    }

    fn current_loop_name_for_session(
        &self,
        session_id: Ulid,
    ) -> BoxFuture<'_, Result<Option<String>, BrainError>> {
        let store = self.store.clone();
        let default_loop_name = self.default_loop_name.clone();
        Box::pin(async move {
            let session = store.sessions().get(session_id).await?;
            if let Some(loop_name) = session.loop_name {
                return Ok(Some(loop_name));
            }

            let project = store.projects().get(session.project_id).await?;
            Ok(project
                .config
                .agent
                .loop_name
                .or(Some(default_loop_name)))
        })
    }

    fn current_model_id_for_session(
        &self,
        session_id: Ulid,
    ) -> BoxFuture<'_, Result<Option<String>, BrainError>> {
        Box::pin(async move {
            self.current_model_for_session(session_id)
                .await
                .map(|model| Some(model.model.id.to_owned()))
        })
    }

    fn current_model_for_session(
        &self,
        session_id: Ulid,
    ) -> BoxFuture<'_, Result<ProviderModelInfo, BrainError>> {
        let store = self.store.clone();
        let providers = self.providers.clone();
        let default_provider_name = self.default_provider_name.clone();
        Box::pin(async move {
            let config = load_agent_config_for_session(store, session_id).await?;
            let (provider_name, provider) = if let Some(provider_name) = config.inference.provider {
                let provider = providers.get(&provider_name).ok_or_else(|| {
                    BrainError::Internal(format!("provider not registered: {provider_name}"))
                })?;
                (provider_name, provider)
            } else if let Some(model_id) = config.inference.model.as_deref() {
                let matches = providers
                    .list()?
                    .into_iter()
                    .filter_map(|(provider_name, provider)| {
                        provider
                            .info()
                            .models
                            .iter()
                            .any(|model| model.id == model_id)
                            .then_some((provider_name, provider))
                    })
                    .collect::<Vec<_>>();
                match matches.as_slice() {
                    [] | [_, _, ..] => {
                        return Err(BrainError::Internal(format!(
                            "unknown or ambiguous model: {model_id}"
                        )));
                    }
                    [(provider_name, provider)] => (provider_name.clone(), provider.clone()),
                }
            } else {
                let provider = providers.get(&default_provider_name).ok_or_else(|| {
                    BrainError::Internal(format!(
                        "provider not registered: {default_provider_name}"
                    ))
                })?;
                (default_provider_name, provider)
            };

            let provider_info = provider.info();
            let model_id = config
                .inference
                .model
                .or(provider_info.default_model.clone())
                .ok_or_else(|| {
                    BrainError::Internal(format!(
                        "provider {provider_name} does not expose a default model"
                    ))
                })?;
            let model = provider_info
                .models
                .iter()
                .find(|model| model.id == model_id)
                .copied()
                .ok_or_else(|| {
                    BrainError::Internal(format!(
                        "model {model_id} is not available for provider {provider_name}"
                    ))
                })?;

            Ok(ProviderModelInfo {
                provider: provider_name,
                model,
            })
        })
    }

    fn set_session_model(
        &self,
        session_id: Ulid,
        model_id: &str,
    ) -> BoxFuture<'_, Result<Session, BrainError>> {
        let model_id = model_id.to_owned();
        let providers = self.providers.clone();
        let store = self.store.clone();
        Box::pin(async move {
            let matches = providers
                .list()?
                .into_iter()
                .filter_map(|(provider_name, provider)| {
                    provider
                        .info()
                        .models
                        .into_iter()
                        .find(|model| model.id == model_id)
                        .map(|model| (provider_name, model))
                })
                .collect::<Vec<_>>();

            let (provider_name, selected_model) = match matches.as_slice() {
                [] | [_, _, ..] => {
                    return Err(BrainError::Internal(format!(
                        "unknown or ambiguous model: {model_id}"
                    )));
                }
                [(provider_name, selected_model)] => (provider_name.clone(), *selected_model),
            };

            let mut session = store.sessions().get(session_id).await?;
            let mut inference = session.inference.unwrap_or_default();
            inference.provider = Some(provider_name);
            inference.model = Some(model_id);
            let keep_reasoning = match (inference.reasoning.as_deref(), selected_model.reasoning) {
                (None, _) => true,
                (Some(value), Some(levels)) => levels.contains(&value),
                (Some(_), None) => false,
            };
            if !keep_reasoning {
                inference.reasoning = None;
            }
            session.inference = Some(inference);

            store.sessions().update(session.id, session).await
        })
    }

    fn set_session_thought_level(
        &self,
        session_id: Ulid,
        thought_level: &str,
    ) -> BoxFuture<'_, Result<Session, BrainError>> {
        let thought_level = thought_level.to_owned();
        Box::pin(async move {
            let current_model = self.current_model_for_session(session_id).await?;
            let supported = current_model.model.reasoning.ok_or_else(|| {
                BrainError::Internal(format!(
                    "model {} does not support thought level configuration",
                    current_model.model.id
                ))
            })?;
            if !supported.contains(&thought_level.as_str()) {
                return Err(BrainError::Internal(format!(
                    "invalid thought level {thought_level} for model {}",
                    current_model.model.id
                )));
            }

            let mut session = self.store.sessions().get(session_id).await?;
            let mut inference = session.inference.unwrap_or_default();
            inference.reasoning = Some(thought_level);
            session.inference = Some(inference);

            self.store.sessions().update(session.id, session).await
        })
    }
}

fn error_stream(error: BrainError) -> EventStream {
    Box::pin(futures::stream::iter(vec![Event::Error {
        code: error.code(),
        message: error.to_string(),
        recoverable: error.recoverable(),
    }]))
}

fn resolve_provider(
    providers: &RegistryProviderHashMap,
    default_provider_name: &str,
    config: &InferenceConfig,
) -> Result<Arc<dyn Provider>, BrainError> {
    if let Some(provider_name) = config.provider.as_deref() {
        let provider = providers
            .get(provider_name)
            .ok_or_else(|| BrainError::Internal(format!("unknown provider: {provider_name}")))?;
        let info = provider.info();

        if let Some(model_id) = config.model.as_deref()
            && !info.models.is_empty()
            && !info.models.iter().any(|model| model.id == model_id)
        {
            return Err(BrainError::Internal(format!(
                "model {model_id} is not available for provider {provider_name}"
            )));
        }

        return Ok(provider);
    }

    if let Some(model_id) = config.model.as_deref() {
        let matching = providers
            .list()?
            .into_iter()
            .filter(|(_, provider)| {
                provider
                    .info()
                    .models
                    .iter()
                    .any(|model| model.id == model_id)
            })
            .collect::<Vec<_>>();

        return match matching.as_slice() {
            [] => Err(BrainError::Internal(format!(
                "unknown or ambiguous model: {model_id}"
            ))),
            [(_, provider)] => Ok(provider.clone()),
            _ => Err(BrainError::Internal(format!(
                "unknown or ambiguous model: {model_id}"
            ))),
        };
    }

    providers.get(default_provider_name).ok_or_else(|| {
        BrainError::Internal(format!("provider not registered: {default_provider_name}"))
    })
}

fn resolve_loop(
    loops: &RegistryLoopHashMap,
    default_loop_name: &str,
    config: &AgentConfig,
) -> Result<Arc<dyn AgentLoop>, BrainError> {
    let loop_name = config
        .loop_name
        .as_deref()
        .unwrap_or(default_loop_name)
        .to_owned();

    loops
        .get(&loop_name)
        .ok_or_else(|| BrainError::Internal(format!("loop not registered: {loop_name}")))
}

fn resolve_tools(tools: &RegistryToolHashMap) -> Result<Vec<Arc<dyn Tool>>, BrainError> {
    let mut tools = tools.list()?;
    tools.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(tools.into_iter().map(|(_, tool)| tool).collect())
}

fn map_store_event(event: StoreEvent) -> RuntimeBusEvent {
    match event {
        StoreEvent::ProjectCreated { project } => RuntimeBusEvent::ProjectCreated { project },
        StoreEvent::ProjectUpdated { project } => RuntimeBusEvent::ProjectUpdated { project },
        StoreEvent::ProjectDeleted { project_id } => RuntimeBusEvent::ProjectDeleted { project_id },
        StoreEvent::SessionCreated { session } => RuntimeBusEvent::SessionCreated { session },
        StoreEvent::SessionUpdated { session } => RuntimeBusEvent::SessionUpdated { session },
        StoreEvent::SessionDeleted {
            session_id,
            project_id,
        } => RuntimeBusEvent::SessionDeleted {
            session_id,
            project_id,
        },
        StoreEvent::TrajectoryCreated {
            session_id,
            trajectory,
        } => RuntimeBusEvent::TrajectoryCreated {
            session_id,
            trajectory,
        },
        StoreEvent::TrajectoryUpdated {
            session_id,
            trajectory,
        } => RuntimeBusEvent::TrajectoryUpdated {
            session_id,
            trajectory,
        },
        StoreEvent::TrajectoryDeleted { session_id } => {
            RuntimeBusEvent::TrajectoryDeleted { session_id }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use async_stream::stream;
    use brain_loops::SimpleLoop;
    use brain_stores::InMemoryStore;
    use futures::StreamExt;
    use futures::future::BoxFuture;
    use tempfile::tempdir;

    use super::*;

    struct NamedProvider {
        name: &'static str,
        models: Vec<(&'static str, Option<&'static [&'static str]>)>,
    }

    impl NamedProvider {
        fn new(name: &'static str, models: Vec<&'static str>) -> Self {
            Self {
                name,
                models: models.into_iter().map(|model| (model, None)).collect(),
            }
        }

        fn with_reasoning(
            name: &'static str,
            models: Vec<(&'static str, Option<&'static [&'static str]>)>,
        ) -> Self {
            Self { name, models }
        }
    }

    impl Provider for NamedProvider {
        fn chat<'a>(
            &'a self,
            _messages: &'a [Message],
            _tools: &'a [ToolDef],
            _config: &'a InferenceConfig,
            _session_id: Option<Ulid>,
        ) -> BoxFuture<'a, Result<ChatStream, BrainError>> {
            Box::pin(async { Ok(Box::pin(futures::stream::empty()) as ChatStream) })
        }

        fn info(&self) -> ProviderInfo {
            ProviderInfo {
                name: self.name.into(),
                default_model: self
                    .models
                    .first()
                    .map(|(model, _reasoning)| (*model).to_owned()),
                models: self
                    .models
                    .iter()
                    .map(|(model, reasoning)| ModelInfo {
                        id: model,
                        name: model,
                        family: None,
                        reasoning: *reasoning,
                        tool_call: true,
                        attachment: false,
                        structured_output: None,
                        temperature: None,
                        knowledge: None,
                        release_date: None,
                        last_updated: None,
                        open_weights: None,
                        input_modalities: &["text"],
                        output_modalities: &["text"],
                        cost: None,
                        limit: None,
                        status: None,
                    })
                    .collect(),
            }
        }
    }

    struct NamedTool(&'static str);

    impl Tool for NamedTool {
        fn definition(&self) -> ToolDef {
            ToolDef {
                name: self.0.into(),
                description: self.0.into(),
                parameters: serde_json::json!({}),
            }
        }

        fn execute(&self, _args: serde_json::Value) -> BoxFuture<'_, Result<String, BrainError>> {
            Box::pin(async { Ok(self.0.to_owned()) })
        }
    }

    struct RecordingLoop {
        name: &'static str,
    }

    impl AgentLoop for RecordingLoop {
        fn run(
            &self,
            provider: Arc<dyn Provider>,
            tools: Vec<Arc<dyn Tool>>,
            _messages: Vec<Message>,
            _config: AgentConfig,
            _cancel: CancellationToken,
            _session_id: Option<Ulid>,
        ) -> EventStream {
            let provider_name = provider.info().name;
            let tool_names = tools
                .iter()
                .map(|tool| tool.definition().name)
                .collect::<Vec<_>>()
                .join(",");
            let content = format!(
                "provider={provider_name};loop={};tools={tool_names}",
                self.name
            );

            Box::pin(futures::stream::iter(vec![
                Event::MessageDone {
                    message: Message::assistant(content),
                },
                Event::TurnDone {
                    iterations: 1,
                    prompt_tokens: None,
                    completion_tokens: None,
                    cache_read_tokens: None,
                    cache_write_tokens: None,
                    reasoning_tokens: None,
                    total_tokens: 0,
                },
            ]))
        }
    }

    struct WaitingLoop;

    impl AgentLoop for WaitingLoop {
        fn run(
            &self,
            _provider: Arc<dyn Provider>,
            _tools: Vec<Arc<dyn Tool>>,
            _messages: Vec<Message>,
            _config: AgentConfig,
            cancel: CancellationToken,
            _session_id: Option<Ulid>,
        ) -> EventStream {
            Box::pin(stream! {
                loop {
                    if cancel.is_cancelled() {
                        yield Event::Error {
                            code: BrainErrorCode::Cancelled,
                            message: BrainError::Cancelled.to_string(),
                            recoverable: true,
                        };
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
        }
    }

    async fn make_runtime(
        default_provider_name: &str,
        default_loop_name: &str,
        project: Project,
    ) -> (BrainRuntimeNative, Arc<InMemoryStore>, Project) {
        let store = Arc::new(InMemoryStore::new());
        let runtime =
            BrainRuntimeNative::new(store.clone(), default_provider_name, default_loop_name);
        let project = store.projects().create(project.id, project).await.unwrap();
        (runtime, store, project)
    }

    async fn run_turn(runtime: &BrainRuntimeNative, session_id: Ulid, input: &str) -> Vec<Event> {
        let mut events = runtime.turn(session_id, input, CancellationToken::new());
        let mut out = Vec::new();
        while let Some(event) = events.next().await {
            out.push(event);
        }
        out
    }

    #[tokio::test]
    async fn resolve_or_create_project_reuses_existing_root() {
        let (runtime, store, _) =
            make_runtime("default", "simple", Project::with_defaults("test")).await;
        let project = Project::new(
            Some("repo".into()),
            Some("/tmp/work/repo".into()),
            ProjectConfig::default(),
        );
        let id = project.id;
        store.projects().create(project.id, project).await.unwrap();

        let resolved = runtime
            .resolve_or_create_project("/tmp/work/./repo".into())
            .await
            .unwrap();
        assert_eq!(resolved.id, id);
    }

    #[tokio::test]
    async fn subscribe_receives_duplexed_turn_events() {
        let (runtime, store, project) =
            make_runtime("default", "simple", Project::with_defaults("test")).await;
        runtime
            .set_provider(
                "default",
                Arc::new(NamedProvider::new("default", vec!["model-a"])),
            )
            .unwrap();
        runtime
            .set_loop("simple", Arc::new(RecordingLoop { name: "simple" }))
            .unwrap();

        let session = Session::new(project.id);
        let session = store.sessions().create(session.id, session).await.unwrap();
        let mut bus = runtime.subscribe();
        let _ = run_turn(&runtime, session.id, "hello").await;

        let first = tokio::time::timeout(Duration::from_secs(1), bus.next())
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(
            first,
            RuntimeBusEvent::Turn {
                session_id,
                event: Event::MessageDone { .. }
            } if session_id == session.id
        ));
    }

    #[tokio::test]
    async fn resolve_or_create_project_publishes_project_created() {
        let (runtime, _store, _project) =
            make_runtime("default", "simple", Project::with_defaults("test")).await;
        let dir = tempdir().unwrap();
        let mut bus = runtime.subscribe();

        let created = runtime
            .resolve_or_create_project(dir.path().join("repo"))
            .await
            .unwrap();

        let event = tokio::time::timeout(Duration::from_secs(1), bus.next())
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(
            event,
            RuntimeBusEvent::ProjectCreated { project } if project.id == created.id
        ));
    }

    #[tokio::test]
    async fn turn_uses_default_provider_and_loop_and_sorted_tools() {
        let (runtime, store, project) =
            make_runtime("default", "simple", Project::with_defaults("test")).await;
        runtime
            .set_provider(
                "default",
                Arc::new(NamedProvider::new("default", vec!["model-a"])),
            )
            .unwrap();
        runtime
            .set_loop("simple", Arc::new(RecordingLoop { name: "simple" }))
            .unwrap();
        runtime.set_tool("b", Arc::new(NamedTool("b"))).unwrap();
        runtime.set_tool("a", Arc::new(NamedTool("a"))).unwrap();

        let session = Session::new(project.id);
        let session = store.sessions().create(session.id, session).await.unwrap();
        let _ = run_turn(&runtime, session.id, "hello").await;

        let messages = store.messages().list_for_session(session.id).await.unwrap();
        assert_eq!(
            messages[1].content,
            MessageContent::text("provider=default;loop=simple;tools=a,b")
        );
    }

    #[tokio::test]
    async fn turn_resolves_provider_from_model_without_router() {
        let (runtime, store, project) = make_runtime(
            "provider-a",
            "simple",
            Project::new(
                Some("test".into()),
                None,
                ProjectConfig {
                    agent: AgentConfig {
                        max_iterations: 20,
                        system_prompt: None,
                        loop_name: None,
                        inference: InferenceConfig {
                            provider: None,
                            model: Some("model-b".into()),
                            reasoning: None,
                            max_tokens: None,
                            temperature: None,
                        },
                        ..AgentConfig::default()
                    },
                },
            ),
        )
        .await;
        runtime
            .set_provider(
                "provider-a",
                Arc::new(NamedProvider::new("provider-a", vec!["model-a"])),
            )
            .unwrap();
        runtime
            .set_provider(
                "provider-b",
                Arc::new(NamedProvider::new("provider-b", vec!["model-b"])),
            )
            .unwrap();
        runtime
            .set_loop("simple", Arc::new(RecordingLoop { name: "simple" }))
            .unwrap();

        let session = Session::new(project.id);
        let session = store.sessions().create(session.id, session).await.unwrap();
        let _ = run_turn(&runtime, session.id, "hello").await;

        let messages = store.messages().list_for_session(session.id).await.unwrap();
        assert!(
            messages[1]
                .content
                .to_string()
                .contains("provider=provider-b")
        );
    }

    #[tokio::test]
    async fn turn_uses_session_loop_override() {
        let (runtime, store, project) = make_runtime(
            "default",
            "simple",
            Project::new(
                Some("test".into()),
                None,
                ProjectConfig {
                    agent: AgentConfig {
                        max_iterations: 20,
                        system_prompt: None,
                        loop_name: Some("default-loop".into()),
                        inference: InferenceConfig::default(),
                        ..AgentConfig::default()
                    },
                },
            ),
        )
        .await;
        runtime
            .set_provider(
                "default",
                Arc::new(NamedProvider::new("default", vec!["model-a"])),
            )
            .unwrap();
        runtime
            .set_loop(
                "default-loop",
                Arc::new(RecordingLoop {
                    name: "default-loop",
                }),
            )
            .unwrap();
        runtime
            .set_loop(
                "override-loop",
                Arc::new(RecordingLoop {
                    name: "override-loop",
                }),
            )
            .unwrap();

        let mut session = Session::new(project.id);
        session.loop_name = Some("override-loop".into());
        let session = store.sessions().create(session.id, session).await.unwrap();

        let _ = run_turn(&runtime, session.id, "hello").await;
        let messages = store.messages().list_for_session(session.id).await.unwrap();
        assert!(
            messages[1]
                .content
                .to_string()
                .contains("loop=override-loop")
        );
    }

    #[tokio::test]
    async fn current_loop_name_prefers_session_then_project_then_runtime_default() {
        let (runtime, store, project) = make_runtime(
            "default",
            "runtime-default",
            Project::new(
                Some("test".into()),
                None,
                ProjectConfig {
                    agent: AgentConfig {
                        max_iterations: 20,
                        system_prompt: None,
                        loop_name: Some("project-loop".into()),
                        inference: InferenceConfig::default(),
                        ..AgentConfig::default()
                    },
                },
            ),
        )
        .await;

        let default_project = Project::with_defaults("default-project");
        let default_project = store
            .projects()
            .create(default_project.id, default_project)
            .await
            .unwrap();

        let mut session_override = Session::new(project.id);
        session_override.loop_name = Some("session-loop".into());
        let session_override = store
            .sessions()
            .create(session_override.id, session_override)
            .await
            .unwrap();
        let project_session = Session::new(project.id);
        let project_session = store
            .sessions()
            .create(project_session.id, project_session)
            .await
            .unwrap();
        let runtime_default_session = Session::new(default_project.id);
        let runtime_default_session = store
            .sessions()
            .create(runtime_default_session.id, runtime_default_session)
            .await
            .unwrap();

        assert_eq!(
            runtime
                .current_loop_name_for_session(session_override.id)
                .await
                .unwrap()
                .as_deref(),
            Some("session-loop")
        );
        assert_eq!(
            runtime
                .current_loop_name_for_session(project_session.id)
                .await
                .unwrap()
                .as_deref(),
            Some("project-loop")
        );
        assert_eq!(
            runtime
                .current_loop_name_for_session(runtime_default_session.id)
                .await
                .unwrap()
                .as_deref(),
            Some("runtime-default")
        );
    }

    #[tokio::test]
    async fn current_model_id_uses_default_provider_metadata() {
        let (runtime, store, project) =
            make_runtime("default", "simple", Project::with_defaults("test")).await;
        runtime
            .set_provider(
                "default",
                Arc::new(NamedProvider::new("default", vec!["model-a"])),
            )
            .unwrap();
        runtime
            .set_loop("simple", Arc::new(RecordingLoop { name: "simple" }))
            .unwrap();

        let session = Session::new(project.id);
        let session = store.sessions().create(session.id, session).await.unwrap();
        let current = runtime
            .current_model_id_for_session(session.id)
            .await
            .unwrap();
        assert_eq!(current.as_deref(), Some("model-a"));

        let current_model = runtime.current_model_for_session(session.id).await.unwrap();
        assert_eq!(current_model.provider, "default");
        assert_eq!(current_model.model.id, "model-a");
    }

    #[tokio::test]
    async fn set_session_model_updates_inference_via_runtime_trait() {
        let (runtime, store, project) =
            make_runtime("default", "simple", Project::with_defaults("test")).await;
        runtime
            .set_provider(
                "provider-a",
                Arc::new(NamedProvider::new("provider-a", vec!["model-a"])),
            )
            .unwrap();
        runtime
            .set_provider(
                "provider-b",
                Arc::new(NamedProvider::new("provider-b", vec!["model-b"])),
            )
            .unwrap();
        runtime
            .set_loop("simple", Arc::new(RecordingLoop { name: "simple" }))
            .unwrap();

        let session = Session::new(project.id);
        let session = store.sessions().create(session.id, session).await.unwrap();
        let updated = runtime
            .set_session_model(session.id, "model-b")
            .await
            .unwrap();

        assert_eq!(
            updated
                .inference
                .as_ref()
                .and_then(|cfg| cfg.provider.as_deref()),
            Some("provider-b")
        );
        assert_eq!(
            updated
                .inference
                .as_ref()
                .and_then(|cfg| cfg.model.as_deref()),
            Some("model-b")
        );
    }

    #[tokio::test]
    async fn set_session_thought_level_updates_session_override() {
        let (runtime, store, project) =
            make_runtime("default", "simple", Project::with_defaults("test")).await;
        runtime
            .set_provider(
                "default",
                Arc::new(NamedProvider::with_reasoning(
                    "default",
                    vec![("model-a", Some(&["low", "medium", "high"]))],
                )),
            )
            .unwrap();
        runtime
            .set_loop("simple", Arc::new(RecordingLoop { name: "simple" }))
            .unwrap();

        let session = Session::new(project.id);
        let session = store.sessions().create(session.id, session).await.unwrap();
        let updated = runtime
            .set_session_thought_level(session.id, "high")
            .await
            .unwrap();

        assert_eq!(
            updated
                .inference
                .as_ref()
                .and_then(|cfg| cfg.reasoning.as_deref()),
            Some("high")
        );
    }

    #[tokio::test]
    async fn set_session_loop_updates_session_override() {
        let (runtime, store, project) =
            make_runtime("default", "simple", Project::with_defaults("test")).await;
        runtime
            .set_provider(
                "default",
                Arc::new(NamedProvider::new("default", vec!["model-a"])),
            )
            .unwrap();
        runtime
            .set_loop("simple", Arc::new(RecordingLoop { name: "simple" }))
            .unwrap();
        runtime
            .set_loop("planner", Arc::new(RecordingLoop { name: "planner" }))
            .unwrap();

        let session = Session::new(project.id);
        let session = store.sessions().create(session.id, session).await.unwrap();
        let updated = runtime
            .set_session_loop(session.id, "planner")
            .await
            .unwrap();

        assert_eq!(updated.loop_name.as_deref(), Some("planner"));
    }

    #[tokio::test]
    async fn set_session_loop_rejects_unknown_loop() {
        let (runtime, store, project) =
            make_runtime("default", "simple", Project::with_defaults("test")).await;
        runtime
            .set_provider(
                "default",
                Arc::new(NamedProvider::new("default", vec!["model-a"])),
            )
            .unwrap();
        runtime
            .set_loop("simple", Arc::new(RecordingLoop { name: "simple" }))
            .unwrap();

        let session = Session::new(project.id);
        let session = store.sessions().create(session.id, session).await.unwrap();
        let error = runtime
            .set_session_loop(session.id, "missing")
            .await
            .expect_err("unknown loop should fail");

        assert!(error.to_string().contains("loop not registered: missing"));
    }

    #[tokio::test]
    async fn set_session_model_clears_incompatible_thought_level() {
        let (runtime, store, project) =
            make_runtime("default", "simple", Project::with_defaults("test")).await;
        runtime
            .set_provider(
                "provider-a",
                Arc::new(NamedProvider::with_reasoning(
                    "provider-a",
                    vec![("model-a", Some(&["low", "medium", "high"]))],
                )),
            )
            .unwrap();
        runtime
            .set_provider(
                "provider-b",
                Arc::new(NamedProvider::new("provider-b", vec!["model-b"])),
            )
            .unwrap();
        runtime
            .set_loop("simple", Arc::new(RecordingLoop { name: "simple" }))
            .unwrap();

        let mut session = Session::new(project.id);
        session.inference = Some(InferenceConfig {
            provider: Some("provider-a".into()),
            model: Some("model-a".into()),
            reasoning: Some("high".into()),
            max_tokens: None,
            temperature: None,
        });
        let session = store.sessions().create(session.id, session).await.unwrap();
        let updated = runtime
            .set_session_model(session.id, "model-b")
            .await
            .unwrap();

        assert_eq!(
            updated
                .inference
                .as_ref()
                .and_then(|cfg| cfg.reasoning.as_deref()),
            None
        );
    }

    #[tokio::test]
    async fn set_session_model_publishes_session_updated() {
        let (runtime, store, project) =
            make_runtime("default", "simple", Project::with_defaults("test")).await;
        runtime
            .set_provider(
                "provider-a",
                Arc::new(NamedProvider::new("provider-a", vec!["model-a"])),
            )
            .unwrap();
        runtime
            .set_provider(
                "provider-b",
                Arc::new(NamedProvider::new("provider-b", vec!["model-b"])),
            )
            .unwrap();
        runtime
            .set_loop("simple", Arc::new(RecordingLoop { name: "simple" }))
            .unwrap();

        let session = Session::new(project.id);
        let session = store.sessions().create(session.id, session).await.unwrap();
        let mut bus = runtime.subscribe();
        let updated = runtime
            .set_session_model(session.id, "model-b")
            .await
            .unwrap();

        let event = tokio::time::timeout(Duration::from_secs(1), bus.next())
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(
            event,
            RuntimeBusEvent::SessionUpdated { session } if session.id == updated.id
        ));
    }

    #[tokio::test]
    async fn turn_returns_error_for_ambiguous_model() {
        let (runtime, store, project) = make_runtime(
            "provider-a",
            "simple",
            Project::new(
                Some("test".into()),
                None,
                ProjectConfig {
                    agent: AgentConfig {
                        max_iterations: 20,
                        system_prompt: None,
                        loop_name: None,
                        inference: InferenceConfig {
                            provider: None,
                            model: Some("shared".into()),
                            reasoning: None,
                            max_tokens: None,
                            temperature: None,
                        },
                        ..AgentConfig::default()
                    },
                },
            ),
        )
        .await;
        runtime
            .set_provider(
                "provider-a",
                Arc::new(NamedProvider::new("provider-a", vec!["shared"])),
            )
            .unwrap();
        runtime
            .set_provider(
                "provider-b",
                Arc::new(NamedProvider::new("provider-b", vec!["shared"])),
            )
            .unwrap();
        runtime
            .set_loop("simple", Arc::new(RecordingLoop { name: "simple" }))
            .unwrap();

        let session = Session::new(project.id);
        let session = store.sessions().create(session.id, session).await.unwrap();
        let events = run_turn(&runtime, session.id, "hello").await;
        assert!(events.iter().any(
            |event| matches!(event, Event::Error { code, .. } if *code == BrainErrorCode::Internal)
        ));
    }

    #[tokio::test]
    async fn cancel_turn_cancels_active_turn_and_blocks_concurrent_turns() {
        let (runtime, store, project) =
            make_runtime("default", "wait", Project::with_defaults("test")).await;
        runtime
            .set_provider(
                "default",
                Arc::new(NamedProvider::new("default", vec!["model-a"])),
            )
            .unwrap();
        runtime.set_loop("wait", Arc::new(WaitingLoop)).unwrap();

        let session = Session::new(project.id);
        let session = store.sessions().create(session.id, session).await.unwrap();
        let mut first = runtime.turn(session.id, "first", CancellationToken::new());
        let mut second = runtime.turn(session.id, "second", CancellationToken::new());

        match second.next().await {
            Some(Event::Error { code, .. }) => assert_eq!(code, BrainErrorCode::TurnActive),
            other => panic!("expected TurnActive error, got {other:?}"),
        }

        runtime.cancel_turn(session.id).await.unwrap();

        match first.next().await {
            Some(Event::Error { code, .. }) => assert_eq!(code, BrainErrorCode::Cancelled),
            other => panic!("expected Cancelled error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn replacing_provider_affects_later_turns() {
        let (runtime, store, project) =
            make_runtime("default", "simple", Project::with_defaults("test")).await;
        runtime
            .set_provider(
                "default",
                Arc::new(NamedProvider::new("first", vec!["model-a"])),
            )
            .unwrap();
        runtime
            .set_loop("simple", Arc::new(RecordingLoop { name: "simple" }))
            .unwrap();

        let session = Session::new(project.id);
        let session = store.sessions().create(session.id, session).await.unwrap();
        let _ = run_turn(&runtime, session.id, "hello").await;

        let previous = runtime
            .set_provider(
                "default",
                Arc::new(NamedProvider::new("second", vec!["model-a"])),
            )
            .unwrap();
        assert!(previous.is_some());

        let next_session = Session::new(project.id);
        let next_session = store
            .sessions()
            .create(next_session.id, next_session)
            .await
            .unwrap();
        let _ = run_turn(&runtime, next_session.id, "hello").await;

        let messages = store
            .messages()
            .list_for_session(next_session.id)
            .await
            .unwrap();
        assert!(messages[1].content.to_string().contains("provider=second"));
    }

    #[tokio::test]
    async fn simple_loop_can_be_registered() {
        let (runtime, _store, _project) =
            make_runtime("default", "simple", Project::with_defaults("test")).await;
        runtime
            .set_provider(
                "default",
                Arc::new(NamedProvider::new("default", vec!["model-a"])),
            )
            .unwrap();
        runtime.set_loop("simple", Arc::new(SimpleLoop)).unwrap();

        let listed = runtime.loops().list().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].0, "simple");
    }
}
