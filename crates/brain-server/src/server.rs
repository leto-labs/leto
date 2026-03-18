use std::collections::HashMap;
use std::sync::Arc;

use futures::StreamExt;
use futures::future::BoxFuture;
use tokio::sync::{RwLock, broadcast};
use tokio_util::sync::CancellationToken;
use ulid::Ulid;

use brain_core::Brain;
use brain_types::*;

use crate::api::BrainApi;
use crate::event_bus::EventBus;
use crate::types::{ServerEvent, ServerStatus};

struct Inner {
    brain: Brain,
    event_bus: EventBus,
    active_turns: RwLock<HashMap<Ulid, CancellationToken>>,
}

#[derive(Clone)]
pub struct BrainServer {
    inner: Arc<Inner>,
}

impl BrainServer {
    pub fn new(brain: Brain) -> Self {
        Self {
            inner: Arc::new(Inner {
                brain,
                event_bus: EventBus::default(),
                active_turns: RwLock::new(HashMap::new()),
            }),
        }
    }

    /// Returns self as an `Arc<dyn BrainApi>` for in-process clients (TUI, tests).
    pub fn client(&self) -> Arc<dyn BrainApi> {
        Arc::new(self.clone())
    }
}

impl BrainApi for BrainServer {
    // -- Project management --

