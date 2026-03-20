use std::sync::Arc;

use agent_client_protocol as acp;
use futures::StreamExt;
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;

use super::app::BackendApp;
#[cfg(feature = "unstable_session_model")]
use super::capabilities::session_model_state;
use super::capabilities::{initialize_response, list_info, session_info_update};
use super::errors::{internal_error, map_brain_error};
use super::event_mapper::{EventMapper, MappedEvent};
use super::history_replay::replay_updates;
use super::ids::{parse_session_id, session_id_from_ulid};

pub(super) type NotificationEnvelope = (acp::SessionNotification, oneshot::Sender<()>);

pub(super) struct BackendAgent {
    app: Arc<BackendApp>,
    session_update_tx: mpsc::UnboundedSender<NotificationEnvelope>,
}

impl BackendAgent {
    pub(super) fn new(
        app: Arc<BackendApp>,
        session_update_tx: mpsc::UnboundedSender<NotificationEnvelope>,
    ) -> Self {
        Self {
            app,
            session_update_tx,
        }
    }

    async fn emit(
        &self,
        session_id: &acp::SessionId,
        update: acp::SessionUpdate,
    ) -> Result<(), acp::Error> {
        let (ack_tx, ack_rx) = oneshot::channel();
        self.session_update_tx
            .send((
                acp::SessionNotification::new(session_id.clone(), update),
                ack_tx,
            ))
            .map_err(|_| acp::Error::internal_error())?;
        ack_rx.await.map_err(|_| acp::Error::internal_error())
    }

    fn prompt_text(blocks: &[acp::ContentBlock]) -> String {
        let text = blocks
            .iter()
            .map(|block| match block {
                acp::ContentBlock::Text(content) => content.text.clone(),
                acp::ContentBlock::ResourceLink(link) => format!("[resource:{}]", link.uri),
                acp::ContentBlock::Resource(_) => "[embedded-resource]".to_owned(),
                acp::ContentBlock::Image(_) => "[image]".to_owned(),
                acp::ContentBlock::Audio(_) => "[audio]".to_owned(),
                _ => "[unsupported-content]".to_owned(),
            })
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_owned();

        if text.is_empty() {
            "(empty prompt)".to_owned()
        } else {
            text
        }
    }

    fn session_title_from_prompt(prompt_text: &str) -> String {
        let trimmed = prompt_text.trim();
        if trimmed.is_empty() {
            return "ACP Session".to_owned();
        }

        if trimmed.len() > 48 {
            format!("{}...", &trimmed[..48])
        } else {
            trimmed.to_owned()
        }
    }

    async fn resolve_project(
        &self,
        cwd: &std::path::Path,
    ) -> Result<brain_core::Project, acp::Error> {
        self.app
            .runtime
            .resolve_or_create_project(cwd.to_path_buf())
            .await
            .map_err(map_brain_error)
    }

    async fn load_project_session(
        &self,
        project_id: brain_core::ProjectId,
        session_id: ulid::Ulid,
    ) -> Result<brain_core::Session, acp::Error> {
        let session = self
            .app
            .runtime
            .store()
            .sessions()
            .get(session_id)
            .await
            .map_err(map_brain_error)?;
        if session.project_id != project_id {
            return Err(acp::Error::invalid_params());
        }
        Ok(session)
    }

    #[cfg(feature = "unstable_session_model")]
    async fn current_model_state_for_session(
        &self,
        session_id: ulid::Ulid,
    ) -> Result<acp::SessionModelState, acp::Error> {
        let current_model = self
            .app
            .runtime
            .current_model_id_for_session(session_id)
            .await
            .map_err(map_brain_error)?
            .ok_or_else(acp::Error::invalid_params)?;
        let models = self.app.runtime.list_models().map_err(map_brain_error)?;
        Ok(session_model_state(current_model, &models))
    }
}

#[async_trait::async_trait(?Send)]
impl acp::Agent for BackendAgent {
    async fn initialize(
        &self,
        arguments: acp::InitializeRequest,
    ) -> Result<acp::InitializeResponse, acp::Error> {
        Ok(initialize_response(arguments.protocol_version))
    }

    async fn authenticate(
        &self,
        _arguments: acp::AuthenticateRequest,
    ) -> Result<acp::AuthenticateResponse, acp::Error> {
        Ok(acp::AuthenticateResponse::new())
    }

    async fn new_session(
        &self,
        arguments: acp::NewSessionRequest,
    ) -> Result<acp::NewSessionResponse, acp::Error> {
        let project = self.resolve_project(&arguments.cwd).await?;
        let session = brain_core::Session::new(project.id);
        let session = self
            .app
            .runtime
            .store()
            .sessions()
            .create(session.id, session)
            .await
            .map_err(map_brain_error)?;
        let session_id = session_id_from_ulid(session.id);

        self.emit(
            &session_id,
            acp::SessionUpdate::SessionInfoUpdate(session_info_update(&session)),
        )
        .await?;

        let response = acp::NewSessionResponse::new(session_id.clone());
        #[cfg(feature = "unstable_session_model")]
        let response = response.models(self.current_model_state_for_session(session.id).await?);

        Ok(response)
    }

