use std::path::{Path, PathBuf};
use std::sync::Arc;

use futures::future::BoxFuture;
use tokio::fs;

use crate::memory::{InMemoryStore, StoreData};
use crate::{
    CredentialEntry, CredentialHealth, CredentialStore, CredentialStoreEvent, CredentialStoreKey,
    MessageStore, MessageStoreEvent, Project, ProjectId, ProjectStore, ProjectStoreEvent, Session,
    SessionId, SessionStore, SessionStoreEvent, SessionUpdate, Store, StoreError, StoreEvent,
    StoreStream, StoredMessage, TrajectoryStore, TrajectoryStoreEvent,
};

/// Filesystem-backed store implementation using human-readable JSON files.
#[derive(Clone)]
pub struct FileStore {
    root: Arc<PathBuf>,
    inner: InMemoryStore,
}

impl FileStore {
    /// Opens or creates a file-backed store rooted at `root`.
    pub async fn new(root: impl Into<PathBuf>) -> Result<Self, StoreError> {
        let root = root.into();
        fs::create_dir_all(root.join("projects")).await?;
        fs::create_dir_all(root.join("sessions")).await?;
        fs::create_dir_all(root.join("messages")).await?;
        fs::create_dir_all(root.join("credentials")).await?;
        fs::create_dir_all(root.join("trajectories")).await?;

        let data = load_data(&root).await?;
        Ok(Self {
            root: Arc::new(root),
            inner: InMemoryStore::from_data(data),
        })
    }

    async fn persist_all(&self) -> Result<(), StoreError> {
        let snapshot = self.inner.snapshot().await;
        persist_data(self.root.as_path(), &snapshot).await
    }
}

async fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, StoreError> {
    let bytes = fs::read(path).await?;
    Ok(serde_json::from_slice(&bytes)?)
}

async fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), StoreError> {
    let bytes = serde_json::to_vec_pretty(value)?;
    fs::write(path, bytes).await?;
    Ok(())
}

async fn load_data(root: &Path) -> Result<StoreData, StoreError> {
    let mut data = StoreData::default();

    let mut projects = fs::read_dir(root.join("projects")).await?;
    while let Some(entry) = projects.next_entry().await? {
        let project: Project = read_json(&entry.path()).await?;
        data.projects.insert(project.id, project);
    }

    let mut sessions = fs::read_dir(root.join("sessions")).await?;
    while let Some(entry) = sessions.next_entry().await? {
        let session: Session = read_json(&entry.path()).await?;
        data.sessions.insert(session.id, session);
    }

    let mut messages = fs::read_dir(root.join("messages")).await?;
    while let Some(entry) = messages.next_entry().await? {
        let session_messages: Vec<StoredMessage> = read_json(&entry.path()).await?;
        if let Some(first) = session_messages.first() {
            data.messages.insert(first.session_id, session_messages);
        }
    }

    let mut providers = fs::read_dir(root.join("credentials")).await?;
    while let Some(provider_dir) = providers.next_entry().await? {
        if !provider_dir.file_type().await?.is_dir() {
            continue;
        }
        let provider_name = provider_dir.file_name().to_string_lossy().to_string();
        let mut creds = fs::read_dir(provider_dir.path()).await?;
        while let Some(entry) = creds.next_entry().await? {
            let credential: CredentialEntry = read_json(&entry.path()).await?;
            let credential_id = entry
                .path()
                .file_stem()
                .and_then(|stem| stem.to_str())
                .ok_or_else(|| StoreError::InvalidInput("invalid credential filename".into()))?
                .to_owned();
            data.credentials
                .insert((provider_name.clone(), credential_id), credential);
        }
    }

    let mut trajectories = fs::read_dir(root.join("trajectories")).await?;
    while let Some(entry) = trajectories.next_entry().await? {
        let trajectory: atif::Trajectory = read_json(&entry.path()).await?;
        let session_id = entry
            .path()
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| StoreError::InvalidInput("invalid trajectory filename".into()))?
            .parse::<SessionId>()
            .map_err(|_| StoreError::InvalidInput("invalid trajectory session id".into()))?;
        data.trajectories.insert(session_id, trajectory);
    }

    Ok(data)
}

