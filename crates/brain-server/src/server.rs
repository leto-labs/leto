use std::collections::HashMap;
use std::sync::Arc;

use futures::future::BoxFuture;
use tokio::sync::{RwLock, broadcast};
use tokio_util::sync::CancellationToken;
use ulid::Ulid;

use brain_core::Brain;
use brain_types::*;

use crate::api::BrainApi;
use crate::event_bus::EventBus;
use crate::types::{ServerEvent, ServerStatus};

mod credentials_status;
mod projects_sessions;
mod turns;

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
    fn create_project(&self, project: Project) -> BoxFuture<'_, Result<Project, BrainError>> {
        self.create_project_api(project)
    }

    fn list_projects(&self) -> BoxFuture<'_, Result<Vec<Project>, BrainError>> {
        self.list_projects_api()
    }

    fn get_project(&self, id: ProjectId) -> BoxFuture<'_, Result<Project, BrainError>> {
        self.get_project_api(id)
    }

    fn update_project(
        &self,
        id: ProjectId,
        update: ProjectUpdate,
    ) -> BoxFuture<'_, Result<(), BrainError>> {
        self.update_project_api(id, update)
    }

    fn delete_project(&self, id: ProjectId) -> BoxFuture<'_, Result<(), BrainError>> {
        self.delete_project_api(id)
    }

    fn create_session(&self, project_id: ProjectId) -> BoxFuture<'_, Result<Session, BrainError>> {
        self.create_session_api(project_id)
    }

    fn list_sessions(
        &self,
        project_id: ProjectId,
    ) -> BoxFuture<'_, Result<Vec<Session>, BrainError>> {
        self.list_sessions_api(project_id)
    }

    fn get_session(&self, id: Ulid) -> BoxFuture<'_, Result<Session, BrainError>> {
        self.get_session_api(id)
    }

    fn update_session(
        &self,
        id: Ulid,
        update: SessionUpdate,
    ) -> BoxFuture<'_, Result<(), BrainError>> {
        self.update_session_api(id, update)
    }

    fn delete_session(&self, id: Ulid) -> BoxFuture<'_, Result<(), BrainError>> {
        self.delete_session_api(id)
    }

    fn list_messages(&self, session_id: Ulid) -> BoxFuture<'_, Result<Vec<Message>, BrainError>> {
        self.list_messages_api(session_id)
    }

    fn send_message(
        &self,
        session_id: Ulid,
        content: &str,
    ) -> BoxFuture<'_, Result<(), BrainError>> {
        self.send_message_api(session_id, content)
    }

    fn send_message_stream(
        &self,
        session_id: Ulid,
        content: &str,
    ) -> BoxFuture<'_, Result<EventStream, BrainError>> {
        self.send_message_stream_api(session_id, content)
    }

    fn cancel_turn(&self, session_id: Ulid) -> BoxFuture<'_, Result<(), BrainError>> {
        self.cancel_turn_api(session_id)
    }

    fn list_providers(&self) -> BoxFuture<'_, Result<Vec<ProviderInfo>, BrainError>> {
        self.list_providers_api()
    }

    fn list_credentials(
        &self,
    ) -> BoxFuture<'_, Result<Vec<(String, CredentialEntry)>, BrainError>> {
        self.list_credentials_api()
    }

    fn get_credentials(
        &self,
        provider_name: &str,
    ) -> BoxFuture<'_, Result<Vec<CredentialEntry>, BrainError>> {
        self.get_credentials_api(provider_name)
    }

    fn save_credential(
        &self,
        provider_name: &str,
        entry: CredentialEntry,
    ) -> BoxFuture<'_, Result<(), BrainError>> {
        self.save_credential_api(provider_name, entry)
    }

    fn delete_credential(
        &self,
        provider_name: &str,
        credential_id: &str,
    ) -> BoxFuture<'_, Result<(), BrainError>> {
        self.delete_credential_api(provider_name, credential_id)
    }

    fn subscribe(&self) -> broadcast::Receiver<ServerEvent> {
        self.inner.event_bus.subscribe()
    }

    fn status(&self) -> BoxFuture<'_, Result<ServerStatus, BrainError>> {
        self.status_api()
    }
}

#[cfg(test)]
mod tests;
