use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use futures::StreamExt;
use tokio_util::sync::CancellationToken;
use ulid::Ulid;

use crate::atif_events::{
    CompleteTurnInput, TurnSummary, complete_turn, completion_events, started_event,
};
use brain_types::*;

pub struct Brain {
    pub provider: Arc<dyn Provider>,
    pub store: Arc<dyn Store>,
    pub agent_loop: Arc<dyn AgentLoop>,
    pub tools: Vec<Arc<dyn Tool>>,
}

impl Brain {
    pub fn new(
        provider: Arc<dyn Provider>,
        store: Arc<dyn Store>,
        agent_loop: Arc<dyn AgentLoop>,
        tools: Vec<Arc<dyn Tool>>,
    ) -> Self {
        Self {
            provider,
            store,
            agent_loop,
            tools,
        }
    }

    /// Execute a single conversational turn: load history, run loop, persist results.
    pub fn turn(&self, session_id: Ulid, input: &str, cancel: CancellationToken) -> EventStream {
        let store = self.store.clone();
        let provider = self.provider.clone();
        let agent_loop = self.agent_loop.clone();
        let tools = self.tools.clone();
        let user_msg = Message::user(input);

        let (tx, rx) = tokio::sync::mpsc::channel::<Event>(256);

        tokio::spawn(async move {
            let result = async {
                let config = load_agent_config_for_session(store.clone(), session_id).await?;
                run_turn_with_config(
                    store,
                    provider,
                    agent_loop,
                    tools,
                    config,
                    cancel,
                    session_id,
                    user_msg,
                    tx.clone(),
                )
                .await
            }
            .await;

            if let Err(e) = result {
                send_error_event(tx, e).await;
            }
        });

        Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx))
    }

    /// Drive an interactive session: recv from transport, dispatch turns, send events back.
    ///
    /// When `resume_session` is `Some(id)`, the given session is resumed instead of
    /// creating a new one. The session must belong to the same project.
    pub async fn run(
        &self,
        project: &Project,
        transport: &dyn Transport,
        resume_session: Option<Ulid>,
    ) -> Result<(), BrainError> {
        let mut session_id = if let Some(id) = resume_session {
            let session = self.store.sessions().get(id).await?;
            if session.project_id != project.id {
                return Err(BrainError::Storage(format!(
                    "session {id} does not belong to project {}",
                    project.id
                )));
            }
            tracing::info!(session_id = %id, "session resumed");
            transport
                .send(Event::SessionResume { session_id: id })
                .await?;
            id
        } else {
            let session = Session::new(project.id);
            let session = self.store.sessions().create(session.id, session).await?;
            tracing::info!(session_id = %session.id, "session started");
            transport
                .send(Event::SessionStart {
                    session_id: session.id,
                })
                .await?;
            session.id
        };
        loop {
            let input = match transport.recv().await? {
                Some(InputEvent::Message(text)) => text,
                Some(InputEvent::ToolApproval { .. }) => {
                    continue;
                }
                Some(InputEvent::Cancel) => {
                    continue;
                }
                Some(InputEvent::SwitchSession(target)) => {
                    let target_session = self.store.sessions().get(target).await?;
                    if target_session.project_id != project.id {
                        return Err(BrainError::Storage(format!(
                            "session {target} does not belong to project {}",
                            project.id
                        )));
                    }
                    session_id = target;
                    transport
                        .send(Event::SessionResume { session_id: target })
                        .await?;
                    continue;
                }
                None => break,
                Some(_) => continue,
            };

            let cancel = CancellationToken::new();
            let mut events = self.turn(session_id, &input, cancel);

            while let Some(event) = events.next().await {
                transport.send(event).await?;
            }
        }

        tracing::info!(session_id = %session_id, "session ended");
        Ok(())
    }

    pub async fn create_session(&self, project_id: ProjectId) -> Result<Session, BrainError> {
        let session = Session::new(project_id);
        self.store.sessions().create(session.id, session).await
    }

    pub async fn list_sessions(&self, project_id: ProjectId) -> Result<Vec<Session>, BrainError> {
        self.store.sessions().list_for_project(project_id).await
    }

    pub async fn update_session_inference(
        &self,
        session_id: Ulid,
        inference: Option<InferenceConfig>,
    ) -> Result<Session, BrainError> {
        let mut session = self.store.sessions().get(session_id).await?;
        session.inference = inference.filter(|config| !config.is_empty());
        self.store.sessions().update(session.id, session).await
    }

    pub async fn effective_inference_for_session(
        &self,
        session_id: Ulid,
    ) -> Result<InferenceConfig, BrainError> {
        let session = self.store.sessions().get(session_id).await?;
        let project = self.store.projects().get(session.project_id).await?;
        Ok(resolve_effective_agent_config(&project.config.agent, &session).inference)
    }

    pub async fn effective_agent_config_for_session(
        &self,
        session_id: Ulid,
    ) -> Result<AgentConfig, BrainError> {
        load_agent_config_for_session(self.store.clone(), session_id).await
    }

    pub async fn resolve_or_create_project(
        &self,
        root: std::path::PathBuf,
    ) -> Result<Project, BrainError> {
        resolve_or_create_project_with_store(self.store.clone(), root).await
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_turn_with_config(
    store: Arc<dyn Store>,
    provider: Arc<dyn Provider>,
    agent_loop: Arc<dyn AgentLoop>,
    tools: Vec<Arc<dyn Tool>>,
    config: AgentConfig,
    cancel: CancellationToken,
    session_id: Ulid,
    user_msg: Message,
    tx: tokio::sync::mpsc::Sender<Event>,
) -> Result<(), BrainError> {
    let existing_trajectory = store.trajectories().get_for_session(session_id).await?;
    let history_before = store.messages().list_for_session(session_id).await?;
    let mut history = history_before.clone();
    history.push(user_msg.clone());

    let mut inner_stream = agent_loop.run(
        provider.clone(),
        tools.clone(),
        history,
        config.clone(),
        cancel,
        Some(session_id),
    );

    let mut new_messages: Vec<Message> = vec![user_msg];
    let mut turn_summary = None::<TurnSummary>;
    let mut pending_turn_done = None::<Event>;
    let started_at = Utc::now();

    if config.atif.emit_events && existing_trajectory.is_none() {
        let event = started_event(session_id, &config, provider.as_ref(), &tools, started_at)?;
        let _ = tx.send(event).await;
    }

    while let Some(event) = inner_stream.next().await {
        match &event {
            Event::MessageDone { message } => {
                new_messages.push(message.clone());
            }
            Event::ToolCallDone { id, result, .. } => {
                new_messages.push(Message::tool_result(id, result));
                let _ = tx.send(event).await;
                continue;
            }
            Event::TurnDone {
                iterations,
                prompt_tokens,
                completion_tokens,
                cache_read_tokens,
                cache_write_tokens,
                reasoning_tokens,
                total_tokens,
            } => {
                turn_summary = Some(TurnSummary {
                    iterations: *iterations,
                    prompt_tokens: *prompt_tokens,
                    completion_tokens: *completion_tokens,
                    cache_read_tokens: *cache_read_tokens,
                    cache_write_tokens: *cache_write_tokens,
                    reasoning_tokens: *reasoning_tokens,
                    total_tokens: *total_tokens,
                });
                pending_turn_done = Some(event);
                continue;
            }
            _ => {}
        }
        let _ = tx.send(event).await;
    }

    let persisted_messages = new_messages.clone();
    let keyed_messages = persisted_messages
        .into_iter()
        .map(|message| ((session_id, message.id), message))
        .collect();
    store.messages().create_many(keyed_messages).await?;

    if config.atif.emit_events
        && let Some(turn_summary) = turn_summary
    {
        let finished_at = Utc::now();
        let completed = complete_turn(CompleteTurnInput {
            session_id,
            existing_trajectory,
            config: &config,
            provider: provider.as_ref(),
            tools: &tools,
            new_messages: &new_messages,
            started_at,
            finished_at,
            turn_summary,
        })?;
        if store
            .trajectories()
            .get_for_session(session_id)
            .await?
            .is_some()
        {
            store
                .trajectories()
                .update(session_id, completed.trajectory.clone())
                .await?;
        } else {
            store
                .trajectories()
                .create(session_id, completed.trajectory.clone())
                .await?;
        }
        for atif_event in completion_events(&completed) {
            let _ = tx.send(atif_event).await;
        }
    }

    // `TurnDone` stays terminal even when ATIF is enabled. Consumers that stop
    // on `TurnDone` should still observe a complete-turn view, including any
    // ATIF completion records emitted for this turn.
    if let Some(turn_done) = pending_turn_done {
        let _ = tx.send(turn_done).await;
    }

    Ok(())
}

pub(crate) async fn send_error_event(tx: tokio::sync::mpsc::Sender<Event>, error: BrainError) {
    let _ = tx
        .send(Event::Error {
            code: error.code(),
            message: error.to_string(),
            recoverable: error.recoverable(),
        })
        .await;
}

pub(crate) fn resolve_effective_inference(
    defaults: &InferenceConfig,
    session_inference: Option<&InferenceConfig>,
) -> InferenceConfig {
    match session_inference {
        Some(overrides) => defaults.merged_with(overrides),
        None => defaults.clone(),
    }
}

pub(crate) fn resolve_effective_agent_config(
    defaults: &AgentConfig,
    session: &Session,
) -> AgentConfig {
    let mut config = defaults.clone();
    config.inference = resolve_effective_inference(&defaults.inference, session.inference.as_ref());
    config.loop_name = session
        .loop_name
        .clone()
        .or_else(|| defaults.loop_name.clone());
    config
}

pub(crate) async fn load_agent_config_for_session(
    store: Arc<dyn Store>,
    session_id: Ulid,
) -> Result<AgentConfig, BrainError> {
    let session = store.sessions().get(session_id).await?;
    let project = store.projects().get(session.project_id).await?;
    Ok(resolve_effective_agent_config(
        &project.config.agent,
        &session,
    ))
}

pub(crate) async fn resolve_or_create_project_with_store(
    store: Arc<dyn Store>,
    root: PathBuf,
) -> Result<Project, BrainError> {
    resolve_or_create_project_with_store_status(store, root)
        .await
        .map(|(project, _)| project)
}

pub(crate) async fn resolve_or_create_project_with_store_status(
    store: Arc<dyn Store>,
    root: PathBuf,
) -> Result<(Project, bool), BrainError> {
    let normalized_root = normalize_project_root(&root);
    if let Some(project) = store.projects().find_by_root(&normalized_root).await? {
        return Ok((project, false));
    }

    let name = normalized_root
        .file_name()
        .and_then(|segment| segment.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| "brain".to_owned());
    let config = resolve_project_config(&normalized_root);
    let project = Project::new(Some(name), Some(normalized_root), config);
    let project_id = project.id;
    store
        .projects()
        .create(project_id, project)
        .await
        .map(|project| (project, true))
}

fn resolve_project_config(root: &Path) -> ProjectConfig {
    let global_config = brain_stores::brain_home().join("config.toml");
    resolve_project_config_with_global(root, Some(global_config))
}

fn resolve_project_config_with_global(
    root: &Path,
    global_config: Option<PathBuf>,
) -> ProjectConfig {
    let mut config = brain_config::resolve_fs_config_with_global(root, global_config.as_deref())
        .unwrap_or_else(|error| {
            tracing::warn!("config resolution failed, using defaults: {error}");
            ProjectConfig::default()
        });

    if config.agent.system_prompt.is_none()
        && let Ok(Some(agents_md)) = brain_config::load_root_agents_md(root)
    {
        config.agent.system_prompt = Some(agents_md);
    }

    config
}

#[cfg(test)]
mod tests {
    use super::*;
    use brain_loops::SimpleLoop;
    use brain_providers::MockProvider;
    use brain_stores::InMemoryStore;
    use futures::future::BoxFuture;
    use std::fs;
    use tempfile::tempdir;

    async fn make_brain() -> (Brain, Project, Arc<InMemoryStore>) {
        make_brain_with_config(ProjectConfig::default()).await
    }

    async fn make_brain_with_config(config: ProjectConfig) -> (Brain, Project, Arc<InMemoryStore>) {
        let project = Project::new(Some("test".into()), None, config);
        let provider: Arc<dyn Provider> = Arc::new(MockProvider::new());
        let store = Arc::new(InMemoryStore::new());
        let agent_loop: Arc<dyn AgentLoop> = Arc::new(SimpleLoop);

        let brain = Brain::new(provider, store.clone(), agent_loop, vec![]);
        store
            .projects()
            .create(project.id, project.clone())
            .await
            .unwrap();
        (brain, project, store)
    }

    #[tokio::test]
    async fn project_agent_config_defaults() {
        let project = Project::with_defaults("test");
        let config = project.config.agent_config();
        assert_eq!(config.max_iterations, 20);
    }

    #[tokio::test]
    async fn create_and_list_sessions() {
        let (brain, project, _store) = make_brain().await;

        let s1 = brain.create_session(project.id).await.unwrap();
        let s2 = brain.create_session(project.id).await.unwrap();
        assert_ne!(s1.id, s2.id);

        let list = brain.list_sessions(project.id).await.unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn resolve_or_create_project_reuses_existing_root() {
        let (brain, _project, store) = make_brain().await;
        let project = Project::new(
            Some("repo".into()),
            Some("/tmp/work/repo".into()),
            ProjectConfig::default(),
        );
        let id = project.id;
        store.projects().create(project.id, project).await.unwrap();

        let resolved = brain
            .resolve_or_create_project("/tmp/work/./repo".into())
            .await
            .unwrap();
        assert_eq!(resolved.id, id);
    }

    #[tokio::test]
    async fn resolve_or_create_project_creates_missing_root() {
        let (brain, _project, _store) = make_brain().await;
        let project = brain
            .resolve_or_create_project("/tmp/new-project".into())
            .await
            .unwrap();
        assert_eq!(project.root, Some("/tmp/new-project".into()));
        assert_eq!(project.name.as_deref(), Some("new-project"));
    }

    #[test]
    fn resolve_project_config_loads_project_file_and_root_agents() {
        let dir = tempdir().unwrap();
        let agents_dir = dir.path().join(".agents");
        fs::create_dir_all(&agents_dir).unwrap();
        fs::write(
            agents_dir.join("config.toml"),
            r#"
[agent]
max_iterations = 7
[agent.inference]
provider = "mock"
"#,
        )
        .unwrap();
        fs::write(agents_dir.join("AGENTS.md"), "root prompt").unwrap();

        let config = resolve_project_config_with_global(dir.path(), None);
        assert_eq!(config.agent.max_iterations, 7);
        assert_eq!(config.agent.inference.provider.as_deref(), Some("mock"));
        assert_eq!(config.agent.system_prompt.as_deref(), Some("root prompt"));
    }

    #[test]
    fn resolve_project_config_preserves_explicit_prompt_over_agents_md() {
        let dir = tempdir().unwrap();
        let agents_dir = dir.path().join(".agents");
        fs::create_dir_all(&agents_dir).unwrap();
        fs::write(
            agents_dir.join("config.toml"),
            r#"
[agent]
system_prompt = "config prompt"
"#,
        )
        .unwrap();
        fs::write(agents_dir.join("AGENTS.md"), "root prompt").unwrap();

        let config = resolve_project_config_with_global(dir.path(), None);
        assert_eq!(config.agent.system_prompt.as_deref(), Some("config prompt"));
    }

    #[tokio::test]
    async fn effective_inference_merges_project_defaults_with_session_overrides() {
        let provider: Arc<dyn Provider> = Arc::new(MockProvider::new());
        let store = Arc::new(InMemoryStore::new());
        let agent_loop: Arc<dyn AgentLoop> = Arc::new(SimpleLoop);
        let brain = Brain::new(provider, store.clone(), agent_loop, vec![]);

        let project = Project::new(
            Some("test".into()),
            None,
            ProjectConfig {
                agent: AgentConfig {
                    max_iterations: 20,
                    system_prompt: None,
                    loop_name: None,
                    inference: InferenceConfig {
                        provider: Some("openai".into()),
                        model: Some("gpt-4o-mini".into()),
                        reasoning: Some("medium".into()),
                        max_tokens: Some(4096),
                        temperature: Some(0.7),
                    },
                    ..AgentConfig::default()
                },
            },
        );
        let project = store.projects().create(project.id, project).await.unwrap();

        let session = brain.create_session(project.id).await.unwrap();
        brain
            .update_session_inference(
                session.id,
                Some(InferenceConfig {
                    provider: None,
                    model: Some("gpt-5".into()),
                    reasoning: None,
                    max_tokens: None,
                    temperature: Some(0.2),
                }),
            )
            .await
            .unwrap();

        let effective = brain
            .effective_inference_for_session(session.id)
            .await
            .unwrap();
        assert_eq!(effective.provider.as_deref(), Some("openai"));
        assert_eq!(effective.model.as_deref(), Some("gpt-5"));
        assert_eq!(effective.reasoning.as_deref(), Some("medium"));
        assert_eq!(effective.max_tokens, Some(4096));
        assert_eq!(effective.temperature, Some(0.2));
    }

    #[tokio::test]
    async fn turn_produces_events_and_persists() {
        let (brain, project, store) = make_brain().await;

        let session = brain.create_session(project.id).await.unwrap();
        let cancel = CancellationToken::new();
        let mut events = brain.turn(session.id, "hello world", cancel);

        let mut saw_token = false;
        let mut saw_message_done = false;
        let mut saw_turn_done = false;

        while let Some(event) = events.next().await {
            match event {
                Event::Token { .. } => saw_token = true,
                Event::MessageDone { .. } => saw_message_done = true,
                Event::TurnDone { iterations, .. } => {
                    assert_eq!(iterations, 1);
                    saw_turn_done = true;
                }
                _ => {}
            }
        }

        assert!(saw_token);
        assert!(saw_message_done);
        assert!(saw_turn_done);

        let messages = store.messages().list_for_session(session.id).await.unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, Role::User);
        assert_eq!(messages[0].content, MessageContent::text("hello world"));
        assert_eq!(messages[1].role, Role::Assistant);
    }

    #[tokio::test]
    async fn turn_appends_to_existing_history() {
        let (brain, project, store) = make_brain().await;
        let session = brain.create_session(project.id).await.unwrap();

        // First turn
        let mut events = brain.turn(session.id, "first", CancellationToken::new());
        while events.next().await.is_some() {}

        // Second turn
        let mut events = brain.turn(session.id, "second", CancellationToken::new());
        while events.next().await.is_some() {}

        let messages = store.messages().list_for_session(session.id).await.unwrap();
        assert_eq!(messages.len(), 4);
    }

    #[tokio::test]
    async fn turn_does_not_emit_atif_events_by_default() {
        let (brain, project, _store) = make_brain().await;
        let session = brain.create_session(project.id).await.unwrap();
        let mut events = brain.turn(session.id, "hello world", CancellationToken::new());

        while let Some(event) = events.next().await {
            assert!(
                !matches!(event, Event::Atif { .. }),
                "ATIF events should be disabled by default"
            );
        }
    }

    #[tokio::test]
    async fn turn_emits_atif_completion_records_when_enabled() {
        let (brain, project, _store) = make_brain_with_config(ProjectConfig {
            agent: AgentConfig {
                system_prompt: Some("system prompt".into()),
                atif: AtifConfig {
                    emit_events: true,
                    ..AtifConfig::default()
                },
                ..AgentConfig::default()
            },
        })
        .await;
        let session = brain.create_session(project.id).await.unwrap();
        let mut events = brain.turn(session.id, "hello world", CancellationToken::new());

        let mut saw_started = false;
        let mut saw_system_step = false;
        let mut saw_user_step = false;
        let mut saw_agent_step = false;
        let mut saw_final_metrics = false;
        let mut saw_completed = false;
        let mut saw_turn_done = false;
        let mut turn_done_after_trajectory = false;

        while let Some(event) = events.next().await {
            match event {
                Event::Atif {
                    event:
                        AtifEvent::TrajectoryStarted {
                            schema_version,
                            session_id,
                            agent,
                        },
                } => {
                    saw_started = true;
                    assert_eq!(schema_version, AtifConfig::default().schema_version);
                    assert_eq!(session_id, session.id.to_string());
                    assert_eq!(agent.name, "brain");
                }
                Event::Atif {
                    event: AtifEvent::StepCompleted { step },
                } => match step.source {
                    atif::StepSource::System => {
                        saw_system_step = true;
                        assert_eq!(step.message, atif::MessageContent::from("system prompt"));
                    }
                    atif::StepSource::User => {
                        saw_user_step = true;
                        assert_eq!(step.message, atif::MessageContent::from("hello world"));
                    }
                    atif::StepSource::Agent => {
                        saw_agent_step = true;
                    }
                },
                Event::Atif {
                    event: AtifEvent::FinalMetrics { final_metrics },
                } => {
                    saw_final_metrics = true;
                    assert!(final_metrics.total_steps.is_some());
                }
                Event::Atif {
                    event: AtifEvent::TrajectoryCompleted { trajectory },
                } => {
                    saw_completed = true;
                    assert_eq!(trajectory.session_id, session.id.to_string());
                    assert!(trajectory.final_metrics.is_some());
                    assert_eq!(trajectory.steps.len(), 3);
                }
                Event::TurnDone { .. } => {
                    saw_turn_done = true;
                    turn_done_after_trajectory = saw_completed;
                }
                _ => {}
            }
        }

        assert!(saw_started);
        assert!(saw_system_step);
        assert!(saw_user_step);
        assert!(saw_agent_step);
        assert!(saw_final_metrics);
        assert!(saw_completed);
        assert!(saw_turn_done);
        assert!(turn_done_after_trajectory);
    }

    #[tokio::test]
    async fn turn_preserves_historical_model_names_across_turns() {
        let (brain, project, store) = make_brain_with_config(ProjectConfig {
            agent: AgentConfig {
                atif: AtifConfig {
                    emit_events: true,
                    ..AtifConfig::default()
                },
                inference: InferenceConfig {
                    provider: Some("mock".into()),
                    model: Some("mock-echo".into()),
                    reasoning: None,
                    max_tokens: None,
                    temperature: None,
                },
                ..AgentConfig::default()
            },
        })
        .await;
        let session = brain.create_session(project.id).await.unwrap();

        let mut first_turn = brain.turn(session.id, "first", CancellationToken::new());
        while first_turn.next().await.is_some() {}

        brain
            .update_session_inference(
                session.id,
                Some(InferenceConfig {
                    provider: Some("mock".into()),
                    model: Some("mock-think".into()),
                    reasoning: None,
                    max_tokens: None,
                    temperature: None,
                }),
            )
            .await
            .unwrap();

        let mut second_turn = brain.turn(session.id, "second", CancellationToken::new());
        while second_turn.next().await.is_some() {}

        let trajectory = store
            .trajectories()
            .get_for_session(session.id)
            .await
            .unwrap()
            .unwrap();
        let agent_steps = trajectory
            .steps
            .iter()
            .filter(|step| matches!(step.source, atif::StepSource::Agent))
            .collect::<Vec<_>>();

        assert_eq!(agent_steps.len(), 2);
        assert_eq!(agent_steps[0].model_name.as_deref(), Some("mock-echo"));
        assert_eq!(agent_steps[1].model_name.as_deref(), Some("mock-think"));
    }

    #[tokio::test]
    async fn turn_does_not_rewrite_original_system_prompt_after_config_change() {
        let (brain, mut project, store) = make_brain_with_config(ProjectConfig {
            agent: AgentConfig {
                system_prompt: Some("system prompt a".into()),
                atif: AtifConfig {
                    emit_events: true,
                    ..AtifConfig::default()
                },
                ..AgentConfig::default()
            },
        })
        .await;
        let session = brain.create_session(project.id).await.unwrap();

        let mut first_turn = brain.turn(session.id, "first", CancellationToken::new());
        while first_turn.next().await.is_some() {}

        project.config.agent.system_prompt = Some("system prompt b".into());
        store.projects().update(project.id, project).await.unwrap();

        let mut second_turn = brain.turn(session.id, "second", CancellationToken::new());
        while second_turn.next().await.is_some() {}

        let trajectory = store
            .trajectories()
            .get_for_session(session.id)
            .await
            .unwrap()
            .unwrap();
        let system_steps = trajectory
            .steps
            .iter()
            .filter(|step| matches!(step.source, atif::StepSource::System))
            .collect::<Vec<_>>();

        assert_eq!(system_steps.len(), 1);
        assert_eq!(
            system_steps[0].message,
            atif::MessageContent::from("system prompt a")
        );
    }

    #[tokio::test]
    async fn run_drives_transport() {
        use std::sync::Mutex;

        struct MockTransport {
            inputs: Mutex<Vec<String>>,
            events: Mutex<Vec<Event>>,
        }

        impl Transport for MockTransport {
            fn name(&self) -> &str {
                "mock"
            }

            fn recv(&self) -> BoxFuture<'_, Result<Option<InputEvent>, BrainError>> {
                Box::pin(async {
                    let mut inputs = self.inputs.lock().unwrap();
                    if inputs.is_empty() {
                        Ok(None)
                    } else {
                        Ok(Some(InputEvent::Message(inputs.remove(0))))
                    }
                })
            }

            fn send(&self, event: Event) -> BoxFuture<'_, Result<(), BrainError>> {
                Box::pin(async {
                    self.events.lock().unwrap().push(event);
                    Ok(())
                })
            }
        }

        let (brain, project, _store) = make_brain().await;
        let transport = MockTransport {
            inputs: Mutex::new(vec!["hello".into()]),
            events: Mutex::new(vec![]),
        };

        brain.run(&project, &transport, None).await.unwrap();

        let events = transport.events.lock().unwrap();
        assert!(!events.is_empty());
        assert!(events.iter().any(|e| matches!(e, Event::TurnDone { .. })));
    }

    #[tokio::test]
    async fn run_emits_session_start() {
        use std::sync::Mutex;

        struct MockTransport {
            inputs: Mutex<Vec<String>>,
            events: Mutex<Vec<Event>>,
        }

        impl Transport for MockTransport {
            fn name(&self) -> &str {
                "mock"
            }

            fn recv(&self) -> BoxFuture<'_, Result<Option<InputEvent>, BrainError>> {
                Box::pin(async {
                    let mut inputs = self.inputs.lock().unwrap();
                    if inputs.is_empty() {
                        Ok(None)
                    } else {
                        Ok(Some(InputEvent::Message(inputs.remove(0))))
                    }
                })
            }

            fn send(&self, event: Event) -> BoxFuture<'_, Result<(), BrainError>> {
                Box::pin(async {
                    self.events.lock().unwrap().push(event);
                    Ok(())
                })
            }
        }

        let (brain, project, _store) = make_brain().await;
        let transport = MockTransport {
            inputs: Mutex::new(vec!["hello".into()]),
            events: Mutex::new(vec![]),
        };

        brain.run(&project, &transport, None).await.unwrap();

        let events = transport.events.lock().unwrap();
        assert!(
            matches!(events.first(), Some(Event::SessionStart { .. })),
            "first event should be SessionStart, got {:?}",
            events.first()
        );
    }

    #[tokio::test]
    async fn run_switch_session_event_changes_target() {
        use std::sync::Mutex;

        struct MockTransport {
            inputs: Mutex<Vec<InputEvent>>,
            events: Mutex<Vec<Event>>,
        }

        impl Transport for MockTransport {
            fn name(&self) -> &str {
                "mock"
            }

            fn recv(&self) -> BoxFuture<'_, Result<Option<InputEvent>, BrainError>> {
                Box::pin(async {
                    let mut inputs = self.inputs.lock().unwrap();
                    if inputs.is_empty() {
                        Ok(None)
                    } else {
                        Ok(Some(inputs.remove(0)))
                    }
                })
            }

            fn send(&self, event: Event) -> BoxFuture<'_, Result<(), BrainError>> {
                Box::pin(async {
                    self.events.lock().unwrap().push(event);
                    Ok(())
                })
            }
        }

        let (brain, project, store) = make_brain().await;
        let alt_session = brain.create_session(project.id).await.unwrap();
        let transport = MockTransport {
            inputs: Mutex::new(vec![
                InputEvent::SwitchSession(alt_session.id),
                InputEvent::Message("hello world".into()),
            ]),
            events: Mutex::new(vec![]),
        };

        brain.run(&project, &transport, None).await.unwrap();

        let events = transport.events.lock().unwrap();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::SessionResume { session_id } if *session_id == alt_session.id)),
            "should emit SessionResume for switched session"
        );

        let alt_messages = store
            .messages()
            .list_for_session(alt_session.id)
            .await
            .unwrap();
        assert!(!alt_messages.is_empty());
        assert_eq!(alt_messages[0].role, Role::User);
        assert_eq!(alt_messages[0].content, MessageContent::text("hello world"));
        assert_eq!(alt_messages[1].role, Role::Assistant);
    }

    #[tokio::test]
    async fn run_resume_session_emits_session_resume() {
        use std::sync::Mutex;

        struct MockTransport {
            inputs: Mutex<Vec<String>>,
            events: Mutex<Vec<Event>>,
        }

        impl Transport for MockTransport {
            fn name(&self) -> &str {
                "mock"
            }

            fn recv(&self) -> BoxFuture<'_, Result<Option<InputEvent>, BrainError>> {
                Box::pin(async {
                    let mut inputs = self.inputs.lock().unwrap();
                    if inputs.is_empty() {
                        Ok(None)
                    } else {
                        Ok(Some(InputEvent::Message(inputs.remove(0))))
                    }
                })
            }

            fn send(&self, event: Event) -> BoxFuture<'_, Result<(), BrainError>> {
                Box::pin(async {
                    self.events.lock().unwrap().push(event);
                    Ok(())
                })
            }
        }

        let (brain, project, store) = make_brain().await;
        let session = brain.create_session(project.id).await.unwrap();

        let transport = MockTransport {
            inputs: Mutex::new(vec!["resumed hello".into()]),
            events: Mutex::new(vec![]),
        };

        brain
            .run(&project, &transport, Some(session.id))
            .await
            .unwrap();

        let events = transport.events.lock().unwrap();
        assert!(
            matches!(events.first(), Some(Event::SessionResume { session_id }) if *session_id == session.id),
            "first event should be SessionResume, got {:?}",
            events.first()
        );
        assert!(
            events.iter().any(|e| matches!(e, Event::TurnDone { .. })),
            "should complete the turn"
        );

        let messages = store.messages().list_for_session(session.id).await.unwrap();
        assert!(!messages.is_empty());
        assert_eq!(messages[0].role, Role::User);
        assert_eq!(messages[0].content, MessageContent::text("resumed hello"));
    }

    #[tokio::test]
    async fn run_resume_wrong_project_errors() {
        use std::sync::Mutex;

        struct MockTransport {
            events: Mutex<Vec<Event>>,
        }

        impl Transport for MockTransport {
            fn name(&self) -> &str {
                "mock"
            }
            fn recv(&self) -> BoxFuture<'_, Result<Option<InputEvent>, BrainError>> {
                Box::pin(async { Ok(None) })
            }
            fn send(&self, event: Event) -> BoxFuture<'_, Result<(), BrainError>> {
                Box::pin(async {
                    self.events.lock().unwrap().push(event);
                    Ok(())
                })
            }
        }

        let (brain, project, store) = make_brain().await;
        let other_project = Project::with_defaults("other");
        let other_project = store
            .projects()
            .create(other_project.id, other_project)
            .await
            .unwrap();
        let other_session = brain.create_session(other_project.id).await.unwrap();

        let transport = MockTransport {
            events: Mutex::new(vec![]),
        };

        let result = brain
            .run(&project, &transport, Some(other_session.id))
            .await;
        assert!(
            result.is_err(),
            "should error when session belongs to different project"
        );
    }
}
