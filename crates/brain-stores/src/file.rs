use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use futures::future::BoxFuture;
use futures::StreamExt;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use ulid::Ulid;

use brain_types::*;

const STORE_EVENT_CAPACITY: usize = 1024;

struct FileStoreInner {
    root: PathBuf,
    store_event_tx: broadcast::Sender<StoreEvent>,
    project_event_tx: broadcast::Sender<ProjectStoreEvent>,
    session_event_tx: broadcast::Sender<SessionStoreEvent>,
    message_event_tx: broadcast::Sender<MessageStoreEvent>,
    credential_event_tx: broadcast::Sender<CredentialStoreEvent>,
}

impl FileStoreInner {
    fn new(root: PathBuf) -> Self {
        let (store_event_tx, _) = broadcast::channel(STORE_EVENT_CAPACITY);
        let (project_event_tx, _) = broadcast::channel(STORE_EVENT_CAPACITY);
        let (session_event_tx, _) = broadcast::channel(STORE_EVENT_CAPACITY);
        let (message_event_tx, _) = broadcast::channel(STORE_EVENT_CAPACITY);
        let (credential_event_tx, _) = broadcast::channel(STORE_EVENT_CAPACITY);
        Self {
            root,
            store_event_tx,
            project_event_tx,
            session_event_tx,
            message_event_tx,
            credential_event_tx,
        }
    }

    fn projects_dir(&self) -> PathBuf {
        self.root.join("projects")
    }

    fn project_file(&self, id: ProjectId) -> PathBuf {
        self.projects_dir().join(format!("{id}.json"))
    }

    fn sessions_dir(&self) -> PathBuf {
        self.root.join("sessions")
    }

    fn session_dir(&self, session_id: Ulid) -> PathBuf {
        self.sessions_dir().join(session_id.to_string())
    }

    fn session_file(&self, session_id: Ulid) -> PathBuf {
        self.session_dir(session_id).join("session.json")
    }

    fn messages_file(&self, session_id: Ulid) -> PathBuf {
        self.session_dir(session_id).join("messages.jsonl")
    }

    fn credentials_dir(&self) -> PathBuf {
        self.root.join("credentials")
    }

    fn credential_provider_dir(&self, provider_name: &str) -> PathBuf {
        self.credentials_dir().join(provider_name)
    }

    fn credential_file(&self, provider_name: &str, credential_id: &str) -> PathBuf {
        self.credential_provider_dir(provider_name)
            .join(format!("{credential_id}.json"))
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

    async fn load_messages(&self, session_id: Ulid) -> Result<Vec<Message>, BrainError> {
        let path = self.messages_file(session_id);
        if !path.exists() {
            return Ok(Vec::new());
        }
        let data = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| BrainError::Storage(format!("read messages {}: {e}", path.display())))?;

        let mut messages = Vec::new();
        for line in data.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let message: Message = serde_json::from_str(line)
                .map_err(|e| BrainError::Storage(format!("parse message line: {e}")))?;
            messages.push(message);
        }
        Ok(messages)
    }

    async fn write_messages(&self, session_id: Ulid, messages: &[Message]) -> Result<(), BrainError> {
        let path = self.messages_file(session_id);
        let mut content = String::new();
        for message in messages {
            let line = serde_json::to_string(message)
                .map_err(|e| BrainError::Storage(format!("serialize message: {e}")))?;
            content.push_str(&line);
            content.push('\n');
        }
        tokio::fs::write(&path, content)
            .await
            .map_err(|e| BrainError::Storage(format!("write messages {}: {e}", path.display())))
    }

    async fn touch_session(&self, session_id: Ulid) -> Result<(), BrainError> {
        let path = self.session_file(session_id);
        if !path.exists() {
            return Ok(());
        }
        let mut session: Session = read_json(&path).await?;
        session.updated_at = Utc::now();
        write_json(&path, &session).await
    }

    async fn list_all_sessions(&self) -> Result<Vec<Session>, BrainError> {
        let sessions_dir = self.sessions_dir();
        if !sessions_dir.exists() {
            return Ok(Vec::new());
        }

        let mut entries = tokio::fs::read_dir(&sessions_dir)
            .await
            .map_err(|e| BrainError::Storage(format!("read sessions dir: {e}")))?;

        let mut sessions = Vec::new();
        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| BrainError::Storage(format!("read dir entry: {e}")))?
        {
            let path = entry.path().join("session.json");
            if !path.exists() {
                continue;
            }
            match read_json::<Session>(&path).await {
                Ok(session) => sessions.push(session),
                Err(_) => continue,
            }
        }
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(sessions)
    }
}

