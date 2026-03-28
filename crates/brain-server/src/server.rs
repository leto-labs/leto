use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use futures::StreamExt;
use futures::future::BoxFuture;
use tokio::sync::{RwLock, broadcast};
use tokio_util::sync::CancellationToken;
use ulid::Ulid;

use brain_core::{Brain, OpenAiConfigPreset};
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
        Box::pin(async move {
            self.inner
                .brain
                .store
                .projects()
                .create(project.id, project)
                .await
        })
    }

    fn list_projects(&self) -> BoxFuture<'_, Result<Vec<Project>, BrainError>> {
        Box::pin(async move { self.inner.brain.store.projects().list().await })
    }

    fn get_project(&self, id: ProjectId) -> BoxFuture<'_, Result<Project, BrainError>> {
        Box::pin(async move { self.inner.brain.store.projects().get(id).await })
    }

    fn update_project(
        &self,
        id: ProjectId,
        update: ProjectUpdate,
    ) -> BoxFuture<'_, Result<(), BrainError>> {
        Box::pin(async move {
            let mut project = self.inner.brain.store.projects().get(id).await?;
            if let Some(name) = update.name {
                project.name = Some(name);
            }
            if let Some(config) = update.config {
                project.config = config;
            }
            project.updated_at = Utc::now();
            self.inner
                .brain
                .store
                .projects()
                .update(id, project)
                .await?;
            Ok(())
        })
    }

    fn delete_project(&self, id: ProjectId) -> BoxFuture<'_, Result<(), BrainError>> {
        Box::pin(async move { self.inner.brain.store.projects().delete(id).await })
    }

    // -- Session management --

    fn create_session(&self, project_id: ProjectId) -> BoxFuture<'_, Result<Session, BrainError>> {
        Box::pin(async move {
            let session = Session::new(project_id);
            self.inner
                .brain
                .store
                .sessions()
                .create(session.id, session)
                .await
        })
    }

    fn list_sessions(
        &self,
        project_id: ProjectId,
    ) -> BoxFuture<'_, Result<Vec<Session>, BrainError>> {
        Box::pin(async move {
            self.inner
                .brain
                .store
                .sessions()
                .list_for_project(project_id)
                .await
        })
    }

    fn get_session(&self, id: Ulid) -> BoxFuture<'_, Result<Session, BrainError>> {
        Box::pin(async move { self.inner.brain.store.sessions().get(id).await })
    }

    fn update_session(
        &self,
        id: Ulid,
        update: SessionUpdate,
    ) -> BoxFuture<'_, Result<(), BrainError>> {
        Box::pin(async move {
            let mut session = self.inner.brain.store.sessions().get(id).await?;
            if let Some(title) = update.title {
                session.title = Some(title);
            }
            if let Some(inference) = update.inference {
                match inference {
                    SessionInferenceUpdate::Set(config) => session.inference = Some(config),
                    SessionInferenceUpdate::Clear => session.inference = None,
                }
            }
            if let Some(loop_name) = update.loop_name {
                match loop_name {
                    SessionLoopUpdate::Set(loop_name) => session.loop_name = Some(loop_name),
                    SessionLoopUpdate::Clear => session.loop_name = None,
                }
            }
            session.updated_at = Utc::now();
            self.inner
                .brain
                .store
                .sessions()
                .update(id, session)
                .await?;
            Ok(())
        })
    }

    fn delete_session(&self, id: Ulid) -> BoxFuture<'_, Result<(), BrainError>> {
        Box::pin(async move { self.inner.brain.store.sessions().delete(id).await })
    }

    // -- Message history --

    fn list_messages(&self, session_id: Ulid) -> BoxFuture<'_, Result<Vec<Message>, BrainError>> {
        Box::pin(async move {
            self.inner
                .brain
                .store
                .messages()
                .list_for_session(session_id)
                .await
        })
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

            let stream = inner.brain.turn(session_id, &content, cancel);

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

            let stream = inner.brain.turn(session_id, &content, cancel);

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
        Box::pin(async move {
            let mut credentials = Vec::new();
            for preset in OpenAiConfigPreset::ALL {
                let provider_name = preset.name.to_owned();
                for entry in self
                    .inner
                    .brain
                    .store
                    .credentials()
                    .list_for_provider(preset.name)
                    .await?
                {
                    credentials.push((provider_name.clone(), entry));
                }
            }
            Ok(credentials)
        })
    }

    fn get_credentials(
        &self,
        provider_name: &str,
    ) -> BoxFuture<'_, Result<Vec<CredentialEntry>, BrainError>> {
        let name = provider_name.to_owned();
        Box::pin(async move {
            self.inner
                .brain
                .store
                .credentials()
                .list_for_provider(&name)
                .await
        })
    }

    fn save_credential(
        &self,
        provider_name: &str,
        entry: CredentialEntry,
    ) -> BoxFuture<'_, Result<(), BrainError>> {
        let name = provider_name.to_owned();
        Box::pin(async move {
            let key = (name, entry.id.clone());
            match self.inner.brain.store.credentials().get(key.clone()).await {
                Ok(_) => {
                    self.inner
                        .brain
                        .store
                        .credentials()
                        .update(key, entry)
                        .await?;
                }
                Err(_) => {
                    self.inner
                        .brain
                        .store
                        .credentials()
                        .create(key, entry)
                        .await?;
                }
            }
            Ok(())
        })
    }

    fn delete_credential(
        &self,
        provider_name: &str,
        credential_id: &str,
    ) -> BoxFuture<'_, Result<(), BrainError>> {
        let name = provider_name.to_owned();
        let cid = credential_id.to_owned();
        Box::pin(async move {
            self.inner
                .brain
                .store
                .credentials()
                .delete((name, cid))
                .await
        })
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
            let projects = inner.brain.store.projects().list().await?;

            let mut total_sessions = 0usize;
            for p in &projects {
                let sessions = inner.brain.store.sessions().list_for_project(p.id).await?;
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
mod tests;
