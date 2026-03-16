use futures::future::BoxFuture;
use tokio::sync::broadcast;
use ulid::Ulid;

use brain_types::{BrainError, CredentialEntry, EventStream, Message, Project, ProjectId, ProjectUpdate, ProviderInfo, Session, SessionUpdate};

use crate::types::{ServerEvent, ServerStatus};

pub trait BrainApi: Send + Sync {
    // Project management
    fn create_project(&self, project: Project) -> BoxFuture<'_, Result<Project, BrainError>>;
    fn list_projects(&self) -> BoxFuture<'_, Result<Vec<Project>, BrainError>>;
    fn get_project(&self, id: ProjectId) -> BoxFuture<'_, Result<Project, BrainError>>;
    fn update_project(&self, id: ProjectId, update: ProjectUpdate) -> BoxFuture<'_, Result<(), BrainError>>;
    fn delete_project(&self, id: ProjectId) -> BoxFuture<'_, Result<(), BrainError>>;

    // Session management
    fn create_session(&self, project_id: ProjectId) -> BoxFuture<'_, Result<Session, BrainError>>;
    fn list_sessions(&self, project_id: ProjectId) -> BoxFuture<'_, Result<Vec<Session>, BrainError>>;
    fn get_session(&self, id: Ulid) -> BoxFuture<'_, Result<Session, BrainError>>;
    fn update_session(&self, id: Ulid, update: SessionUpdate) -> BoxFuture<'_, Result<(), BrainError>>;
    fn delete_session(&self, id: Ulid) -> BoxFuture<'_, Result<(), BrainError>>;

    // Message history
    fn list_messages(&self, session_id: Ulid) -> BoxFuture<'_, Result<Vec<Message>, BrainError>>;

    // Turn management
    fn send_message(&self, session_id: Ulid, content: &str) -> BoxFuture<'_, Result<(), BrainError>>;
    fn send_message_stream(&self, session_id: Ulid, content: &str) -> BoxFuture<'_, Result<EventStream, BrainError>>;
    fn cancel_turn(&self, session_id: Ulid) -> BoxFuture<'_, Result<(), BrainError>>;

    // Provider & credentials
    fn list_providers(&self) -> BoxFuture<'_, Result<Vec<ProviderInfo>, BrainError>>;
    fn list_credentials(&self) -> BoxFuture<'_, Result<Vec<(String, CredentialEntry)>, BrainError>>;
    fn get_credentials(&self, provider_name: &str) -> BoxFuture<'_, Result<Vec<CredentialEntry>, BrainError>>;
    fn save_credential(&self, provider_name: &str, entry: CredentialEntry) -> BoxFuture<'_, Result<(), BrainError>>;
    fn delete_credential(&self, provider_name: &str, credential_id: &str) -> BoxFuture<'_, Result<(), BrainError>>;

    // Events & status
    fn subscribe(&self) -> broadcast::Receiver<ServerEvent>;
    fn status(&self) -> BoxFuture<'_, Result<ServerStatus, BrainError>>;
}