async fn persist_data(root: &Path, data: &StoreData) -> Result<(), StoreError> {
    for dir in [
        root.join("projects"),
        root.join("sessions"),
        root.join("messages"),
        root.join("credentials"),
        root.join("trajectories"),
    ] {
        match fs::remove_dir_all(&dir).await {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(StoreError::Io(error)),
        }
    }

    fs::create_dir_all(root.join("projects")).await?;
    fs::create_dir_all(root.join("sessions")).await?;
    fs::create_dir_all(root.join("messages")).await?;
    fs::create_dir_all(root.join("credentials")).await?;
    fs::create_dir_all(root.join("trajectories")).await?;

    for project in data.projects.values() {
        write_json(
            &root.join("projects").join(format!("{}.json", project.id)),
            project,
        )
        .await?;
    }

    for session in data.sessions.values() {
        write_json(
            &root.join("sessions").join(format!("{}.json", session.id)),
            session,
        )
        .await?;
    }

    for (session_id, messages) in &data.messages {
        write_json(
            &root.join("messages").join(format!("{session_id}.json")),
            messages,
        )
        .await?;
    }

    for ((provider, credential_id), credential) in &data.credentials {
        let dir = root.join("credentials").join(provider);
        fs::create_dir_all(&dir).await?;
        write_json(&dir.join(format!("{credential_id}.json")), credential).await?;
    }

    for (session_id, trajectory) in &data.trajectories {
        write_json(
            &root.join("trajectories").join(format!("{session_id}.json")),
            trajectory,
        )
        .await?;
    }

    Ok(())
}

macro_rules! delegate_and_persist {
    ($self:expr, $expr:expr) => {{
        let result = $expr.await?;
        $self.persist_all().await?;
        Ok(result)
    }};
}

impl ProjectStore for FileStore {
    fn create(&self, project: Project) -> BoxFuture<'_, Result<Project, StoreError>> {
        Box::pin(async move { delegate_and_persist!(self, self.inner.projects().create(project)) })
    }

    fn get(&self, id: ProjectId) -> BoxFuture<'_, Result<Project, StoreError>> {
        self.inner.projects().get(id)
    }

    fn list(&self) -> BoxFuture<'_, Result<Vec<Project>, StoreError>> {
        self.inner.projects().list()
    }

    fn update(
        &self,
        id: ProjectId,
        project: Project,
    ) -> BoxFuture<'_, Result<Project, StoreError>> {
        Box::pin(
            async move { delegate_and_persist!(self, self.inner.projects().update(id, project)) },
        )
    }

    fn delete(&self, id: ProjectId) -> BoxFuture<'_, Result<(), StoreError>> {
        Box::pin(async move { delegate_and_persist!(self, self.inner.projects().delete(id)) })
    }

    fn find_by_root(&self, root: &Path) -> BoxFuture<'_, Result<Option<Project>, StoreError>> {
        self.inner.projects().find_by_root(root)
    }

    fn subscribe(&self) -> StoreStream<ProjectStoreEvent> {
        self.inner.projects().subscribe()
    }
}

impl SessionStore for FileStore {
    fn create(&self, session: Session) -> BoxFuture<'_, Result<Session, StoreError>> {
        Box::pin(async move { delegate_and_persist!(self, self.inner.sessions().create(session)) })
    }

    fn get(&self, id: SessionId) -> BoxFuture<'_, Result<Session, StoreError>> {
        self.inner.sessions().get(id)
    }

    fn list(&self) -> BoxFuture<'_, Result<Vec<Session>, StoreError>> {
        self.inner.sessions().list()
    }

    fn list_for_project(
        &self,
        project_id: ProjectId,
    ) -> BoxFuture<'_, Result<Vec<Session>, StoreError>> {
        self.inner.sessions().list_for_project(project_id)
    }

    fn update(
        &self,
        id: SessionId,
        session: Session,
    ) -> BoxFuture<'_, Result<Session, StoreError>> {
        Box::pin(
            async move { delegate_and_persist!(self, self.inner.sessions().update(id, session)) },
        )
    }

    fn patch(
        &self,
        id: SessionId,
        update: SessionUpdate,
    ) -> BoxFuture<'_, Result<Session, StoreError>> {
        Box::pin(
            async move { delegate_and_persist!(self, self.inner.sessions().patch(id, update)) },
        )
    }

    fn delete(&self, id: SessionId) -> BoxFuture<'_, Result<(), StoreError>> {
        Box::pin(async move { delegate_and_persist!(self, self.inner.sessions().delete(id)) })
    }

    fn subscribe(&self) -> StoreStream<SessionStoreEvent> {
        self.inner.sessions().subscribe()
    }
}

impl MessageStore for FileStore {
    fn list_for_session(
        &self,
        session_id: SessionId,
    ) -> BoxFuture<'_, Result<Vec<StoredMessage>, StoreError>> {
        self.inner.messages().list_for_session(session_id)
    }

    fn replace_for_session(
        &self,
        session_id: SessionId,
        messages: Vec<StoredMessage>,
    ) -> BoxFuture<'_, Result<Vec<StoredMessage>, StoreError>> {
        Box::pin(async move {
            delegate_and_persist!(
                self,
                self.inner
                    .messages()
                    .replace_for_session(session_id, messages)
            )
        })
    }

    fn delete_for_session(&self, session_id: SessionId) -> BoxFuture<'_, Result<(), StoreError>> {
        Box::pin(async move {
            delegate_and_persist!(self, self.inner.messages().delete_for_session(session_id))
        })
    }

    fn subscribe(&self) -> StoreStream<MessageStoreEvent> {
        self.inner.messages().subscribe()
    }
}