#[derive(Clone)]
struct FileProjectStore {
    inner: Arc<FileStoreInner>,
}

#[derive(Clone)]
struct FileSessionStore {
    inner: Arc<FileStoreInner>,
}

#[derive(Clone)]
struct FileMessageStore {
    inner: Arc<FileStoreInner>,
}

#[derive(Clone)]
struct FileCredentialStore {
    inner: Arc<FileStoreInner>,
}

pub struct FileStore {
    inner: Arc<FileStoreInner>,
    projects: FileProjectStore,
    sessions: FileSessionStore,
    messages: FileMessageStore,
    credentials: FileCredentialStore,
}

impl FileStore {
    pub async fn new(root: impl Into<PathBuf>) -> Result<Self, BrainError> {
        let root = root.into();
        tokio::fs::create_dir_all(root.join("projects"))
            .await
            .map_err(|e| BrainError::Storage(format!("failed to create projects dir: {e}")))?;
        tokio::fs::create_dir_all(root.join("sessions"))
            .await
            .map_err(|e| BrainError::Storage(format!("failed to create sessions dir: {e}")))?;
        tokio::fs::create_dir_all(root.join("credentials"))
            .await
            .map_err(|e| BrainError::Storage(format!("failed to create credentials dir: {e}")))?;

        let inner = Arc::new(FileStoreInner::new(root));
        Ok(Self {
            projects: FileProjectStore {
                inner: Arc::clone(&inner),
            },
            sessions: FileSessionStore {
                inner: Arc::clone(&inner),
            },
            messages: FileMessageStore {
                inner: Arc::clone(&inner),
            },
            credentials: FileCredentialStore {
                inner: Arc::clone(&inner),
            },
            inner,
        })
    }
}

fn broadcast_stream<T: Clone + Send + 'static>(
    receiver: broadcast::Receiver<T>,
) -> CrudStoreEventStream<T> {
    Box::pin(BroadcastStream::new(receiver).filter_map(|result| async move {
        match result {
            Ok(event) => Some(event),
            Err(error) => {
                tracing::warn!("store event receive error: {error}");
                None
            }
        }
    }))
}

async fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, BrainError> {
    let data = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| BrainError::Storage(format!("read {}: {e}", path.display())))?;
    serde_json::from_str(&data)
        .map_err(|e| BrainError::Storage(format!("parse {}: {e}", path.display())))
}

async fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), BrainError> {
    let data = serde_json::to_string_pretty(value)
        .map_err(|e| BrainError::Storage(format!("serialize: {e}")))?;
    tokio::fs::write(path, data)
        .await
        .map_err(|e| BrainError::Storage(format!("write {}: {e}", path.display())))
}

