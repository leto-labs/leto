use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use chrono::Utc;
use futures::StreamExt;
use futures::future::BoxFuture;
use tokio::sync::{RwLock, broadcast};
use tokio_stream::wrappers::BroadcastStream;
use ulid::Ulid;

use brain_types::*;

const STORE_EVENT_CAPACITY: usize = 1024;

struct SessionData {
    session: Session,
    messages: Vec<Message>,
    trajectory: Option<atif::Trajectory>,
}

struct MemoryStoreInner {
    projects: RwLock<HashMap<ProjectId, Project>>,
    sessions: RwLock<HashMap<Ulid, SessionData>>,
    credentials: RwLock<HashMap<CredentialStoreKey, CredentialEntry>>,
    store_event_tx: broadcast::Sender<StoreEvent>,
    project_event_tx: broadcast::Sender<ProjectStoreEvent>,
    session_event_tx: broadcast::Sender<SessionStoreEvent>,
    message_event_tx: broadcast::Sender<MessageStoreEvent>,
    credential_event_tx: broadcast::Sender<CredentialStoreEvent>,
    trajectory_event_tx: broadcast::Sender<TrajectoryStoreEvent>,
}

impl MemoryStoreInner {
    fn new() -> Self {
        let (store_event_tx, _) = broadcast::channel(STORE_EVENT_CAPACITY);
        let (project_event_tx, _) = broadcast::channel(STORE_EVENT_CAPACITY);
        let (session_event_tx, _) = broadcast::channel(STORE_EVENT_CAPACITY);
        let (message_event_tx, _) = broadcast::channel(STORE_EVENT_CAPACITY);
        let (credential_event_tx, _) = broadcast::channel(STORE_EVENT_CAPACITY);
        let (trajectory_event_tx, _) = broadcast::channel(STORE_EVENT_CAPACITY);
        Self {
            projects: RwLock::new(HashMap::new()),
            sessions: RwLock::new(HashMap::new()),
            credentials: RwLock::new(HashMap::new()),
            store_event_tx,
            project_event_tx,
            session_event_tx,
            message_event_tx,
            credential_event_tx,
            trajectory_event_tx,
        }
    }

    fn publish_store(&self, event: StoreEvent) {
        if let Err(error) = self.store_event_tx.send(event) {
            tracing::debug!("store event dropped: {error}");
        }
    }

    fn publish_project(&self, event: ProjectStoreEvent) {
        if let Err(error) = self.project_event_tx.send(event) {
            tracing::debug!("project event dropped: {error}");
        }
    }

    fn publish_session(&self, event: SessionStoreEvent) {
        if let Err(error) = self.session_event_tx.send(event) {
            tracing::debug!("session event dropped: {error}");
        }
    }

    fn publish_message(&self, event: MessageStoreEvent) {
        if let Err(error) = self.message_event_tx.send(event) {
            tracing::debug!("message event dropped: {error}");
        }
    }

    fn publish_credential(&self, event: CredentialStoreEvent) {
        if let Err(error) = self.credential_event_tx.send(event) {
            tracing::debug!("credential event dropped: {error}");
        }
    }

    fn publish_trajectory(&self, event: TrajectoryStoreEvent) {
        if let Err(error) = self.trajectory_event_tx.send(event) {
            tracing::debug!("trajectory event dropped: {error}");
        }
    }
}

#[derive(Clone)]
struct MemoryProjectStore {
    inner: Arc<MemoryStoreInner>,
}

#[derive(Clone)]
struct MemorySessionStore {
    inner: Arc<MemoryStoreInner>,
}

#[derive(Clone)]
struct MemoryMessageStore {
    inner: Arc<MemoryStoreInner>,
}

#[derive(Clone)]
struct MemoryCredentialStore {
    inner: Arc<MemoryStoreInner>,
}

#[derive(Clone)]
struct MemoryTrajectoryStore {
    inner: Arc<MemoryStoreInner>,
}

