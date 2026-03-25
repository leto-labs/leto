use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use chrono::Utc;
use futures::{StreamExt, future::BoxFuture};
use tokio::sync::{RwLock, broadcast};
use tokio_stream::wrappers::BroadcastStream;

use crate::{
    CredentialEntry, CredentialHealth, CredentialStore, CredentialStoreEvent, CredentialStoreKey,
    MessageStore, MessageStoreEvent, Project, ProjectId, ProjectStore, ProjectStoreEvent, Session,
    SessionId, SessionStore, SessionStoreEvent, SessionUpdate, Store, StoreError, StoreEvent,
    StoreStream, StoredMessage, TrajectoryStore, TrajectoryStoreEvent, normalize_project_root,
};

#[derive(Debug, Clone, Default)]
pub(crate) struct StoreData {
    pub projects: BTreeMap<ProjectId, Project>,
    pub sessions: BTreeMap<SessionId, Session>,
    pub messages: BTreeMap<SessionId, Vec<StoredMessage>>,
    pub credentials: BTreeMap<CredentialStoreKey, CredentialEntry>,
    pub trajectories: BTreeMap<SessionId, atif::Trajectory>,
}

/// In-memory store implementation for tests and transient flows.
#[derive(Clone)]
pub struct InMemoryStore {
    data: Arc<RwLock<StoreData>>,
    events: broadcast::Sender<StoreEvent>,
}

impl InMemoryStore {
    /// Creates an empty in-memory store.
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            data: Arc::new(RwLock::new(StoreData::default())),
            events,
        }
    }

    pub(crate) fn from_data(data: StoreData) -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            data: Arc::new(RwLock::new(data)),
            events,
        }
    }

    pub(crate) async fn snapshot(&self) -> StoreData {
        self.data.read().await.clone()
    }

    fn emit(&self, event: StoreEvent) {
        let _ = self.events.send(event);
    }
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

fn filtered_stream<T>(
    rx: broadcast::Receiver<StoreEvent>,
    map: fn(StoreEvent) -> Option<T>,
) -> StoreStream<T>
where
    T: Send + 'static,
{
    Box::pin(BroadcastStream::new(rx).filter_map(move |item| async move {
        match item {
            Ok(event) => map(event),
            Err(_) => None,
        }
    }))
}

impl ProjectStore for InMemoryStore {
    fn create(&self, mut project: Project) -> BoxFuture<'_, Result<Project, StoreError>> {
        Box::pin(async move {
            let mut data = self.data.write().await;
            if data.projects.contains_key(&project.id) {
                return Err(StoreError::AlreadyExists(format!("project {}", project.id)));
            }
            if let Some(root) = project.root.clone() {
                project.root = Some(normalize_project_root(&root));
            }
            data.projects.insert(project.id, project.clone());
            drop(data);
            self.emit(StoreEvent::Project(ProjectStoreEvent::Created {
                project: project.clone(),
            }));
            Ok(project)
        })
    }

    fn get(&self, id: ProjectId) -> BoxFuture<'_, Result<Project, StoreError>> {
        Box::pin(async move {
            self.data
                .read()
                .await
                .projects
                .get(&id)
                .cloned()
                .ok_or_else(|| StoreError::NotFound(format!("project {id}")))
        })
    }

    fn list(&self) -> BoxFuture<'_, Result<Vec<Project>, StoreError>> {
        Box::pin(async move {
            let mut projects = self
                .data
                .read()
                .await
                .projects
                .values()
                .cloned()
                .collect::<Vec<_>>();
            projects.sort_by_key(|project| project.created_at);
            Ok(projects)
        })
    }

    fn update(
        &self,
        id: ProjectId,
        mut project: Project,
    ) -> BoxFuture<'_, Result<Project, StoreError>> {
        Box::pin(async move {
            let mut data = self.data.write().await;
            if !data.projects.contains_key(&id) {
                return Err(StoreError::NotFound(format!("project {id}")));
            }
            if let Some(root) = project.root.clone() {
                project.root = Some(normalize_project_root(&root));
            }
            project.updated_at = Utc::now();
            data.projects.insert(id, project.clone());
            drop(data);
            self.emit(StoreEvent::Project(ProjectStoreEvent::Updated {
                project: project.clone(),
            }));
            Ok(project)
        })
    }

    fn delete(&self, id: ProjectId) -> BoxFuture<'_, Result<(), StoreError>> {
        Box::pin(async move {
            let mut data = self.data.write().await;
            if data.projects.remove(&id).is_none() {
                return Err(StoreError::NotFound(format!("project {id}")));
            }
            let session_ids = data
                .sessions
                .values()
                .filter(|session| session.project_id == id)
                .map(|session| session.id)
                .collect::<Vec<_>>();
            for session_id in session_ids {
                data.sessions.remove(&session_id);
                data.messages.remove(&session_id);
                data.trajectories.remove(&session_id);
            }
            drop(data);
            self.emit(StoreEvent::Project(ProjectStoreEvent::Deleted {
                project_id: id,
            }));
            Ok(())
        })
    }

    fn find_by_root(&self, root: &Path) -> BoxFuture<'_, Result<Option<Project>, StoreError>> {
        let normalized = normalize_project_root(root);
        Box::pin(async move {
            let data = self.data.read().await;
            Ok(data
                .projects
                .values()
                .find(|project| project.root.as_ref() == Some(&normalized))
                .cloned())
        })
    }

    fn subscribe(&self) -> StoreStream<ProjectStoreEvent> {
        filtered_stream(self.events.subscribe(), |event| match event {
            StoreEvent::Project(event) => Some(event),
            _ => None,
        })
    }
}