    fn create_project(&self, project: Project) -> BoxFuture<'_, Result<Project, BrainError>> {
        Box::pin(async move { self.inner.brain.store.project_create(project).await })
    }

    fn list_projects(&self) -> BoxFuture<'_, Result<Vec<Project>, BrainError>> {
        Box::pin(async move { self.inner.brain.store.project_list().await })
    }

    fn get_project(&self, id: ProjectId) -> BoxFuture<'_, Result<Project, BrainError>> {
        Box::pin(async move { self.inner.brain.store.project_get(id).await })
    }

    fn update_project(
        &self,
        id: ProjectId,
        update: ProjectUpdate,
    ) -> BoxFuture<'_, Result<(), BrainError>> {
        Box::pin(async move { self.inner.brain.store.project_update(id, update).await })
    }

    fn delete_project(&self, id: ProjectId) -> BoxFuture<'_, Result<(), BrainError>> {
        Box::pin(async move { self.inner.brain.store.project_delete(id).await })
    }

    // -- Session management --

    fn create_session(&self, project_id: ProjectId) -> BoxFuture<'_, Result<Session, BrainError>> {
        Box::pin(async move { self.inner.brain.store.session_create(project_id).await })
    }

    fn list_sessions(
        &self,
        project_id: ProjectId,
    ) -> BoxFuture<'_, Result<Vec<Session>, BrainError>> {
        Box::pin(async move { self.inner.brain.store.session_list(project_id).await })
    }

    fn get_session(&self, id: Ulid) -> BoxFuture<'_, Result<Session, BrainError>> {
        Box::pin(async move { self.inner.brain.store.session_get(id).await })
    }

    fn update_session(
        &self,
        id: Ulid,
        update: SessionUpdate,
    ) -> BoxFuture<'_, Result<(), BrainError>> {
        Box::pin(async move { self.inner.brain.store.session_update(id, update).await })
    }

    fn delete_session(&self, id: Ulid) -> BoxFuture<'_, Result<(), BrainError>> {
        Box::pin(async move { self.inner.brain.store.session_delete(id).await })
    }

    // -- Message history --

    fn list_messages(&self, session_id: Ulid) -> BoxFuture<'_, Result<Vec<Message>, BrainError>> {
        Box::pin(async move { self.inner.brain.store.message_list(session_id).await })
    }

    // -- Turn management --

    fn send_message(
        &self,
        session_id: Ulid,
        content: &str,
    ) -> BoxFuture<'_, Result<(), BrainError>> {
        let content = content.to_owned();
        let inner = self.inner.clone();
        Box::pin(async move {
            let cancel = CancellationToken::new();

            {
                let mut turns = inner.active_turns.write().await;
                if turns.contains_key(&session_id) {
                    return Err(BrainError::TurnActive(session_id));
                }
                turns.insert(session_id, cancel.clone());
            }

            // Look up the session's project to get AgentConfig
            let session = inner.brain.store.session_get(session_id).await?;
            let project = inner.brain.store.project_get(session.project_id).await?;
            let config = project.config.agent.clone();

            let stream = inner.brain.turn(session_id, &content, config, cancel);

            tokio::spawn(drain_turn(inner, session_id, stream));
            Ok(())
        })
    }

    fn send_message_stream(
        &self,
        session_id: Ulid,
        content: &str,
    ) -> BoxFuture<'_, Result<EventStream, BrainError>> {
        let content = content.to_owned();
        let inner = self.inner.clone();
        Box::pin(async move {
            let cancel = CancellationToken::new();

            {
                let mut turns = inner.active_turns.write().await;
                if turns.contains_key(&session_id) {
                    return Err(BrainError::TurnActive(session_id));
                }
                turns.insert(session_id, cancel.clone());
            }

            let session = inner.brain.store.session_get(session_id).await?;
            let project = inner.brain.store.project_get(session.project_id).await?;
            let config = project.config.agent.clone();

            let stream = inner.brain.turn(session_id, &content, config, cancel);

            // Tap the stream: each event is published to the bus AND forwarded to the caller.
            let (tx, rx) = tokio::sync::mpsc::channel::<Event>(256);
            let inner_clone = inner.clone();
            tokio::spawn(async move {
                use futures::StreamExt;
                let mut stream = stream;
                while let Some(event) = stream.next().await {
                    inner_clone
                        .event_bus
                        .publish(ServerEvent::new(session_id, event.clone()));
                    if tx.send(event).await.is_err() {
                        break;
                    }
                }
                inner_clone.active_turns.write().await.remove(&session_id);
            });

            Ok(Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx)) as EventStream)
        })
    }

    fn cancel_turn(&self, session_id: Ulid) -> BoxFuture<'_, Result<(), BrainError>> {
        Box::pin(async move {
            let turns = self.inner.active_turns.read().await;
            if let Some(token) = turns.get(&session_id) {
                token.cancel();
            }
            Ok(())
        })
    }

    // -- Provider & credentials --

    fn list_providers(&self) -> BoxFuture<'_, Result<Vec<ProviderInfo>, BrainError>> {
        Box::pin(async move { Ok(vec![self.inner.brain.provider.info()]) })
    }

    fn list_credentials(
        &self,
    ) -> BoxFuture<'_, Result<Vec<(String, CredentialEntry)>, BrainError>> {
        Box::pin(async move { self.inner.brain.store.credential_list().await })
    }

    fn get_credentials(
        &self,
        provider_name: &str,
    ) -> BoxFuture<'_, Result<Vec<CredentialEntry>, BrainError>> {
        let name = provider_name.to_owned();
        Box::pin(async move { self.inner.brain.store.credential_load_all(&name).await })
    }

    fn save_credential(
        &self,
        provider_name: &str,
        entry: CredentialEntry,
    ) -> BoxFuture<'_, Result<(), BrainError>> {
        let name = provider_name.to_owned();
        Box::pin(async move { self.inner.brain.store.credential_save(&name, &entry).await })
    }

    fn delete_credential(
        &self,
        provider_name: &str,
        credential_id: &str,
    ) -> BoxFuture<'_, Result<(), BrainError>> {
        let name = provider_name.to_owned();
        let cid = credential_id.to_owned();
        Box::pin(async move { self.inner.brain.store.credential_delete(&name, &cid).await })
    }

    // -- Events & status --

    fn subscribe(&self) -> broadcast::Receiver<ServerEvent> {
        self.inner.event_bus.subscribe()
    }

    fn status(&self) -> BoxFuture<'_, Result<ServerStatus, BrainError>> {
        let inner = &self.inner;
        Box::pin(async move {
            let tools: Vec<String> = inner
                .brain
                .tools
                .iter()
                .map(|t| t.definition().name)
                .collect();
            let active = inner.active_turns.read().await;
            let active_turn_ids: Vec<Ulid> = active.keys().copied().collect();
            let projects = inner.brain.store.project_list().await?;

            let mut total_sessions = 0usize;
            for p in &projects {
                let sessions = inner.brain.store.session_list(p.id).await?;
                total_sessions += sessions.len();
            }

            Ok(ServerStatus {
                providers: vec![inner.brain.provider.info()],
                tools,
                active_sessions: total_sessions,
                active_turns: active_turn_ids,
            })
        })
    }
}