pub struct InMemoryStore {
    projects: MemoryProjectStore,
    sessions: MemorySessionStore,
    messages: MemoryMessageStore,
    credentials: MemoryCredentialStore,
    trajectories: MemoryTrajectoryStore,
    inner: Arc<MemoryStoreInner>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        let inner = Arc::new(MemoryStoreInner::new());
        Self {
            projects: MemoryProjectStore {
                inner: Arc::clone(&inner),
            },
            sessions: MemorySessionStore {
                inner: Arc::clone(&inner),
            },
            messages: MemoryMessageStore {
                inner: Arc::clone(&inner),
            },
            credentials: MemoryCredentialStore {
                inner: Arc::clone(&inner),
            },
            trajectories: MemoryTrajectoryStore {
                inner: Arc::clone(&inner),
            },
            inner,
        }
    }
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

fn broadcast_stream<T: Clone + Send + 'static>(
    receiver: broadcast::Receiver<T>,
) -> CrudStoreEventStream<T> {
    Box::pin(
        BroadcastStream::new(receiver).filter_map(|result| async move {
            match result {
                Ok(event) => Some(event),
                Err(error) => {
                    tracing::warn!("store event receive error: {error}");
                    None
                }
            }
        }),
    )
}

impl CrudStore for MemoryProjectStore {
    type Key = ProjectId;
    type Record = Project;
    type Event = ProjectStoreEvent;

    fn create(
        &self,
        key: ProjectId,
        project: Project,
    ) -> BoxFuture<'_, Result<Project, BrainError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            if key != project.id {
                return Err(BrainError::Storage(format!(
                    "project key {key} does not match record id {}",
                    project.id
                )));
            }
            let mut projects = inner.projects.write().await;
            if projects.contains_key(&key) {
                return Err(BrainError::Storage(format!(
                    "project already exists: {key}"
                )));
            }
            projects.insert(key, project.clone());
            drop(projects);
            inner.publish_project(ProjectStoreEvent::Created {
                project: project.clone(),
            });
            inner.publish_store(StoreEvent::ProjectCreated {
                project: project.clone(),
            });
            Ok(project)
        })
    }

    fn get(&self, id: ProjectId) -> BoxFuture<'_, Result<Project, BrainError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let projects = inner.projects.read().await;
            projects
                .get(&id)
                .cloned()
                .ok_or_else(|| BrainError::Storage(format!("project not found: {id}")))
        })
    }

    fn list(&self) -> BoxFuture<'_, Result<Vec<Project>, BrainError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let projects = inner.projects.read().await;
            let mut list: Vec<Project> = projects.values().cloned().collect();
            list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
            Ok(list)
        })
    }

    fn update(
        &self,
        key: ProjectId,
        project: Project,
    ) -> BoxFuture<'_, Result<Project, BrainError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            if key != project.id {
                return Err(BrainError::Storage(format!(
                    "project key {key} does not match record id {}",
                    project.id
                )));
            }
            let mut projects = inner.projects.write().await;
            let existing = projects
                .get_mut(&key)
                .ok_or_else(|| BrainError::Storage(format!("project not found: {key}")))?;
            *existing = project.clone();
            drop(projects);
            inner.publish_project(ProjectStoreEvent::Updated {
                project: project.clone(),
            });
            inner.publish_store(StoreEvent::ProjectUpdated {
                project: project.clone(),
            });
            Ok(project)
        })
    }

    fn delete(&self, id: ProjectId) -> BoxFuture<'_, Result<(), BrainError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut projects = inner.projects.write().await;
            projects.remove(&id);
            drop(projects);

            let mut sessions = inner.sessions.write().await;
            let deleted_sessions = sessions
                .values()
                .filter(|data| data.session.project_id == id)
                .map(|data| data.session.clone())
                .collect::<Vec<_>>();
            sessions.retain(|_, data| data.session.project_id != id);
            drop(sessions);

            for session in deleted_sessions {
                inner.publish_session(SessionStoreEvent::Deleted {
                    session_id: session.id,
                    project_id: session.project_id,
                });
                inner.publish_store(StoreEvent::SessionDeleted {
                    session_id: session.id,
                    project_id: session.project_id,
                });
            }

            inner.publish_project(ProjectStoreEvent::Deleted { project_id: id });
            inner.publish_store(StoreEvent::ProjectDeleted { project_id: id });
            Ok(())
        })
    }

    fn subscribe(&self) -> ProjectStoreEventStream {
        broadcast_stream(self.inner.project_event_tx.subscribe())
    }
}