impl CredentialStore for FileStore {
    fn create(
        &self,
        key: CredentialStoreKey,
        credential: CredentialEntry,
    ) -> BoxFuture<'_, Result<CredentialEntry, StoreError>> {
        Box::pin(async move {
            delegate_and_persist!(self, self.inner.credentials().create(key, credential))
        })
    }

    fn get(&self, key: CredentialStoreKey) -> BoxFuture<'_, Result<CredentialEntry, StoreError>> {
        self.inner.credentials().get(key)
    }

    fn list(
        &self,
    ) -> BoxFuture<'_, Result<Vec<(CredentialStoreKey, CredentialEntry)>, StoreError>> {
        self.inner.credentials().list()
    }

    fn list_for_provider(
        &self,
        provider_name: &str,
    ) -> BoxFuture<'_, Result<Vec<(CredentialStoreKey, CredentialEntry)>, StoreError>> {
        self.inner.credentials().list_for_provider(provider_name)
    }

    fn update(
        &self,
        key: CredentialStoreKey,
        credential: CredentialEntry,
    ) -> BoxFuture<'_, Result<CredentialEntry, StoreError>> {
        Box::pin(async move {
            delegate_and_persist!(self, self.inner.credentials().update(key, credential))
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
            delegate_and_persist!(
                self,
                self.inner
                    .credentials()
                    .update_health(&provider_name, &credential_id, &health)
            )
        })
    }

    fn delete(&self, key: CredentialStoreKey) -> BoxFuture<'_, Result<(), StoreError>> {
        Box::pin(async move { delegate_and_persist!(self, self.inner.credentials().delete(key)) })
    }

    fn subscribe(&self) -> StoreStream<CredentialStoreEvent> {
        self.inner.credentials().subscribe()
    }
}

impl TrajectoryStore for FileStore {
    fn get_for_session(
        &self,
        session_id: SessionId,
    ) -> BoxFuture<'_, Result<Option<atif::Trajectory>, StoreError>> {
        self.inner.trajectories().get_for_session(session_id)
    }

    fn list(&self) -> BoxFuture<'_, Result<Vec<(SessionId, atif::Trajectory)>, StoreError>> {
        self.inner.trajectories().list()
    }

    fn upsert(
        &self,
        session_id: SessionId,
        trajectory: atif::Trajectory,
    ) -> BoxFuture<'_, Result<atif::Trajectory, StoreError>> {
        Box::pin(async move {
            delegate_and_persist!(
                self,
                self.inner.trajectories().upsert(session_id, trajectory)
            )
        })
    }

    fn delete(&self, session_id: SessionId) -> BoxFuture<'_, Result<(), StoreError>> {
        Box::pin(async move {
            delegate_and_persist!(self, self.inner.trajectories().delete(session_id))
        })
    }

    fn subscribe(&self) -> StoreStream<TrajectoryStoreEvent> {
        self.inner.trajectories().subscribe()
    }
}

impl Store for FileStore {
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
        Store::subscribe(&self.inner)
    }
}

#[cfg(test)]
mod tests {
    use provider::Message;
    use tempfile::TempDir;

    use super::*;
    use crate::{ProjectConfig, StoredMessage};

    #[tokio::test]
    async fn file_store_roundtrips_persisted_state() {
        let temp = TempDir::new().unwrap();
        let store = FileStore::new(temp.path()).await.unwrap();
        let project = store
            .projects()
            .create(Project::new(
                Some("demo".into()),
                None,
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
                    Message::assistant_text("hello"),
                )],
            )
            .await
            .unwrap();

        let reopened = FileStore::new(temp.path()).await.unwrap();
        let messages = reopened
            .messages()
            .list_for_session(session.id)
            .await
            .unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].message.plain_text_lossy(), "hello");
    }

    #[tokio::test]
    async fn file_store_removes_deleted_files_on_persist() {
        let temp = TempDir::new().unwrap();
        let store = FileStore::new(temp.path()).await.unwrap();
        let project = store
            .projects()
            .create(Project::new(
                Some("demo".into()),
                None,
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
                    Message::assistant_text("hello"),
                )],
            )
            .await
            .unwrap();

        store.sessions().delete(session.id).await.unwrap();

        let reopened = FileStore::new(temp.path()).await.unwrap();
        let messages = reopened
            .messages()
            .list_for_session(session.id)
            .await
            .unwrap();
        assert!(messages.is_empty());
    }
}