async fn drain_turn(inner: Arc<Inner>, session_id: Ulid, mut stream: EventStream) {
    while let Some(event) = stream.next().await {
        inner.event_bus.publish(ServerEvent::new(session_id, event));
    }
    inner.active_turns.write().await.remove(&session_id);
}

#[cfg(test)]
mod tests {
    use super::*;
    use brain_loops::SimpleLoop;
    use brain_providers::MockProvider;
    use brain_stores::InMemoryStore;

    fn make_brain() -> Brain {
        let provider: Arc<dyn Provider> = Arc::new(MockProvider::new());
        let store = Arc::new(InMemoryStore::new());
        let agent_loop: Arc<dyn AgentLoop> = Arc::new(SimpleLoop);
        Brain::new(provider, store, agent_loop, vec![])
    }

    fn make_server() -> BrainServer {
        BrainServer::new(make_brain())
    }

    async fn make_project(server: &BrainServer) -> Project {
        server
            .create_project(Project::with_defaults("test"))
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn project_crud() {
        let server = make_server();

        let p = server
            .create_project(Project::with_defaults("myproject"))
            .await
            .unwrap();
        assert_eq!(p.name.as_deref(), Some("myproject"));

        let got = server.get_project(p.id).await.unwrap();
        assert_eq!(got.id, p.id);

        let list = server.list_projects().await.unwrap();
        assert_eq!(list.len(), 1);

        server.delete_project(p.id).await.unwrap();
        let list = server.list_projects().await.unwrap();
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn create_and_list_sessions() {
        let server = make_server();
        let project = make_project(&server).await;

        let s1 = server.create_session(project.id).await.unwrap();
        let s2 = server.create_session(project.id).await.unwrap();
        assert_ne!(s1.id, s2.id);

        let list = server.list_sessions(project.id).await.unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn sessions_scoped_to_project() {
        let server = make_server();
        let p1 = make_project(&server).await;
        let p2 = server
            .create_project(Project::with_defaults("other"))
            .await
            .unwrap();

        server.create_session(p1.id).await.unwrap();
        server.create_session(p1.id).await.unwrap();
        server.create_session(p2.id).await.unwrap();

        assert_eq!(server.list_sessions(p1.id).await.unwrap().len(), 2);
        assert_eq!(server.list_sessions(p2.id).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn get_session() {
        let server = make_server();
        let project = make_project(&server).await;
        let s = server.create_session(project.id).await.unwrap();

        let got = server.get_session(s.id).await.unwrap();
        assert_eq!(got.id, s.id);
        assert_eq!(got.project_id, project.id);
    }

    #[tokio::test]
    async fn delete_session() {
        let server = make_server();
        let project = make_project(&server).await;
        let s = server.create_session(project.id).await.unwrap();

        server.delete_session(s.id).await.unwrap();
        let list = server.list_sessions(project.id).await.unwrap();
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn send_message_produces_events() {
        let server = make_server();
        let project = make_project(&server).await;
        let session = server.create_session(project.id).await.unwrap();

        let mut rx = server.subscribe();
        server.send_message(session.id, "hello").await.unwrap();

        let mut saw_token = false;

        loop {
            let ev = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
                .await
                .expect("timed out waiting for event")
                .expect("channel closed");

            assert_eq!(ev.session_id, session.id);
            match &ev.event {
                Event::Token { .. } => saw_token = true,
                Event::TurnDone { .. } => break,
                _ => {}
            }
        }

        assert!(saw_token, "should have received token events");
    }

    #[tokio::test]
    async fn reject_concurrent_turn_same_session() {
        let provider: Arc<dyn Provider> = Arc::new(MockProvider::new().with_delay(200));
        let store = Arc::new(InMemoryStore::new());
        let agent_loop: Arc<dyn AgentLoop> = Arc::new(SimpleLoop);
        let brain = Brain::new(provider, store, agent_loop, vec![]);
        let server = BrainServer::new(brain);

        let project = make_project(&server).await;
        let session = server.create_session(project.id).await.unwrap();

        server.send_message(session.id, "first").await.unwrap();

        let result = server.send_message(session.id, "second").await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), BrainErrorCode::TurnActive);
    }

    #[tokio::test]
    async fn concurrent_turns_different_sessions() {
        let server = make_server();
        let project = make_project(&server).await;
        let s1 = server.create_session(project.id).await.unwrap();
        let s2 = server.create_session(project.id).await.unwrap();

        server.send_message(s1.id, "hello").await.unwrap();
        server.send_message(s2.id, "world").await.unwrap();

        let mut rx = server.subscribe();
        let mut s1_done = false;
        let mut s2_done = false;

        for _ in 0..50 {
            match tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv()).await {
                Ok(Ok(ev)) => {
                    if matches!(&ev.event, Event::TurnDone { .. }) {
                        if ev.session_id == s1.id {
                            s1_done = true;
                        }
                        if ev.session_id == s2.id {
                            s2_done = true;
                        }
                    }
                    if s1_done && s2_done {
                        break;
                    }
                }
                _ => break,
            }
        }

        assert!(s1_done, "session 1 should complete");
        assert!(s2_done, "session 2 should complete");
    }

    #[tokio::test]
    async fn cancel_active_turn() {
        let provider: Arc<dyn Provider> = Arc::new(MockProvider::new().with_delay(500));
        let store = Arc::new(InMemoryStore::new());
        let agent_loop: Arc<dyn AgentLoop> = Arc::new(SimpleLoop);
        let brain = Brain::new(provider, store, agent_loop, vec![]);
        let server = BrainServer::new(brain);

        let project = make_project(&server).await;
        let session = server.create_session(project.id).await.unwrap();
        let mut rx = server.subscribe();

        server.send_message(session.id, "hello").await.unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        server.cancel_turn(session.id).await.unwrap();

        let mut saw_cancelled = false;
        for _ in 0..20 {
            match tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv()).await {
                Ok(Ok(ev)) => {
                    if let Event::Error {
                        code: BrainErrorCode::Cancelled,
                        ..
                    } = &ev.event
                    {
                        saw_cancelled = true;
                        break;
                    }
                }
                _ => break,
            }
        }

        assert!(saw_cancelled, "should receive cancelled event");
    }

    #[tokio::test]
    async fn cancel_no_active_turn_is_ok() {
        let server = make_server();
        let project = make_project(&server).await;
        let session = server.create_session(project.id).await.unwrap();
        server.cancel_turn(session.id).await.unwrap();
    }

    #[tokio::test]
    async fn status_reports_tools_and_sessions() {
        let server = make_server();
        let project = make_project(&server).await;
        server.create_session(project.id).await.unwrap();
        server.create_session(project.id).await.unwrap();

        let st = server.status().await.unwrap();
        assert_eq!(st.active_sessions, 2);
        assert!(st.active_turns.is_empty());
        assert!(st.tools.is_empty());
    }

    #[tokio::test]
    async fn client_returns_working_api() {
        let server = make_server();
        let client = server.client();

        let project = client
            .create_project(Project::with_defaults("test"))
            .await
            .unwrap();
        let session = client.create_session(project.id).await.unwrap();
        let list = client.list_sessions(project.id).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, session.id);
    }

    #[tokio::test]
    async fn active_turn_cleared_after_completion() {
        let server = make_server();
        let project = make_project(&server).await;
        let session = server.create_session(project.id).await.unwrap();

        server.send_message(session.id, "hello").await.unwrap();

        let mut rx = server.subscribe();
        loop {
            match tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv()).await {
                Ok(Ok(ev)) if matches!(&ev.event, Event::TurnDone { .. }) => break,
                Ok(Ok(_)) => continue,
                _ => panic!("timed out waiting for TurnDone"),
            }
        }

        server.send_message(session.id, "again").await.unwrap();
    }