impl ProjectStore for MemoryProjectStore {
    fn find_by_root(&self, root: &Path) -> BoxFuture<'_, Result<Option<Project>, BrainError>> {
        let inner = Arc::clone(&self.inner);
        let root = normalize_project_root(root);
        Box::pin(async move {
            let projects = inner.projects.read().await;
            Ok(projects.values().find_map(|project| {
                project.root.as_ref().and_then(|project_root| {
                    (normalize_project_root(project_root) == root).then(|| project.clone())
                })
            }))
        })
    }
}

impl CrudStore for MemorySessionStore {
    type Key = Ulid;
    type Record = Session;
    type Event = SessionStoreEvent;

    fn create(&self, key: Ulid, session: Session) -> BoxFuture<'_, Result<Session, BrainError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            if key != session.id {
                return Err(BrainError::Storage(format!(
                    "session key {key} does not match record id {}",
                    session.id
                )));
            }
            let mut sessions = inner.sessions.write().await;
            if sessions.contains_key(&key) {
                return Err(BrainError::Storage(format!(
                    "session already exists: {key}"
                )));
            }
            sessions.insert(
                key,
                SessionData {
                    session: session.clone(),
                    messages: Vec::new(),
                    trajectory: None,
                },
            );
            drop(sessions);
            inner.publish_session(SessionStoreEvent::Created {
                session: session.clone(),
            });
            inner.publish_store(StoreEvent::SessionCreated {
                session: session.clone(),
            });
            Ok(session)
        })
    }

    fn get(&self, id: Ulid) -> BoxFuture<'_, Result<Session, BrainError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let sessions = inner.sessions.read().await;
            sessions
                .get(&id)
                .map(|data| data.session.clone())
                .ok_or_else(|| BrainError::Storage(format!("session not found: {id}")))
        })
    }

    fn list(&self) -> BoxFuture<'_, Result<Vec<Session>, BrainError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let sessions = inner.sessions.read().await;
            let mut list: Vec<Session> =
                sessions.values().map(|data| data.session.clone()).collect();
            list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
            Ok(list)
        })
    }

    fn update(&self, key: Ulid, session: Session) -> BoxFuture<'_, Result<Session, BrainError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            if key != session.id {
                return Err(BrainError::Storage(format!(
                    "session key {key} does not match record id {}",
                    session.id
                )));
            }
            let mut sessions = inner.sessions.write().await;
            let entry = sessions
                .get_mut(&key)
                .ok_or_else(|| BrainError::Storage(format!("session not found: {key}")))?;
            entry.session = session.clone();
            drop(sessions);
            inner.publish_session(SessionStoreEvent::Updated {
                session: session.clone(),
            });
            inner.publish_store(StoreEvent::SessionUpdated {
                session: session.clone(),
            });
            Ok(session)
        })
    }

    fn delete(&self, id: Ulid) -> BoxFuture<'_, Result<(), BrainError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut sessions = inner.sessions.write().await;
            if let Some(data) = sessions.remove(&id) {
                let project_id = data.session.project_id;
                drop(sessions);
                inner.publish_session(SessionStoreEvent::Deleted {
                    session_id: id,
                    project_id,
                });
                inner.publish_store(StoreEvent::SessionDeleted {
                    session_id: id,
                    project_id,
                });
            }
            Ok(())
        })
    }

    fn subscribe(&self) -> SessionStoreEventStream {
        broadcast_stream(self.inner.session_event_tx.subscribe())
    }
}