impl CrudStore for FileProjectStore {
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
            let path = inner.project_file(key);
            if path.exists() {
                return Err(BrainError::Storage(format!("project already exists: {key}")));
            }
            write_json(&path, &project).await?;
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
            let path = inner.project_file(id);
            if !path.exists() {
                return Err(BrainError::Storage(format!("project not found: {id}")));
            }
            read_json(&path).await
        })
    }

    fn list(&self) -> BoxFuture<'_, Result<Vec<Project>, BrainError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let projects_dir = inner.projects_dir();
            let mut entries = tokio::fs::read_dir(&projects_dir)
                .await
                .map_err(|e| BrainError::Storage(format!("read projects dir: {e}")))?;

            let mut projects = Vec::new();
            while let Some(entry) = entries
                .next_entry()
                .await
                .map_err(|e| BrainError::Storage(format!("read dir entry: {e}")))?
            {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                match read_json::<Project>(&path).await {
                    Ok(project) => projects.push(project),
                    Err(_) => continue,
                }
            }
            projects.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
            Ok(projects)
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
            let path = inner.project_file(key);
            if !path.exists() {
                return Err(BrainError::Storage(format!("project not found: {key}")));
            }
            write_json(&path, &project).await?;
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
            let path = inner.project_file(id);
            match tokio::fs::remove_file(&path).await {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(BrainError::Storage(format!("delete project: {error}")));
                }
            }

            let sessions = inner.list_all_sessions().await?;
            for session in sessions.into_iter().filter(|session| session.project_id == id) {
                let dir = inner.session_dir(session.id);
                if dir.exists() {
                    tokio::fs::remove_dir_all(&dir)
                        .await
                        .map_err(|e| BrainError::Storage(format!("delete session: {e}")))?;
                }
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

impl ProjectStore for FileProjectStore {
    fn find_by_root(&self, root: &Path) -> BoxFuture<'_, Result<Option<Project>, BrainError>> {
        let root = normalize_project_root(root);
        let project_store = self.clone();
        Box::pin(async move {
            let projects = project_store.list().await?;
            Ok(projects.into_iter().find(|project| {
                project
                    .root
                    .as_ref()
                    .map(|project_root| normalize_project_root(project_root) == root)
                    .unwrap_or(false)
            }))
        })
    }
}

impl CrudStore for FileSessionStore {
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
            let dir = inner.session_dir(key);
            let session_path = inner.session_file(key);
            if session_path.exists() {
                return Err(BrainError::Storage(format!("session already exists: {key}")));
            }
            tokio::fs::create_dir_all(&dir)
                .await
                .map_err(|e| BrainError::Storage(format!("create session dir: {e}")))?;
            write_json(&session_path, &session).await?;
            tokio::fs::write(inner.messages_file(key), "")
                .await
                .map_err(|e| BrainError::Storage(format!("create messages file: {e}")))?;
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
            let path = inner.session_file(id);
            if !path.exists() {
                return Err(BrainError::Storage(format!("session not found: {id}")));
            }
            read_json(&path).await
        })
    }

    fn list(&self) -> BoxFuture<'_, Result<Vec<Session>, BrainError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move { inner.list_all_sessions().await })
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
            let path = inner.session_file(key);
            if !path.exists() {
                return Err(BrainError::Storage(format!("session not found: {key}")));
            }
            write_json(&path, &session).await?;
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
            let session = match read_json::<Session>(&inner.session_file(id)).await {
                Ok(session) => Some(session),
                Err(_) => None,
            };
            let dir = inner.session_dir(id);
            if dir.exists() {
                tokio::fs::remove_dir_all(&dir)
                    .await
                    .map_err(|e| BrainError::Storage(format!("delete session: {e}")))?;
            }
            if let Some(session) = session {
                inner.publish_session(SessionStoreEvent::Deleted {
                    session_id: id,
                    project_id: session.project_id,
                });
                inner.publish_store(StoreEvent::SessionDeleted {
                    session_id: id,
                    project_id: session.project_id,
                });
            }
            Ok(())
        })
    }

    fn subscribe(&self) -> SessionStoreEventStream {
        broadcast_stream(self.inner.session_event_tx.subscribe())
    }
}

impl SessionStore for FileSessionStore {
    fn list_for_project(
        &self,
        project_id: ProjectId,
    ) -> BoxFuture<'_, Result<Vec<Session>, BrainError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let sessions = inner.list_all_sessions().await?;
            Ok(sessions
                .into_iter()
                .filter(|session| session.project_id == project_id)
                .collect())
        })
    }
}

