use std::path::Path;
use std::pin::Pin;

use crate::{
    CredentialEntry, CredentialHealth, CredentialStoreKey, Project, ProjectId, Session, SessionId,
    SessionUpdate, StoredMessage,
};
use futures::{Stream, future::BoxFuture};
use serde::{Deserialize, Serialize};

/// Shared event-stream type used by store traits.
pub type StoreStream<T> = Pin<Box<dyn Stream<Item = T> + Send>>;

/// Aggregate store event emitted by concrete store implementations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StoreEvent {
    Project(ProjectStoreEvent),
    Session(SessionStoreEvent),
    Message(MessageStoreEvent),
    Credential(CredentialStoreEvent),
    Trajectory(TrajectoryStoreEvent),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProjectStoreEvent {
    Created { project: Project },
    Updated { project: Project },
    Deleted { project_id: ProjectId },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionStoreEvent {
    Created {
        session: Session,
    },
    Updated {
        session: Session,
    },
    Deleted {
        session_id: SessionId,
        project_id: ProjectId,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessageStoreEvent {
    Replaced {
        session_id: SessionId,
        message_count: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CredentialStoreEvent {
    Created {
        key: CredentialStoreKey,
        credential: CredentialEntry,
    },
    Updated {
        key: CredentialStoreKey,
        credential: CredentialEntry,
    },
    Deleted {
        key: CredentialStoreKey,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TrajectoryStoreEvent {
    Created {
        session_id: SessionId,
        trajectory: atif::Trajectory,
    },
    Updated {
        session_id: SessionId,
        trajectory: atif::Trajectory,
    },
    Deleted {
        session_id: SessionId,
    },
}

/// Store-level error.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("record already exists: {0}")]
    AlreadyExists(String),
    #[error("record not found: {0}")]
    NotFound(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

/// Store for project records.
pub trait ProjectStore: Send + Sync {
    fn create(&self, project: Project) -> BoxFuture<'_, Result<Project, StoreError>>;
    fn get(&self, id: ProjectId) -> BoxFuture<'_, Result<Project, StoreError>>;
    fn list(&self) -> BoxFuture<'_, Result<Vec<Project>, StoreError>>;
    fn update(&self, id: ProjectId, project: Project)
    -> BoxFuture<'_, Result<Project, StoreError>>;
    fn delete(&self, id: ProjectId) -> BoxFuture<'_, Result<(), StoreError>>;
    fn find_by_root(&self, root: &Path) -> BoxFuture<'_, Result<Option<Project>, StoreError>>;
    fn subscribe(&self) -> StoreStream<ProjectStoreEvent>;
}

/// Store for session records.
pub trait SessionStore: Send + Sync {
    fn create(&self, session: Session) -> BoxFuture<'_, Result<Session, StoreError>>;
    fn get(&self, id: SessionId) -> BoxFuture<'_, Result<Session, StoreError>>;
    fn list(&self) -> BoxFuture<'_, Result<Vec<Session>, StoreError>>;
    fn list_for_project(
        &self,
        project_id: ProjectId,
    ) -> BoxFuture<'_, Result<Vec<Session>, StoreError>>;
    fn update(&self, id: SessionId, session: Session)
    -> BoxFuture<'_, Result<Session, StoreError>>;
    fn patch(
        &self,
        id: SessionId,
        update: SessionUpdate,
    ) -> BoxFuture<'_, Result<Session, StoreError>>;
    fn delete(&self, id: SessionId) -> BoxFuture<'_, Result<(), StoreError>>;
    fn subscribe(&self) -> StoreStream<SessionStoreEvent>;
}

/// Store for persisted transcript messages.
pub trait MessageStore: Send + Sync {
    fn list_for_session(
        &self,
        session_id: SessionId,
    ) -> BoxFuture<'_, Result<Vec<StoredMessage>, StoreError>>;
    fn replace_for_session(
        &self,
        session_id: SessionId,
        messages: Vec<StoredMessage>,
    ) -> BoxFuture<'_, Result<Vec<StoredMessage>, StoreError>>;
    fn delete_for_session(&self, session_id: SessionId) -> BoxFuture<'_, Result<(), StoreError>>;
    fn subscribe(&self) -> StoreStream<MessageStoreEvent>;
}

/// Store for provider credentials.
pub trait CredentialStore: Send + Sync {
    fn create(
        &self,
        key: CredentialStoreKey,
        credential: CredentialEntry,
    ) -> BoxFuture<'_, Result<CredentialEntry, StoreError>>;
    fn get(&self, key: CredentialStoreKey) -> BoxFuture<'_, Result<CredentialEntry, StoreError>>;
    fn list(&self)
    -> BoxFuture<'_, Result<Vec<(CredentialStoreKey, CredentialEntry)>, StoreError>>;
    fn list_for_provider(
        &self,
        provider_name: &str,
    ) -> BoxFuture<'_, Result<Vec<(CredentialStoreKey, CredentialEntry)>, StoreError>>;
    fn update(
        &self,
        key: CredentialStoreKey,
        credential: CredentialEntry,
    ) -> BoxFuture<'_, Result<CredentialEntry, StoreError>>;
    fn update_health(
        &self,
        provider_name: &str,
        credential_id: &str,
        health: &CredentialHealth,
    ) -> BoxFuture<'_, Result<(), StoreError>>;
    fn delete(&self, key: CredentialStoreKey) -> BoxFuture<'_, Result<(), StoreError>>;
    fn subscribe(&self) -> StoreStream<CredentialStoreEvent>;
}

/// Store for persisted ATIF trajectories.
pub trait TrajectoryStore: Send + Sync {
    fn get_for_session(
        &self,
        session_id: SessionId,
    ) -> BoxFuture<'_, Result<Option<atif::Trajectory>, StoreError>>;
    fn list(&self) -> BoxFuture<'_, Result<Vec<(SessionId, atif::Trajectory)>, StoreError>>;
    fn upsert(
        &self,
        session_id: SessionId,
        trajectory: atif::Trajectory,
    ) -> BoxFuture<'_, Result<atif::Trajectory, StoreError>>;
    fn delete(&self, session_id: SessionId) -> BoxFuture<'_, Result<(), StoreError>>;
    fn subscribe(&self) -> StoreStream<TrajectoryStoreEvent>;
}

/// Composed store surface used by `agent-core`.
pub trait Store: Send + Sync {
    fn projects(&self) -> &dyn ProjectStore;
    fn sessions(&self) -> &dyn SessionStore;
    fn messages(&self) -> &dyn MessageStore;
    fn credentials(&self) -> &dyn CredentialStore;
    fn trajectories(&self) -> &dyn TrajectoryStore;
    fn subscribe(&self) -> StoreStream<StoreEvent>;
}