impl SessionStore for MemorySessionStore {
    fn list_for_project(
        &self,
        project_id: ProjectId,
    ) -> BoxFuture<'_, Result<Vec<Session>, BrainError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let sessions = inner.sessions.read().await;
            let mut list: Vec<Session> = sessions
                .values()
                .filter(|data| data.session.project_id == project_id)
                .map(|data| data.session.clone())
                .collect();
            list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
            Ok(list)
        })
    }
}

impl CrudStore for MemoryMessageStore {
    type Key = MessageStoreKey;
    type Record = Message;
    type Event = MessageStoreEvent;

    fn create(
        &self,
        key: MessageStoreKey,
        message: Message,
    ) -> BoxFuture<'_, Result<Message, BrainError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let (session_id, message_id) = key;
            if message_id != message.id {
                return Err(BrainError::Storage(format!(
                    "message key {message_id} does not match record id {}",
                    message.id
                )));
            }
            let mut sessions = inner.sessions.write().await;
            let entry = sessions
                .get_mut(&session_id)
                .ok_or_else(|| BrainError::Storage(format!("session not found: {session_id}")))?;
            if entry.messages.iter().any(|stored| stored.id == message_id) {
                return Err(BrainError::Storage(format!(
                    "message already exists: {message_id}"
                )));
            }
            entry.messages.push(message.clone());
            entry.session.updated_at = Utc::now();
            drop(sessions);
            inner.publish_message(MessageStoreEvent::Created {
                key,
                message: message.clone(),
            });
            Ok(message)
        })
    }

    fn get(&self, key: MessageStoreKey) -> BoxFuture<'_, Result<Message, BrainError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let (session_id, message_id) = key;
            let sessions = inner.sessions.read().await;
            let entry = sessions
                .get(&session_id)
                .ok_or_else(|| BrainError::Storage(format!("session not found: {session_id}")))?;
            entry
                .messages
                .iter()
                .find(|message| message.id == message_id)
                .cloned()
                .ok_or_else(|| BrainError::Storage(format!("message not found: {message_id}")))
        })
    }

    fn list(&self) -> BoxFuture<'_, Result<Vec<Message>, BrainError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let sessions = inner.sessions.read().await;
            Ok(sessions
                .values()
                .flat_map(|entry| entry.messages.iter().cloned())
                .collect())
        })
    }

    fn update(
        &self,
        key: MessageStoreKey,
        message: Message,
    ) -> BoxFuture<'_, Result<Message, BrainError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let (session_id, message_id) = key;
            if message_id != message.id {
                return Err(BrainError::Storage(format!(
                    "message key {message_id} does not match record id {}",
                    message.id
                )));
            }
            let mut sessions = inner.sessions.write().await;
            let entry = sessions
                .get_mut(&session_id)
                .ok_or_else(|| BrainError::Storage(format!("session not found: {session_id}")))?;
            let stored = entry
                .messages
                .iter_mut()
                .find(|stored| stored.id == message_id)
                .ok_or_else(|| BrainError::Storage(format!("message not found: {message_id}")))?;
            *stored = message.clone();
            entry.session.updated_at = Utc::now();
            drop(sessions);
            inner.publish_message(MessageStoreEvent::Updated {
                key,
                message: message.clone(),
            });
            Ok(message)
        })
    }

    fn delete(&self, key: MessageStoreKey) -> BoxFuture<'_, Result<(), BrainError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let (session_id, message_id) = key;
            let mut sessions = inner.sessions.write().await;
            let entry = sessions
                .get_mut(&session_id)
                .ok_or_else(|| BrainError::Storage(format!("session not found: {session_id}")))?;
            let before = entry.messages.len();
            entry.messages.retain(|message| message.id != message_id);
            if entry.messages.len() == before {
                return Err(BrainError::Storage(format!(
                    "message not found: {message_id}"
                )));
            }
            entry.session.updated_at = Utc::now();
            drop(sessions);
            inner.publish_message(MessageStoreEvent::Deleted { key });
            Ok(())
        })
    }

    fn subscribe(&self) -> MessageStoreEventStream {
        broadcast_stream(self.inner.message_event_tx.subscribe())
    }
}

