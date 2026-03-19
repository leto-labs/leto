use std::sync::Arc;

use futures::StreamExt;
use tokio_util::sync::CancellationToken;
use ulid::Ulid;

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
            if let Err(e) = turn_inner(
                store,
                provider,
                agent_loop,
                tools,
                cancel,
                session_id,
                user_msg,
                tx.clone(),
            )
            .await
            {
                let _ = tx
                    .send(Event::Error {
                        code: e.code(),
                        message: e.to_string(),
                        recoverable: e.recoverable(),
                    })
                    .await;
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
            let session = self.store.session_get(id).await?;
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
            let session = self.store.session_create(project.id).await?;
            tracing::info!(session_id = %session.id, "session started");
            transport
                .send(Event::SessionStart {
                    session_id: session.id,
                })
                .await?;
            session.id
        };
        let mut active_cancel = None::<CancellationToken>;

        loop {
            let input = match transport.recv().await? {
                Some(InputEvent::Message(text)) => text,
                Some(InputEvent::ToolApproval { .. }) => {
                    continue;
                }
                Some(InputEvent::Cancel) => {
                    if let Some(token) = active_cancel.take() {
                        token.cancel();
                    }
                    continue;
                }
                Some(InputEvent::SwitchSession(target)) => {
                    let target_session = self.store.session_get(target).await?;
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
            active_cancel = Some(cancel.clone());
            let mut events = self.turn(session_id, &input, cancel);

            while let Some(event) = events.next().await {
                transport.send(event).await?;
            }
            active_cancel = None;
        }

        tracing::info!(session_id = %session_id, "session ended");
        Ok(())
    }

    pub async fn create_session(&self, project_id: ProjectId) -> Result<Session, BrainError> {
        self.store.session_create(project_id).await
    }

    pub async fn list_sessions(&self, project_id: ProjectId) -> Result<Vec<Session>, BrainError> {
        self.store.session_list(project_id).await
    }

    pub async fn update_session_inference(
        &self,
        session_id: Ulid,
        inference: Option<InferenceConfig>,
    ) -> Result<Session, BrainError> {
        let normalized = inference.filter(|config| !config.is_empty());
        let update = match normalized {
            Some(config) => SessionUpdate::inference(config),
            None => SessionUpdate::clear_inference(),
        };

        self.store.session_update(session_id, update).await?;
        self.store.session_get(session_id).await
    }

    pub async fn effective_inference_for_session(
        &self,
        session_id: Ulid,
    ) -> Result<InferenceConfig, BrainError> {
        let session = self.store.session_get(session_id).await?;
        let project = self.store.project_get(session.project_id).await?;
        Ok(resolve_effective_inference(
            &project.config.agent.inference,
            session.inference.as_ref(),
        ))
    }

    pub async fn effective_agent_config_for_session(
        &self,
        session_id: Ulid,
    ) -> Result<AgentConfig, BrainError> {
        let session = self.store.session_get(session_id).await?;
        let project = self.store.project_get(session.project_id).await?;
        let mut config = project.config.agent.clone();
        config.inference = resolve_effective_inference(
            &project.config.agent.inference,
            session.inference.as_ref(),
        );
        Ok(config)
    }

    pub async fn resolve_or_create_project(
        &self,
        root: std::path::PathBuf,
    ) -> Result<Project, BrainError> {
        let normalized_root = normalize_project_root(&root);
        if let Some(project) = self.store.project_find_by_root(&normalized_root).await? {
            return Ok(project);
        }

        let name = normalized_root
            .file_name()
            .and_then(|segment| segment.to_str())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| "brain".to_owned());
        let project = Project::new(Some(name), Some(normalized_root), ProjectConfig::default());
        self.store.project_create(project).await
    }
}

#[allow(clippy::too_many_arguments)]
async fn turn_inner(
    store: Arc<dyn Store>,
    provider: Arc<dyn Provider>,
    agent_loop: Arc<dyn AgentLoop>,
    tools: Vec<Arc<dyn Tool>>,
    cancel: CancellationToken,
    session_id: Ulid,
    user_msg: Message,
    tx: tokio::sync::mpsc::Sender<Event>,
) -> Result<(), BrainError> {
    let config = load_agent_config_for_session(store.clone(), session_id).await?;
    let mut history = store.message_list(session_id).await?;
    history.push(user_msg.clone());

    let mut inner_stream =
        agent_loop.run(provider, tools, history, config, cancel, Some(session_id));

    let mut new_messages: Vec<Message> = vec![user_msg];

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
            _ => {}
        }
        let _ = tx.send(event).await;
    }

    store.message_append(session_id, &new_messages).await?;

    Ok(())
}

fn resolve_effective_inference(
    defaults: &InferenceConfig,
    session_inference: Option<&InferenceConfig>,
) -> InferenceConfig {
    match session_inference {
        Some(overrides) => defaults.merged_with(overrides),
        None => defaults.clone(),
    }
}

async fn load_agent_config_for_session(
    store: Arc<dyn Store>,
    session_id: Ulid,
) -> Result<AgentConfig, BrainError> {
    let session = store.session_get(session_id).await?;
    let project = store.project_get(session.project_id).await?;
    let mut config = project.config.agent.clone();
    config.inference =
        resolve_effective_inference(&project.config.agent.inference, session.inference.as_ref());
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use brain_loops::SimpleLoop;
    use brain_providers::MockProvider;
    use brain_stores::InMemoryStore;
    use futures::future::BoxFuture;

    async fn make_brain() -> (Brain, Project, Arc<InMemoryStore>) {
        let project = Project::with_defaults("test");
        let provider: Arc<dyn Provider> = Arc::new(MockProvider::new());
        let store = Arc::new(InMemoryStore::new());
        let agent_loop: Arc<dyn AgentLoop> = Arc::new(SimpleLoop);

        let brain = Brain::new(provider, store.clone(), agent_loop, vec![]);
        store.project_create(project.clone()).await.unwrap();
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
        store.project_create(project).await.unwrap();

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
                    inference: InferenceConfig {
                        provider: Some("openai".into()),
                        model: Some("gpt-4o-mini".into()),
                        max_tokens: Some(4096),
                        temperature: Some(0.7),
                    },
                },
            },
        );
        let project = store.project_create(project).await.unwrap();

        let session = brain.create_session(project.id).await.unwrap();
        brain
            .update_session_inference(
                session.id,
                Some(InferenceConfig {
                    provider: None,
                    model: Some("gpt-5".into()),
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

        let messages = store.message_list(session.id).await.unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, Role::User);
        assert_eq!(messages[0].content, "hello world");
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

        let messages = store.message_list(session.id).await.unwrap();
        assert_eq!(messages.len(), 4);
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

        let alt_messages = store.message_list(alt_session.id).await.unwrap();
        assert!(!alt_messages.is_empty());
        assert_eq!(alt_messages[0].role, Role::User);
        assert_eq!(alt_messages[0].content, "hello world");
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

        let messages = store.message_list(session.id).await.unwrap();
        assert!(!messages.is_empty());
        assert_eq!(messages[0].role, Role::User);
        assert_eq!(messages[0].content, "resumed hello");
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
        let other_project = store
            .project_create(Project::with_defaults("other"))
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