impl CrudStore for FileMessageStore {
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
            if !inner.session_file(session_id).exists() {
                return Err(BrainError::Storage(format!("session not found: {session_id}")));
            }
            let mut messages = inner.load_messages(session_id).await?;
            if messages.iter().any(|stored| stored.id == message_id) {
                return Err(BrainError::Storage(format!("message already exists: {message_id}")));
            }
            messages.push(message.clone());
            inner.write_messages(session_id, &messages).await?;
            inner.touch_session(session_id).await?;
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
            if !inner.session_file(session_id).exists() {
                return Err(BrainError::Storage(format!("session not found: {session_id}")));
            }
            inner
                .load_messages(session_id)
                .await?
                .into_iter()
                .find(|message| message.id == message_id)
                .ok_or_else(|| BrainError::Storage(format!("message not found: {message_id}")))
        })
    }

    fn list(&self) -> BoxFuture<'_, Result<Vec<Message>, BrainError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let sessions = inner.list_all_sessions().await?;
            let mut messages = Vec::new();
            for session in sessions {
                messages.extend(inner.load_messages(session.id).await?);
            }
            Ok(messages)
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
            if !inner.session_file(session_id).exists() {
                return Err(BrainError::Storage(format!("session not found: {session_id}")));
            }
            let mut messages = inner.load_messages(session_id).await?;
            let stored = messages
                .iter_mut()
                .find(|stored| stored.id == message_id)
                .ok_or_else(|| BrainError::Storage(format!("message not found: {message_id}")))?;
            *stored = message.clone();
            inner.write_messages(session_id, &messages).await?;
            inner.touch_session(session_id).await?;
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
            if !inner.session_file(session_id).exists() {
                return Err(BrainError::Storage(format!("session not found: {session_id}")));
            }
            let mut messages = inner.load_messages(session_id).await?;
            let before = messages.len();
            messages.retain(|message| message.id != message_id);
            if messages.len() == before {
                return Err(BrainError::Storage(format!("message not found: {message_id}")));
            }
            inner.write_messages(session_id, &messages).await?;
            inner.touch_session(session_id).await?;
            inner.publish_message(MessageStoreEvent::Deleted { key });
            Ok(())
        })
    }

    fn subscribe(&self) -> MessageStoreEventStream {
        broadcast_stream(self.inner.message_event_tx.subscribe())
    }
}

impl MessageStore for FileMessageStore {
    fn list_for_session(&self, session_id: Ulid) -> BoxFuture<'_, Result<Vec<Message>, BrainError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            if !inner.session_file(session_id).exists() {
                return Ok(Vec::new());
            }
            inner.load_messages(session_id).await
        })
    }
}

impl CrudStore for FileCredentialStore {
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
            let path = inner.credential_file(&key.0, &key.1);
            if path.exists() {
                return Err(BrainError::Storage(format!(
                    "credential already exists: {}:{}",
                    key.0, key.1
                )));
            }
            tokio::fs::create_dir_all(inner.credential_provider_dir(&key.0))
                .await
                .map_err(|e| BrainError::Storage(format!("create credentials dir: {e}")))?;
            write_json(&path, &credential).await?;
            set_file_permissions_restrictive(&path).await?;
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
            let path = inner.credential_file(&key.0, &key.1);
            if !path.exists() {
                return Err(BrainError::Storage(format!(
                    "credential not found: {}:{}",
                    key.0, key.1
                )));
            }
            read_json(&path).await
        })
    }

    fn list(&self) -> BoxFuture<'_, Result<Vec<CredentialEntry>, BrainError>> {
        let credential_store = self.clone();
        Box::pin(async move {
            let credentials = credential_store.list_all().await?;
            Ok(credentials.into_iter().map(|(_, credential)| credential).collect())
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
            let path = inner.credential_file(&key.0, &key.1);
            if !path.exists() {
                return Err(BrainError::Storage(format!(
                    "credential not found: {}:{}",
                    key.0, key.1
                )));
            }
            write_json(&path, &credential).await?;
            set_file_permissions_restrictive(&path).await?;
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
            let path = inner.credential_file(&key.0, &key.1);
            match tokio::fs::remove_file(&path).await {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(BrainError::Storage(format!(
                        "delete credential {}: {error}",
                        path.display()
                    )));
                }
            }
            inner.publish_credential(CredentialStoreEvent::Deleted { key });
            Ok(())
        })
    }

    fn subscribe(&self) -> CredentialStoreEventStream {
        broadcast_stream(self.inner.credential_event_tx.subscribe())
    }
}