impl MessageStore for MemoryMessageStore {
    fn list_for_session(
        &self,
        session_id: Ulid,
    ) -> BoxFuture<'_, Result<Vec<Message>, BrainError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let sessions = inner.sessions.read().await;
            Ok(sessions
                .get(&session_id)
                .map(|data| data.messages.clone())
                .unwrap_or_default())
        })
    }
}

impl CrudStore for MemoryCredentialStore {
    type Key = CredentialStoreKey;
    type Record = CredentialEntry;
    type Event = CredentialStoreEvent;

    fn create(
        &self,
        key: CredentialStoreKey,
        credential: CredentialEntry,
    ) -> BoxFuture<'_, Result<CredentialEntry, BrainError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            if key.1 != credential.id {
                return Err(BrainError::Storage(format!(
                    "credential key {} does not match record id {}",
                    key.1, credential.id
                )));
            }
            let mut credentials = inner.credentials.write().await;
            if credentials.contains_key(&key) {
                return Err(BrainError::Storage(format!(
                    "credential already exists: {}:{}",
                    key.0, key.1
                )));
            }
            credentials.insert(key.clone(), credential.clone());
            drop(credentials);
            inner.publish_credential(CredentialStoreEvent::Created {
                key,
                credential: credential.clone(),
            });
            Ok(credential)
        })
    }

    fn get(&self, key: CredentialStoreKey) -> BoxFuture<'_, Result<CredentialEntry, BrainError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            inner
                .credentials
                .read()
                .await
                .get(&key)
                .cloned()
                .ok_or_else(|| {
                    BrainError::Storage(format!("credential not found: {}:{}", key.0, key.1))
                })
        })
    }

    fn list(&self) -> BoxFuture<'_, Result<Vec<CredentialEntry>, BrainError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let credentials = inner.credentials.read().await;
            Ok(credentials.values().cloned().collect())
        })
    }

    fn update(
        &self,
        key: CredentialStoreKey,
        credential: CredentialEntry,
    ) -> BoxFuture<'_, Result<CredentialEntry, BrainError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            if key.1 != credential.id {
                return Err(BrainError::Storage(format!(
                    "credential key {} does not match record id {}",
                    key.1, credential.id
                )));
            }
            let mut credentials = inner.credentials.write().await;
            if !credentials.contains_key(&key) {
                return Err(BrainError::Storage(format!(
                    "credential not found: {}:{}",
                    key.0, key.1
                )));
            }
            credentials.insert(key.clone(), credential.clone());
            drop(credentials);
            inner.publish_credential(CredentialStoreEvent::Updated {
                key,
                credential: credential.clone(),
            });
            Ok(credential)
        })
    }

    fn delete(&self, key: CredentialStoreKey) -> BoxFuture<'_, Result<(), BrainError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            inner.credentials.write().await.remove(&key);
            inner.publish_credential(CredentialStoreEvent::Deleted { key });
            Ok(())
        })
    }

    fn subscribe(&self) -> CredentialStoreEventStream {
        broadcast_stream(self.inner.credential_event_tx.subscribe())
    }
}

impl CredentialStore for MemoryCredentialStore {
    fn list_for_provider(
        &self,
        provider_name: &str,
    ) -> BoxFuture<'_, Result<Vec<CredentialEntry>, BrainError>> {
        let inner = Arc::clone(&self.inner);
        let provider_name = provider_name.to_owned();
        Box::pin(async move {
            let credentials = inner.credentials.read().await;
            Ok(credentials
                .iter()
                .filter(|((stored_provider, _), _)| *stored_provider == provider_name)
                .map(|(_, credential)| credential.clone())
                .collect())
        })
    }

    fn update_health(
        &self,
        provider_name: &str,
        credential_id: &str,
        health: &CredentialHealth,
    ) -> BoxFuture<'_, Result<(), BrainError>> {
        let inner = Arc::clone(&self.inner);
        let key = (provider_name.to_owned(), credential_id.to_owned());
        let health = health.clone();
        Box::pin(async move {
            let mut credentials = inner.credentials.write().await;
            if let Some(credential) = credentials.get_mut(&key) {
                credential.health = health;
            }
            Ok(())
        })
    }
}

