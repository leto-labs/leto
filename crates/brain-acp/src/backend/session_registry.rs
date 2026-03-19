use std::collections::HashMap;

use agent_client_protocol as acp;
use brain_core::{Brain, BrainError, Project, Session};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use ulid::Ulid;

use super::errors::map_brain_error;

#[derive(Default)]
pub struct SessionRegistry {
    active_turns: Mutex<HashMap<Ulid, CancellationToken>>,
}

impl SessionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn create_session(
        &self,
        brain: &Brain,
        project: &Project,
    ) -> Result<Session, acp::Error> {
        brain
            .create_session(project.id)
            .await
            .map_err(map_brain_error)
    }

    pub async fn load_session(
        &self,
        brain: &Brain,
        project: &Project,
        session_id: Ulid,
    ) -> Result<Session, acp::Error> {
        let session = brain
            .store
            .session_get(session_id)
            .await
            .map_err(map_brain_error)?;
        if session.project_id != project.id {
            return Err(acp::Error::invalid_params());
        }
        Ok(session)
    }

    pub async fn list_sessions_for_project(
        &self,
        brain: &Brain,
        project_id: brain_core::ProjectId,
    ) -> Result<Vec<Session>, acp::Error> {
        brain
            .list_sessions(project_id)
            .await
            .map_err(map_brain_error)
    }

    pub async fn list_all_sessions(
        &self,
        brain: &Brain,
    ) -> Result<Vec<(brain_core::Project, Session)>, acp::Error> {
        let projects = brain.store.project_list().await.map_err(map_brain_error)?;
        let mut sessions = Vec::new();
        for project in projects {
            for session in brain
                .list_sessions(project.id)
                .await
                .map_err(map_brain_error)?
            {
                sessions.push((project.clone(), session));
            }
        }
        Ok(sessions)
    }

    pub async fn start_turn(&self, session_id: Ulid) -> Result<CancellationToken, acp::Error> {
        let mut active_turns = self.active_turns.lock().await;
        if active_turns.contains_key(&session_id) {
            return Err(map_brain_error(BrainError::TurnActive(session_id)));
        }

        let cancel = CancellationToken::new();
        active_turns.insert(session_id, cancel.clone());
        Ok(cancel)
    }

    pub async fn finish_turn(&self, session_id: Ulid) {
        self.active_turns.lock().await.remove(&session_id);
    }

    pub async fn cancel_turn(&self, session_id: Ulid) {
        if let Some(cancel) = self.active_turns.lock().await.get(&session_id).cloned() {
            cancel.cancel();
        }
    }
}