impl FileCredentialStore {
    async fn list_all(&self) -> Result<Vec<(String, CredentialEntry)>, BrainError> {
        let dir = self.inner.credentials_dir();
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut provider_dirs = tokio::fs::read_dir(&dir)
            .await
            .map_err(|e| BrainError::Storage(format!("read credentials dir: {e}")))?;

        let mut credentials = Vec::new();
        while let Some(provider_entry) = provider_dirs
            .next_entry()
            .await
            .map_err(|e| BrainError::Storage(format!("read dir entry: {e}")))?
        {
            let provider_path = provider_entry.path();
            if !provider_path.is_dir() {
                continue;
            }
            let provider_name = provider_path
                .file_name()
                .and_then(|segment| segment.to_str())
                .unwrap_or_default()
                .to_owned();

            let mut entries = match tokio::fs::read_dir(&provider_path).await {
                Ok(entries) => entries,
                Err(_) => continue,
            };
            while let Some(entry) = entries
                .next_entry()
                .await
                .map_err(|e| BrainError::Storage(format!("read dir entry: {e}")))?
            {
                let path = entry.path();
                if path.extension().and_then(|segment| segment.to_str()) != Some("json") {
                    continue;
                }
                match read_json::<CredentialEntry>(&path).await {
                    Ok(credential) => credentials.push((provider_name.clone(), credential)),
                    Err(_) => continue,
                }
            }
        }
        Ok(credentials)
    }
}

impl CredentialStore for FileCredentialStore {
    fn list_for_provider(
        &self,
        provider_name: &str,
    ) -> BoxFuture<'_, Result<Vec<CredentialEntry>, BrainError>> {
        let inner = Arc::clone(&self.inner);
        let provider_name = provider_name.to_owned();
        Box::pin(async move {
            let dir = inner.credential_provider_dir(&provider_name);
            if !dir.exists() {
                return Ok(Vec::new());
            }

            let mut entries = tokio::fs::read_dir(&dir)
                .await
                .map_err(|e| BrainError::Storage(format!("read credentials dir: {e}")))?;
            let mut credentials = Vec::new();
            while let Some(entry) = entries
                .next_entry()
                .await
                .map_err(|e| BrainError::Storage(format!("read dir entry: {e}")))?
            {
                let path = entry.path();
                if path.extension().and_then(|segment| segment.to_str()) != Some("json") {
                    continue;
                }
                match read_json::<CredentialEntry>(&path).await {
                    Ok(credential) => credentials.push(credential),
                    Err(_) => continue,
                }
            }
            Ok(credentials)
        })
    }

    fn update_health(
        &self,
        provider_name: &str,
        credential_id: &str,
        health: &CredentialHealth,
    ) -> BoxFuture<'_, Result<(), BrainError>> {
        let credential_store = self.clone();
        let key = (provider_name.to_owned(), credential_id.to_owned());
        let health = health.clone();
        Box::pin(async move {
            let mut credential = match credential_store.get(key.clone()).await {
                Ok(credential) => credential,
                Err(_) => return Ok(()),
            };
            credential.health = health;
            credential_store.update(key, credential).await?;
            Ok(())
        })
    }
}

impl Store for FileStore {
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

    fn subscribe(&self) -> StoreEventStream {
        broadcast_stream(self.inner.store_event_tx.subscribe())
    }
}