    #[tokio::test]
    async fn update_session_title() {
        let server = make_server();
        let project = make_project(&server).await;
        let session = server.create_session(project.id).await.unwrap();
        assert!(session.title.is_none());

        server
            .update_session(
                session.id,
                SessionUpdate {
                    title: Some("renamed".into()),
                },
            )
            .await
            .unwrap();

        let got = server.get_session(session.id).await.unwrap();
        assert_eq!(got.title.as_deref(), Some("renamed"));
    }

    #[tokio::test]
    async fn list_messages_empty_session() {
        let server = make_server();
        let project = make_project(&server).await;
        let session = server.create_session(project.id).await.unwrap();

        let msgs = server.list_messages(session.id).await.unwrap();
        assert!(msgs.is_empty());
    }

    #[tokio::test]
    async fn list_messages_after_turn() {
        let server = make_server();
        let project = make_project(&server).await;
        let session = server.create_session(project.id).await.unwrap();

        server.send_message(session.id, "hello").await.unwrap();

        // Wait for turn to complete
        let mut rx = server.subscribe();
        loop {
            match tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv()).await {
                Ok(Ok(ev)) if matches!(&ev.event, Event::TurnDone { .. }) => break,
                Ok(Ok(_)) => continue,
                _ => panic!("timed out waiting for TurnDone"),
            }
        }