impl SessionStore for InMemoryStore {
    fn create(&self, session: Session) -> BoxFuture<'_, Result<Session, StoreError>> {
        Box::pin(async move {
            let mut data = self.data.write().await;
            if !data.projects.contains_key(&session.project_id) {
                return Err(StoreError::NotFound(format!(
                    "project {}",
                    session.project_id
                )));
            }
            if data.sessions.contains_key(&session.id) {
                return Err(StoreError::AlreadyExists(format!("session {}", session.id)));
            }
            data.sessions.insert(session.id, session.clone());
            drop(data);
            self.emit(StoreEvent::Session(SessionStoreEvent::Created {
                session: session.clone(),
            }));
            Ok(session)
        })
    }

    fn get(&self, id: SessionId) -> BoxFuture<'_, Result<Session, StoreError>> {
        Box::pin(async move {
            self.data
                .read()
                .await
                .sessions
                .get(&id)
                .cloned()
                .ok_or_else(|| StoreError::NotFound(format!("session {id}")))
        })
    }

    fn list(&self) -> BoxFuture<'_, Result<Vec<Session>, StoreError>> {
        Box::pin(async move {
            let mut sessions = self
                .data
                .read()
                .await
                .sessions
                .values()
                .cloned()
                .collect::<Vec<_>>();
            sessions.sort_by_key(|session| session.created_at);
            Ok(sessions)
        })
    }

    fn list_for_project(
        &self,
        project_id: ProjectId,
    ) -> BoxFuture<'_, Result<Vec<Session>, StoreError>> {
        Box::pin(async move {
            let mut sessions = self
                .data
                .read()
                .await
                .sessions
                .values()
                .filter(|session| session.project_id == project_id)
                .cloned()
                .collect::<Vec<_>>();
            sessions.sort_by_key(|session| session.created_at);
            Ok(sessions)
        })
    }

    fn update(
        &self,
        id: SessionId,
        mut session: Session,
    ) -> BoxFuture<'_, Result<Session, StoreError>> {
        Box::pin(async move {
            let mut data = self.data.write().await;
            if !data.sessions.contains_key(&id) {
                return Err(StoreError::NotFound(format!("session {id}")));
            }
            session.updated_at = Utc::now();
            data.sessions.insert(id, session.clone());
            drop(data);
            self.emit(StoreEvent::Session(SessionStoreEvent::Updated {
                session: session.clone(),
            }));
            Ok(session)
        })
    }

    fn patch(
        &self,
        id: SessionId,
        update: SessionUpdate,
    ) -> BoxFuture<'_, Result<Session, StoreError>> {
        Box::pin(async move {
            let mut data = self.data.write().await;
            let session = data
                .sessions
                .get_mut(&id)
                .ok_or_else(|| StoreError::NotFound(format!("session {id}")))?;
            if let Some(title) = update.title {
                session.title = Some(title);
            }
            if let Some(provider) = update.provider {
                session.provider = provider;
            }
            if let Some(model) = update.model {
                session.model = model;
            }
            if let Some(loop_name) = update.loop_name {
                session.loop_name = loop_name;
            }
            if let Some(request) = update.request {
                session.request = request;
            }
            session.updated_at = Utc::now();
            let session = session.clone();
            drop(data);
            self.emit(StoreEvent::Session(SessionStoreEvent::Updated {
                session: session.clone(),
            }));
            Ok(session)
        })
    }

    fn delete(&self, id: SessionId) -> BoxFuture<'_, Result<(), StoreError>> {
        Box::pin(async move {
            let mut data = self.data.write().await;
            let session = data
                .sessions
                .remove(&id)
                .ok_or_else(|| StoreError::NotFound(format!("session {id}")))?;
            data.messages.remove(&id);
            data.trajectories.remove(&id);
            drop(data);
            self.emit(StoreEvent::Session(SessionStoreEvent::Deleted {
                session_id: id,
                project_id: session.project_id,
            }));
            Ok(())
        })
    }

    fn subscribe(&self) -> StoreStream<SessionStoreEvent> {
        filtered_stream(self.events.subscribe(), |event| match event {
            StoreEvent::Session(event) => Some(event),
            _ => None,
        })
    }
}