#[cfg(unix)]
async fn set_file_permissions_restrictive(path: &Path) -> Result<(), BrainError> {
    use std::os::unix::fs::PermissionsExt;

    let perms = std::fs::Permissions::from_mode(0o600);
    tokio::fs::set_permissions(path, perms)
        .await
        .map_err(|e| BrainError::Storage(format!("set permissions on {}: {e}", path.display())))
}

#[cfg(not(unix))]
async fn set_file_permissions_restrictive(_path: &Path) -> Result<(), BrainError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;

    use super::*;

    fn api_key_entry(id: &str, key: &str) -> CredentialEntry {
        CredentialEntry::api_key(id, key)
    }

    async fn temp_store() -> (FileStore, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let store = FileStore::new(dir.path()).await.unwrap();
        (store, dir)
    }

    #[tokio::test]
    async fn project_crud_uses_keyed_access() {
        let (store, _dir) = temp_store().await;
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
    async fn session_crud_and_project_listing_work() {
        let (store, _dir) = temp_store().await;
        let project = Project::with_defaults("test");
        store.projects().create(project.id, project.clone()).await.unwrap();

        let mut session = Session::new(project.id);
        let id = session.id;
        store.sessions().create(id, session.clone()).await.unwrap();
        assert_eq!(store.sessions().get(id).await.unwrap().id, id);
        assert_eq!(store.sessions().list_for_project(project.id).await.unwrap().len(), 1);

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
    async fn message_crud_uses_tuple_key() {
        let (store, _dir) = temp_store().await;
        let project = Project::with_defaults("project");
        store.projects().create(project.id, project.clone()).await.unwrap();
        let session = Session::new(project.id);
        store.sessions().create(session.id, session.clone()).await.unwrap();

        let first = Message::user("first");
        let second = Message::assistant("second");
        let first_key = (session.id, first.id);
        let second_key = (session.id, second.id);

        store.messages().create(first_key, first.clone()).await.unwrap();
        store.messages().create(second_key, second.clone()).await.unwrap();
        assert_eq!(store.messages().list_for_session(session.id).await.unwrap().len(), 2);
        assert_eq!(store.messages().get(first_key).await.unwrap().id, first.id);

        let updated = Message {
            content: "updated".into(),
            ..first.clone()
        };
        store.messages().update(first_key, updated.clone()).await.unwrap();
        assert_eq!(
            store.messages().get(first_key).await.unwrap().content,
            updated.content
        );

        store.messages().delete(second_key).await.unwrap();
        assert_eq!(store.messages().list_for_session(session.id).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn credential_crud_uses_tuple_key() {
        let (store, _dir) = temp_store().await;
        let key = ("openai".to_owned(), "key-1".to_owned());
        let entry = api_key_entry("key-1", "sk-123");

        store.credentials().create(key.clone(), entry.clone()).await.unwrap();
        assert_eq!(store.credentials().get(key.clone()).await.unwrap().id, "key-1");
        assert_eq!(store.credentials().list_for_provider("openai").await.unwrap().len(), 1);

        let mut updated = entry.clone();
        updated.health.record_error("test", None);
        store.credentials().update(key.clone(), updated.clone()).await.unwrap();
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
    async fn project_delete_cascades_session_events() {
        let (store, _dir) = temp_store().await;
        let project = Project::with_defaults("project");
        store.projects().create(project.id, project.clone()).await.unwrap();
        let session = Session::new(project.id);
        store.sessions().create(session.id, session.clone()).await.unwrap();

        let mut events = store.subscribe();
        store.projects().delete(project.id).await.unwrap();

        let first = events.next().await.unwrap();
        let second = events.next().await.unwrap();
        match (first, second) {
            (
                StoreEvent::SessionDeleted { session_id, project_id },
                StoreEvent::ProjectDeleted { project_id: deleted_project_id },
            ) => {
                assert_eq!(session_id, session.id);
                assert_eq!(project_id, project.id);
                assert_eq!(deleted_project_id, project.id);
            }
            other => panic!("unexpected events: {other:?}"),
        }
    }
}