impl CrudStore for MemoryTrajectoryStore {
    type Key = Ulid;
    type Record = atif::Trajectory;
    type Event = TrajectoryStoreEvent;

    fn create(
        &self,
        session_id: Ulid,
        trajectory: atif::Trajectory,
    ) -> BoxFuture<'_, Result<atif::Trajectory, BrainError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut sessions = inner.sessions.write().await;
            let entry = sessions
                .get_mut(&session_id)
                .ok_or_else(|| BrainError::Storage(format!("session not found: {session_id}")))?;
            if entry.trajectory.is_some() {
                return Err(BrainError::Storage(format!(
                    "trajectory already exists for session: {session_id}"
                )));
            }
            entry.trajectory = Some(trajectory.clone());
            entry.session.updated_at = Utc::now();
            drop(sessions);
            inner.publish_trajectory(TrajectoryStoreEvent::Created {
                session_id,
                trajectory: trajectory.clone(),
            });
            inner.publish_store(StoreEvent::TrajectoryCreated {
                session_id,
                trajectory: trajectory.clone(),
            });
            Ok(trajectory)
        })
    }

    fn get(&self, session_id: Ulid) -> BoxFuture<'_, Result<atif::Trajectory, BrainError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let sessions = inner.sessions.read().await;
            let entry = sessions
                .get(&session_id)
                .ok_or_else(|| BrainError::Storage(format!("session not found: {session_id}")))?;
            entry
                .trajectory
                .clone()
                .ok_or_else(|| BrainError::Storage(format!("trajectory not found: {session_id}")))
        })
    }

    fn list(&self) -> BoxFuture<'_, Result<Vec<atif::Trajectory>, BrainError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let sessions = inner.sessions.read().await;
            Ok(sessions
                .values()
                .filter_map(|entry| entry.trajectory.clone())
                .collect())
        })
    }

    fn update(
        &self,
        session_id: Ulid,
        trajectory: atif::Trajectory,
    ) -> BoxFuture<'_, Result<atif::Trajectory, BrainError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut sessions = inner.sessions.write().await;
            let entry = sessions
                .get_mut(&session_id)
                .ok_or_else(|| BrainError::Storage(format!("session not found: {session_id}")))?;
            if entry.trajectory.is_none() {
                return Err(BrainError::Storage(format!(
                    "trajectory not found: {session_id}"
                )));
            }
            entry.trajectory = Some(trajectory.clone());
            entry.session.updated_at = Utc::now();
            drop(sessions);
            inner.publish_trajectory(TrajectoryStoreEvent::Updated {
                session_id,
                trajectory: trajectory.clone(),
            });
            inner.publish_store(StoreEvent::TrajectoryUpdated {
                session_id,
                trajectory: trajectory.clone(),
            });
            Ok(trajectory)
        })
    }

    fn delete(&self, session_id: Ulid) -> BoxFuture<'_, Result<(), BrainError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut sessions = inner.sessions.write().await;
            let entry = sessions
                .get_mut(&session_id)
                .ok_or_else(|| BrainError::Storage(format!("session not found: {session_id}")))?;
            entry.trajectory = None;
            entry.session.updated_at = Utc::now();
            drop(sessions);
            inner.publish_trajectory(TrajectoryStoreEvent::Deleted { session_id });
            inner.publish_store(StoreEvent::TrajectoryDeleted { session_id });
            Ok(())
        })
    }

    fn subscribe(&self) -> TrajectoryStoreEventStream {
        broadcast_stream(self.inner.trajectory_event_tx.subscribe())
    }
}