impl MessageStore for InMemoryStore {
    fn list_for_session(
        &self,
        session_id: SessionId,
    ) -> BoxFuture<'_, Result<Vec<StoredMessage>, StoreError>> {
        Box::pin(async move {
            Ok(self
                .data
                .read()
                .await
                .messages
                .get(&session_id)
                .cloned()
                .unwrap_or_default())
        })
    }

    fn replace_for_session(
        &self,
        session_id: SessionId,
        mut messages: Vec<StoredMessage>,
    ) -> BoxFuture<'_, Result<Vec<StoredMessage>, StoreError>> {
        Box::pin(async move {
            if !self.data.read().await.sessions.contains_key(&session_id) {
                return Err(StoreError::NotFound(format!("session {session_id}")));
            }
            for (idx, message) in messages.iter_mut().enumerate() {
                message.session_id = session_id;
                message.ordinal = idx as u64;
            }
            let count = messages.len();
            self.data
                .write()
                .await
                .messages
                .insert(session_id, messages.clone());
            self.emit(StoreEvent::Message(MessageStoreEvent::Replaced {
                session_id,
                message_count: count,
            }));
            Ok(messages)
        })
    }

    fn delete_for_session(&self, session_id: SessionId) -> BoxFuture<'_, Result<(), StoreError>> {
        Box::pin(async move {
            self.data.write().await.messages.remove(&session_id);
            self.emit(StoreEvent::Message(MessageStoreEvent::Replaced {
                session_id,
                message_count: 0,
            }));
            Ok(())
        })
    }

    fn subscribe(&self) -> StoreStream<MessageStoreEvent> {
        filtered_stream(self.events.subscribe(), |event| match event {
            StoreEvent::Message(event) => Some(event),
            _ => None,
        })
    }
}

