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
    pub fn turn(
        &self,
        session_id: Ulid,
        input: &str,
        config: AgentConfig,
        cancel: CancellationToken,
    ) -> EventStream {
        let store = self.store.clone();
        let provider = self.provider.clone();
        let agent_loop = self.agent_loop.clone();
        let tools = self.tools.clone();
        let user_msg = Message::user(input);

        let (tx, rx) = tokio::sync::mpsc::channel::<Event>(256);

        tokio::spawn(async move {
            if let Err(e) = turn_inner(
                store, provider, agent_loop, tools, config, cancel,
                session_id, user_msg, tx.clone(),
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
            transport.send(Event::SessionResume { session_id: id }).await?;
            id
        } else {
            let session = self.store.session_create(project.id).await?;
            tracing::info!(session_id = %session.id, "session started");
            transport.send(Event::SessionStart { session_id: session.id }).await?;
            session.id
        };
        let config = project.config.agent.clone();
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
                    transport.send(Event::SessionResume { session_id: target }).await?;
                    continue;
                }
                None => break,
                Some(_) => continue,
            };

            let cancel = CancellationToken::new();
            active_cancel = Some(cancel.clone());
            let mut events = self.turn(session_id, &input, config.clone(), cancel);

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
}

#[allow(clippy::too_many_arguments)]
async fn turn_inner(
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
    let mut history = store.message_list(session_id).await?;
    history.push(user_msg.clone());

    let mut inner_stream = agent_loop.run(
        provider,
        tools,
        history,
        config,
        cancel,
        Some(session_id),
    );

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

#[cfg(test)]
mod tests {
    use super::*;
    use futures::future::BoxFuture;
    use brain_providers::MockProvider;
    use brain_stores::InMemoryStore;
    use brain_loops::SimpleLoop;

    fn make_brain() -> (Brain, Project, Arc<InMemoryStore>) {
        let project = Project::with_defaults("test");
        let provider: Arc<dyn Provider> = Arc::new(MockProvider::new());
        let store = Arc::new(InMemoryStore::new());
        let agent_loop: Arc<dyn AgentLoop> = Arc::new(SimpleLoop);

        let brain = Brain::new(
            provider,
            store.clone(),
            agent_loop,
            vec![],
        );
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
        let (brain, project, _store) = make_brain();

        let s1 = brain.create_session(project.id).await.unwrap();
        let s2 = brain.create_session(project.id).await.unwrap();
        assert_ne!(s1.id, s2.id);

        let list = brain.list_sessions(project.id).await.unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn turn_produces_events_and_persists() {
        let (brain, project, store) = make_brain();

        let session = brain.create_session(project.id).await.unwrap();
        let config = project.config.agent.clone();
        let cancel = CancellationToken::new();
        let mut events = brain.turn(session.id, "hello world", config, cancel);

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
        let (brain, project, store) = make_brain();
        let session = brain.create_session(project.id).await.unwrap();
        let config = project.config.agent.clone();

        // First turn
        let mut events = brain.turn(session.id, "first", config.clone(), CancellationToken::new());
        while events.next().await.is_some() {}

        // Second turn
        let mut events = brain.turn(session.id, "second", config, CancellationToken::new());
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
            fn name(&self) -> &str { "mock" }

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

        let (brain, project, _store) = make_brain();
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
            fn name(&self) -> &str { "mock" }

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

        let (brain, project, _store) = make_brain();
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

        let (brain, project, store) = make_brain();
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
            fn name(&self) -> &str { "mock" }

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

        let (brain, project, store) = make_brain();
        let _ = store.project_create(project.clone()).await.unwrap();
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
            fn name(&self) -> &str { "mock" }
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

        let (brain, project, store) = make_brain();
        let _ = store.project_create(project.clone()).await.unwrap();
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
        assert!(result.is_err(), "should error when session belongs to different project");
    }
}