    async fn load_session(
        &self,
        arguments: acp::LoadSessionRequest,
    ) -> Result<acp::LoadSessionResponse, acp::Error> {
        let session_id = parse_session_id(&arguments.session_id)?;
        let project = self.resolve_project(&arguments.cwd).await?;
        let session = self.load_project_session(project.id, session_id).await?;
        let messages = self
            .app
            .runtime
            .store()
            .messages()
            .list_for_session(session.id)
            .await
            .map_err(map_brain_error)?;

        let acp_session_id = session_id_from_ulid(session.id);
        for update in replay_updates(&session, &messages) {
            self.emit(&acp_session_id, update).await?;
        }

        let response = acp::LoadSessionResponse::new();
        #[cfg(feature = "unstable_session_model")]
        let response = response.models(self.current_model_state_for_session(session.id).await?);

        Ok(response)
    }

    async fn list_sessions(
        &self,
        arguments: acp::ListSessionsRequest,
    ) -> Result<acp::ListSessionsResponse, acp::Error> {
        let mut items = Vec::new();

        if let Some(cwd) = arguments.cwd {
            let project = self.resolve_project(&cwd).await?;
            let cwd = project
                .root
                .clone()
                .ok_or_else(acp::Error::internal_error)?;
            for session in self
                .app
                .runtime
                .store()
                .sessions()
                .list_for_project(project.id)
                .await
                .map_err(map_brain_error)?
            {
                items.push(list_info(&session, cwd.clone()));
            }
        } else {
            let projects = self
                .app
                .runtime
                .store()
                .projects()
                .list()
                .await
                .map_err(map_brain_error)?;
            for project in projects {
                let Some(cwd) = project.root.clone() else {
                    continue;
                };
                for session in self
                    .app
                    .runtime
                    .store()
                    .sessions()
                    .list_for_project(project.id)
                    .await
                    .map_err(map_brain_error)?
                {
                    items.push(list_info(&session, cwd.clone()));
                }
            }
        }

        Ok(acp::ListSessionsResponse::new(items))
    }

    async fn prompt(
        &self,
        arguments: acp::PromptRequest,
    ) -> Result<acp::PromptResponse, acp::Error> {
        let session_id = parse_session_id(&arguments.session_id)?;
        let stop_reason = async {
            let session = self
                .app
                .runtime
                .store()
                .sessions()
                .get(session_id)
                .await
                .map_err(map_brain_error)?;
            let prompt_text = Self::prompt_text(&arguments.prompt);

            if session.title.is_none() {
                let title = Self::session_title_from_prompt(&prompt_text);
                let mut updated_session = session.clone();
                updated_session.title = Some(title.clone());
                self.app
                    .runtime
                    .store()
                    .sessions()
                    .update(updated_session.id, updated_session.clone())
                    .await
                    .map_err(map_brain_error)?;
                self.emit(
                    &arguments.session_id,
                    acp::SessionUpdate::SessionInfoUpdate(session_info_update(&updated_session)),
                )
                .await?;
            }

            let mut stream =
                self.app
                    .runtime
                    .turn(session.id, &prompt_text, CancellationToken::new());
            let mut mapper = EventMapper::new();

            while let Some(event) = stream.next().await {
                match mapper.map(event) {
                    MappedEvent::Updates(updates) => {
                        for update in updates {
                            self.emit(&arguments.session_id, update).await?;
                        }
                    }
                    MappedEvent::Cancelled => return Ok(acp::StopReason::Cancelled),
                    MappedEvent::Failed(message) => return Err(internal_error(message)),
                    MappedEvent::TurnComplete(stop_reason) => return Ok(stop_reason),
                }
            }

            Ok(acp::StopReason::EndTurn)
        }
        .await;

        stop_reason.map(acp::PromptResponse::new)
    }

    async fn cancel(&self, arguments: acp::CancelNotification) -> Result<(), acp::Error> {
        let session_id = parse_session_id(&arguments.session_id)?;
        self.app
            .runtime
            .cancel_turn(session_id)
            .await
            .map_err(map_brain_error)
    }

    #[cfg(feature = "unstable_session_model")]
    async fn set_session_model(
        &self,
        arguments: acp::SetSessionModelRequest,
    ) -> Result<acp::SetSessionModelResponse, acp::Error> {
        let session_id = parse_session_id(&arguments.session_id)?;
        self.app
            .runtime
            .set_session_model(session_id, arguments.model_id.0.as_ref())
            .await
            .map_err(map_brain_error)?;

        Ok(acp::SetSessionModelResponse::new())
    }
}