impl CredentialStore for InMemoryStore {
    fn create(
        &self,
        key: CredentialStoreKey,
        credential: CredentialEntry,
    ) -> BoxFuture<'_, Result<CredentialEntry, StoreError>> {
        Box::pin(async move {
            let mut data = self.data.write().await;
            if data.credentials.contains_key(&key) {
                return Err(StoreError::AlreadyExists(format!(
                    "credential {}/{}",
                    key.0, key.1
                )));
            }
            data.credentials.insert(key.clone(), credential.clone());
            drop(data);
            self.emit(StoreEvent::Credential(CredentialStoreEvent::Created {
                key,
                credential: credential.clone(),
            }));
            Ok(credential)
        })
    }

    fn get(&self, key: CredentialStoreKey) -> BoxFuture<'_, Result<CredentialEntry, StoreError>> {
        Box::pin(async move {
            self.data
                .read()
                .await
                .credentials
                .get(&key)
                .cloned()
                .ok_or_else(|| StoreError::NotFound(format!("credential {}/{}", key.0, key.1)))
        })
    }

    fn list(
        &self,
    ) -> BoxFuture<'_, Result<Vec<(CredentialStoreKey, CredentialEntry)>, StoreError>> {
        Box::pin(async move {
            Ok(self
                .data
                .read()
                .await
                .credentials
                .iter()
                .map(|(key, credential)| (key.clone(), credential.clone()))
                .collect())
        })
    }

    fn list_for_provider(
        &self,
        provider_name: &str,
    ) -> BoxFuture<'_, Result<Vec<(CredentialStoreKey, CredentialEntry)>, StoreError>> {
        let provider_name = provider_name.to_owned();
        Box::pin(async move {
            Ok(self
                .data
                .read()
                .await
                .credentials
                .iter()
                .filter(|((provider, _), _)| provider == &provider_name)
                .map(|(key, credential)| (key.clone(), credential.clone()))
                .collect())
        })
    }

    fn update(
        &self,
        key: CredentialStoreKey,
        credential: CredentialEntry,
    ) -> BoxFuture<'_, Result<CredentialEntry, StoreError>> {
        Box::pin(async move {
            let mut data = self.data.write().await;
            if !data.credentials.contains_key(&key) {
                return Err(StoreError::NotFound(format!(
                    "credential {}/{}",
                    key.0, key.1
                )));
            }
            data.credentials.insert(key.clone(), credential.clone());
            drop(data);
            self.emit(StoreEvent::Credential(CredentialStoreEvent::Updated {
                key,
                credential: credential.clone(),
            }));
            Ok(credential)
        })
    }

    fn update_health(
        &self,
        provider_name: &str,
        credential_id: &str,
        health: &CredentialHealth,
    ) -> BoxFuture<'_, Result<(), StoreError>> {
        let provider_name = provider_name.to_owned();
        let credential_id = credential_id.to_owned();
        let health = health.clone();
        Box::pin(async move {
            let mut data = self.data.write().await;
            let key = (provider_name.clone(), credential_id.clone());
            let credential = data.credentials.get_mut(&key).ok_or_else(|| {
                StoreError::NotFound(format!("credential {provider_name}/{credential_id}"))
            })?;
            credential.health = health;
            let credential = credential.clone();
            drop(data);
            self.emit(StoreEvent::Credential(CredentialStoreEvent::Updated {
                key,
                credential,
            }));
            Ok(())
        })
    }

    fn delete(&self, key: CredentialStoreKey) -> BoxFuture<'_, Result<(), StoreError>> {
        Box::pin(async move {
            let mut data = self.data.write().await;
            if data.credentials.remove(&key).is_none() {
                return Err(StoreError::NotFound(format!(
                    "credential {}/{}",
                    key.0, key.1
                )));
            }
            drop(data);
            self.emit(StoreEvent::Credential(CredentialStoreEvent::Deleted {
                key,
            }));
            Ok(())
        })
    }

    fn subscribe(&self) -> StoreStream<CredentialStoreEvent> {
        filtered_stream(self.events.subscribe(), |event| match event {
            StoreEvent::Credential(event) => Some(event),
            _ => None,
        })
    }
}

