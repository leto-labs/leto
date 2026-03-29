use chrono::Utc;
use futures::future::BoxFuture;
use ulid::Ulid;

use brain_types::*;

use super::BrainServer;

impl BrainServer {
    pub(super) fn create_project_api(
        &self,
        project: Project,
    ) -> BoxFuture<'_, Result<Project, BrainError>> {
        Box::pin(async move {
            self.inner
                .brain
                .store
                .projects()
                .create(project.id, project)
                .await
        })
    }

    pub(super) fn list_projects_api(&self) -> BoxFuture<'_, Result<Vec<Project>, BrainError>> {
        Box::pin(async move { self.inner.brain.store.projects().list().await })
    }

    pub(super) fn get_project_api(
        &self,
        id: ProjectId,
    ) -> BoxFuture<'_, Result<Project, BrainError>> {
        Box::pin(async move { self.inner.brain.store.projects().get(id).await })
    }

    pub(super) fn update_project_api(
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

    pub(super) fn delete_project_api(
        &self,
        id: ProjectId,
    ) -> BoxFuture<'_, Result<(), BrainError>> {
        Box::pin(async move { self.inner.brain.store.projects().delete(id).await })
    }

    pub(super) fn create_session_api(
        &self,
        project_id: ProjectId,
    ) -> BoxFuture<'_, Result<Session, BrainError>> {
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

    pub(super) fn list_sessions_api(
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

    pub(super) fn get_session_api(&self, id: Ulid) -> BoxFuture<'_, Result<Session, BrainError>> {
        Box::pin(async move { self.inner.brain.store.sessions().get(id).await })
    }

    pub(super) fn update_session_api(
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

    pub(super) fn delete_session_api(&self, id: Ulid) -> BoxFuture<'_, Result<(), BrainError>> {
        Box::pin(async move { self.inner.brain.store.sessions().delete(id).await })
    }

    pub(super) fn list_messages_api(
        &self,
        session_id: Ulid,
    ) -> BoxFuture<'_, Result<Vec<Message>, BrainError>> {
        Box::pin(async move {
            self.inner
                .brain
                .store
                .messages()
                .list_for_session(session_id)
                .await
        })
    }
}