        let msgs = server.list_messages(session.id).await.unwrap();
        assert!(!msgs.is_empty(), "should have messages after a turn");
        assert_eq!(msgs[0].role, brain_types::Role::User);
        assert_eq!(msgs[0].content, "hello");
    }

    #[tokio::test]
    async fn send_message_stream_returns_events() {
        let server = make_server();
        let project = make_project(&server).await;
        let session = server.create_session(project.id).await.unwrap();

        let mut stream = server
            .send_message_stream(session.id, "hello")
            .await
            .unwrap();

        let mut saw_token = false;
        let mut saw_done = false;

        while let Some(event) = futures::StreamExt::next(&mut stream).await {
            match &event {
                Event::Token { .. } => saw_token = true,
                Event::TurnDone { .. } => {
                    saw_done = true;
                    break;
                }
                _ => {}
            }
        }

        assert!(saw_token, "should have received token events");
        assert!(saw_done, "should have received TurnDone");
    }

    #[tokio::test]
    async fn send_message_stream_also_publishes_to_bus() {
        let server = make_server();
        let project = make_project(&server).await;
        let session = server.create_session(project.id).await.unwrap();

        let mut bus_rx = server.subscribe();
        let mut stream = server
            .send_message_stream(session.id, "hello")
            .await
            .unwrap();

        // Drain the stream to completion
        while futures::StreamExt::next(&mut stream).await.is_some() {}

        // Bus should also have received events
        let mut bus_saw_done = false;
        for _ in 0..50 {
            match tokio::time::timeout(std::time::Duration::from_secs(2), bus_rx.recv()).await {
                Ok(Ok(ev)) if matches!(&ev.event, Event::TurnDone { .. }) => {
                    bus_saw_done = true;
                    break;
                }
                Ok(Ok(_)) => continue,
                _ => break,
            }
        }

        assert!(bus_saw_done, "bus should have received TurnDone");
    }

    #[tokio::test]
    async fn list_providers_returns_mock() {
        let server = make_server();
        let providers = server.list_providers().await.unwrap();
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].name, "mock");
        assert!(!providers[0].models.is_empty());
    }

    #[tokio::test]
    async fn credential_crud() {
        let server = make_server();

        let list = server.list_credentials().await.unwrap();
        assert!(list.is_empty());

        let entry = brain_types::CredentialEntry::api_key("key-1", "sk-test");
        server.save_credential("openai", entry).await.unwrap();

        let loaded = server.get_credentials("openai").await.unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "key-1");
        match &loaded[0].credential {
            brain_types::ProviderCredential::ApiKey { api_key } => assert_eq!(api_key, "sk-test"),
            _ => panic!("expected ApiKey"),
        }

        let list = server.list_credentials().await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].0, "openai");

        server.delete_credential("openai", "key-1").await.unwrap();
        let loaded = server.get_credentials("openai").await.unwrap();
        assert!(loaded.is_empty());
    }

    #[tokio::test]
    async fn status_includes_provider_info() {
        let server = make_server();
        let st = server.status().await.unwrap();
        assert_eq!(st.providers.len(), 1);
        assert_eq!(st.providers[0].name, "mock");
    }
}