impl TrajectoryStore for InMemoryStore {
    fn get_for_session(
        &self,
        session_id: SessionId,
    ) -> BoxFuture<'_, Result<Option<atif::Trajectory>, StoreError>> {
        Box::pin(async move {
            Ok(self
                .data
                .read()
                .await
                .trajectories
                .get(&session_id)
                .cloned())
        })
    }

    fn list(&self) -> BoxFuture<'_, Result<Vec<(SessionId, atif::Trajectory)>, StoreError>> {
        Box::pin(async move {
            Ok(self
                .data
                .read()
                .await
                .trajectories
                .iter()
                .map(|(session_id, trajectory)| (*session_id, trajectory.clone()))
                .collect())
        })
    }

    fn upsert(
        &self,
        session_id: SessionId,
        trajectory: atif::Trajectory,
    ) -> BoxFuture<'_, Result<atif::Trajectory, StoreError>> {
        Box::pin(async move {
            trajectory
                .validate()
                .map_err(|error| StoreError::InvalidInput(error.to_string()))?;
            let mut data = self.data.write().await;
            if !data.sessions.contains_key(&session_id) {
                return Err(StoreError::NotFound(format!("session {session_id}")));
            }
            let created = data
                .trajectories
                .insert(session_id, trajectory.clone())
                .is_none();
            drop(data);
            self.emit(StoreEvent::Trajectory(if created {
                TrajectoryStoreEvent::Created {
                    session_id,
                    trajectory: trajectory.clone(),
                }
            } else {
                TrajectoryStoreEvent::Updated {
                    session_id,
                    trajectory: trajectory.clone(),
                }
            }));
            Ok(trajectory)
        })
    }

    fn delete(&self, session_id: SessionId) -> BoxFuture<'_, Result<(), StoreError>> {
        Box::pin(async move {
            self.data.write().await.trajectories.remove(&session_id);
            self.emit(StoreEvent::Trajectory(TrajectoryStoreEvent::Deleted {
                session_id,
            }));
            Ok(())
        })
    }

    fn subscribe(&self) -> StoreStream<TrajectoryStoreEvent> {
        filtered_stream(self.events.subscribe(), |event| match event {
            StoreEvent::Trajectory(event) => Some(event),
            _ => None,
        })
    }
}

impl Store for InMemoryStore {
    fn projects(&self) -> &dyn ProjectStore {
        self
    }

    fn sessions(&self) -> &dyn SessionStore {
        self
    }

    fn messages(&self) -> &dyn MessageStore {
        self
    }

    fn credentials(&self) -> &dyn CredentialStore {
        self
    }

    fn trajectories(&self) -> &dyn TrajectoryStore {
        self
    }

    fn subscribe(&self) -> StoreStream<StoreEvent> {
        Box::pin(
            BroadcastStream::new(self.events.subscribe())
                .filter_map(|item| async move { item.ok() }),
        )
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;
    use provider::Message;

    use super::*;
    use crate::{ProjectConfig, StoredMessage};

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
    async fn memory_store_roundtrips_project_session_and_messages() {
        let store = InMemoryStore::new();
        let project = store
            .projects()
            .create(Project::new(
                Some("demo".into()),
                Some("/tmp/demo".into()),
                ProjectConfig::default(),
            ))
            .await
            .unwrap();
        let session = store
            .sessions()
            .create(Session::new(project.id))
            .await
            .unwrap();
        store
            .messages()
            .replace_for_session(
                session.id,
                vec![StoredMessage::new(
                    session.id,
                    0,
                    Message::user_text("hello"),
                )],
            )
            .await
            .unwrap();

        let messages = store.messages().list_for_session(session.id).await.unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].message.plain_text_lossy(), "hello");
    }

    #[tokio::test]
    async fn memory_store_emits_store_events() {
        let store = InMemoryStore::new();
        let mut events = Store::subscribe(&store);
        let project = store
            .projects()
            .create(Project::new(None, None, ProjectConfig::default()))
            .await
            .unwrap();

        let event = events.next().await.unwrap();
        match event {
            StoreEvent::Project(ProjectStoreEvent::Created { project: created }) => {
                assert_eq!(created.id, project.id);
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[tokio::test]
    async fn memory_store_persists_validated_trajectories() {
        let store = InMemoryStore::new();
        let project = store
            .projects()
            .create(Project::new(None, None, ProjectConfig::default()))
            .await
            .unwrap();
        let session = store
            .sessions()
            .create(Session::new(project.id))
            .await
            .unwrap();
        let trajectory = sample_trajectory(session.id);

        let saved = store
            .trajectories()
            .upsert(session.id, trajectory.clone())
            .await
            .unwrap();

        assert_eq!(saved.session_id, trajectory.session_id);
    }
}
