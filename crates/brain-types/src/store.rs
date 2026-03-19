use std::path::Path;

use futures::future::BoxFuture;
use ulid::Ulid;

use crate::{
    BrainError, CredentialEntry, CredentialHealth, Message, Project, ProjectId, ProjectUpdate,
    Session, SessionUpdate,
};

pub trait ProjectStore: Send + Sync {
    fn project_create(&self, project: Project) -> BoxFuture<'_, Result<Project, BrainError>>;
    fn project_get(&self, id: ProjectId) -> BoxFuture<'_, Result<Project, BrainError>>;
    fn project_list(&self) -> BoxFuture<'_, Result<Vec<Project>, BrainError>>;
    fn project_find_by_root(
        &self,
        root: &Path,
    ) -> BoxFuture<'_, Result<Option<Project>, BrainError>>;
    fn project_update(
        &self,
        id: ProjectId,
        update: ProjectUpdate,
    ) -> BoxFuture<'_, Result<(), BrainError>>;
    fn project_delete(&self, id: ProjectId) -> BoxFuture<'_, Result<(), BrainError>>;
}

pub trait SessionStore: Send + Sync {
    fn session_create(&self, project_id: ProjectId) -> BoxFuture<'_, Result<Session, BrainError>>;
    fn session_get(&self, id: Ulid) -> BoxFuture<'_, Result<Session, BrainError>>;
    fn session_list(
        &self,
        project_id: ProjectId,
    ) -> BoxFuture<'_, Result<Vec<Session>, BrainError>>;
    fn session_update(
        &self,
        id: Ulid,
        update: SessionUpdate,
    ) -> BoxFuture<'_, Result<(), BrainError>>;
    fn session_delete(&self, id: Ulid) -> BoxFuture<'_, Result<(), BrainError>>;
}

pub trait MessageStore: Send + Sync {
    fn message_append(
        &self,
        session_id: Ulid,
        msgs: &[Message],
    ) -> BoxFuture<'_, Result<(), BrainError>>;
    fn message_list(&self, session_id: Ulid) -> BoxFuture<'_, Result<Vec<Message>, BrainError>>;
}

pub trait CredentialStore: Send + Sync {
    fn credential_save(
        &self,
        provider_name: &str,
        entry: &CredentialEntry,
    ) -> BoxFuture<'_, Result<(), BrainError>>;
    fn credential_load(
        &self,
        provider_name: &str,
        credential_id: &str,
    ) -> BoxFuture<'_, Result<Option<CredentialEntry>, BrainError>>;
    fn credential_load_all(
        &self,
        provider_name: &str,
    ) -> BoxFuture<'_, Result<Vec<CredentialEntry>, BrainError>>;
    fn credential_delete(
        &self,
        provider_name: &str,
        credential_id: &str,
    ) -> BoxFuture<'_, Result<(), BrainError>>;
    fn credential_update_health(
        &self,
        provider_name: &str,
        credential_id: &str,
        health: &CredentialHealth,
    ) -> BoxFuture<'_, Result<(), BrainError>>;
    fn credential_list(&self) -> BoxFuture<'_, Result<Vec<(String, CredentialEntry)>, BrainError>>;
}

pub trait Store: ProjectStore + SessionStore + MessageStore + CredentialStore {}

impl<T: ProjectStore + SessionStore + MessageStore + CredentialStore> Store for T {}