impl TrajectoryStore for MemoryTrajectoryStore {
    fn get_for_session(
        &self,
        session_id: Ulid,
    ) -> BoxFuture<'_, Result<Option<atif::Trajectory>, BrainError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let sessions = inner.sessions.read().await;
            let trajectory = sessions
                .get(&session_id)
                .ok_or_else(|| BrainError::Storage(format!("session not found: {session_id}")))?
                .trajectory
                .clone();
            Ok(trajectory)
        })
    }
}

impl Store for InMemoryStore {
    fn projects(&self) -> &dyn ProjectStore {
        &self.projects
    }

    fn sessions(&self) -> &dyn SessionStore {
        &self.sessions
    }

    fn messages(&self) -> &dyn MessageStore {
        &self.messages
    }

    fn credentials(&self) -> &dyn CredentialStore {
        &self.credentials
    }

    fn trajectories(&self) -> &dyn TrajectoryStore {
        &self.trajectories
    }

    fn subscribe(&self) -> StoreEventStream {
        broadcast_stream(self.inner.store_event_tx.subscribe())
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;

    use super::*;

    fn api_key_entry(id: &str, key: &str) -> CredentialEntry {
        CredentialEntry::api_key(id, key)
    }

    #[tokio::test]
    async fn project_crud_uses_keyed_access() {
        let store = InMemoryStore::new();
        let mut project = Project::with_defaults("test");
        let id = project.id;

        let created = store.projects().create(id, project.clone()).await.unwrap();
        assert_eq!(created.name.as_deref(), Some("test"));
        assert_eq!(store.projects().get(id).await.unwrap().id, id);
        assert_eq!(store.projects().list().await.unwrap().len(), 1);

        project.name = Some("renamed".into());
        store.projects().update(id, project.clone()).await.unwrap();
        assert_eq!(
            store.projects().get(id).await.unwrap().name.as_deref(),
            Some("renamed")
        );

        store.projects().delete(id).await.unwrap();
        assert!(store.projects().get(id).await.is_err());
    }

    #[tokio::test]
    async fn project_find_by_root_matches_normalized_path() {
        let store = InMemoryStore::new();
        let mut project = Project::with_defaults("repo");
        project.root = Some(std::path::PathBuf::from("/tmp/work/repo"));
        store
            .projects()
            .create(project.id, project.clone())
            .await
            .unwrap();

        let found = store
            .projects()
            .find_by_root(std::path::Path::new("/tmp/work/./repo"))
            .await
            .unwrap();
        assert_eq!(found.unwrap().id, project.id);
    }

    #[tokio::test]
    async fn session_crud_and_project_listing_work() {
        let store = InMemoryStore::new();
        let project = Project::with_defaults("test");
        store
            .projects()
            .create(project.id, project.clone())
            .await
            .unwrap();

        let mut session = Session::new(project.id);
        let id = session.id;
        store.sessions().create(id, session.clone()).await.unwrap();
        assert_eq!(store.sessions().get(id).await.unwrap().id, id);
        assert_eq!(
            store
                .sessions()
                .list_for_project(project.id)
                .await
                .unwrap()
                .len(),
            1
        );

        session.title = Some("My Chat".into());
        store.sessions().update(id, session.clone()).await.unwrap();
        assert_eq!(
            store.sessions().get(id).await.unwrap().title.as_deref(),
            Some("My Chat")
        );

        store.sessions().delete(id).await.unwrap();
        assert!(store.sessions().get(id).await.is_err());
    }

    #[tokio::test]
    async fn project_delete_cascades_sessions_and_emits_store_events() {
        let store = InMemoryStore::new();
        let project = Project::with_defaults("project");
        store
            .projects()
            .create(project.id, project.clone())
            .await
            .unwrap();
        let session = Session::new(project.id);
        store
            .sessions()
            .create(session.id, session.clone())
            .await
            .unwrap();

        let mut events = store.subscribe();
        store.projects().delete(project.id).await.unwrap();

        let first = events.next().await.unwrap();
        let second = events.next().await.unwrap();
        match (first, second) {
            (
                StoreEvent::SessionDeleted {
                    session_id,
                    project_id,
                },
                StoreEvent::ProjectDeleted {
                    project_id: deleted_project_id,
                },
            ) => {
                assert_eq!(session_id, session.id);
                assert_eq!(project_id, project.id);
                assert_eq!(deleted_project_id, project.id);
            }
            other => panic!("unexpected events: {other:?}"),
        }
    }

    #[tokio::test]
    async fn message_crud_uses_tuple_key() {
        let store = InMemoryStore::new();
        let project = Project::with_defaults("project");
        store
            .projects()
            .create(project.id, project.clone())
            .await
            .unwrap();
        let session = Session::new(project.id);
        store
            .sessions()
            .create(session.id, session.clone())
            .await
            .unwrap();

        let first = Message::user("first");
        let second = Message::assistant("second");
        let first_key = (session.id, first.id);
        let second_key = (session.id, second.id);

        store
            .messages()
            .create(first_key, first.clone())
            .await
            .unwrap();
        store
            .messages()
            .create(second_key, second.clone())
            .await
            .unwrap();

        let listed = store.messages().list_for_session(session.id).await.unwrap();
        assert_eq!(listed.len(), 2);
        assert_eq!(store.messages().get(first_key).await.unwrap().id, first.id);

        let updated = Message {
            content: "updated".into(),
            ..first.clone()
        };
        store
            .messages()
            .update(first_key, updated.clone())
            .await
            .unwrap();
        assert_eq!(
            store.messages().get(first_key).await.unwrap().content,
            updated.content
        );

        store.messages().delete(second_key).await.unwrap();
        assert_eq!(
            store
                .messages()
                .list_for_session(session.id)
                .await
                .unwrap()
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn credential_crud_uses_tuple_key() {
        let store = InMemoryStore::new();
        let key = ("openai".to_owned(), "key-1".to_owned());
        let entry = api_key_entry("key-1", "sk-123");

        store
            .credentials()
            .create(key.clone(), entry.clone())
            .await
            .unwrap();
        assert_eq!(
            store.credentials().get(key.clone()).await.unwrap().id,
            "key-1"
        );
        assert_eq!(
            store
                .credentials()
                .list_for_provider("openai")
                .await
                .unwrap()
                .len(),
            1
        );

        let mut updated = entry.clone();
        updated.health.record_error("test", None);
        store
            .credentials()
            .update(key.clone(), updated.clone())
            .await
            .unwrap();
        assert_eq!(
            store
                .credentials()
                .get(key.clone())
                .await
                .unwrap()
                .health
                .consecutive_errors,
            1
        );

        store
            .credentials()
            .update_health("openai", "key-1", &CredentialHealth::default())
            .await
            .unwrap();
        assert_eq!(
            store
                .credentials()
                .get(key.clone())
                .await
                .unwrap()
                .health
                .consecutive_errors,
            0
        );

        store.credentials().delete(key.clone()).await.unwrap();
        assert!(store.credentials().get(key).await.is_err());
    }

    #[tokio::test]
    async fn keyed_batch_helpers_are_available() {
        let store = InMemoryStore::new();
        let project = Project::with_defaults("project");
        store
            .projects()
            .create(project.id, project.clone())
            .await
            .unwrap();
        let session = Session::new(project.id);
        store
            .sessions()
            .create(session.id, session.clone())
            .await
            .unwrap();

        let first = Message::user("first");
        let second = Message::assistant("second");
        store
            .messages()
            .create_many(vec![
                ((session.id, first.id), first.clone()),
                ((session.id, second.id), second.clone()),
            ])
            .await
            .unwrap();

        let fetched = store
            .messages()
            .get_many(vec![(session.id, first.id), (session.id, second.id)])
            .await
            .unwrap();
        assert_eq!(fetched.len(), 2);
    }
}
