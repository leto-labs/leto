use std::sync::Arc;

use agent_client_protocol as acp;
use futures::StreamExt;
use tokio::sync::{mpsc, oneshot};

use super::app::BackendApp;
#[cfg(feature = "unstable_session_model")]
use super::capabilities::session_model_state;
use super::capabilities::{initialize_response, list_info, session_info_update};
use super::errors::{internal_error, map_brain_error};
use super::event_mapper::{EventMapper, MappedEvent};
use super::history_replay::replay_updates;
use super::ids::{parse_session_id, session_id_from_ulid};
use super::project_resolver::resolve_project;
use super::session_registry::SessionRegistry;

pub(super) type NotificationEnvelope = (acp::SessionNotification, oneshot::Sender<()>);

pub(super) struct BackendAgent {
    app: Arc<BackendApp>,
    sessions: SessionRegistry,
    session_update_tx: mpsc::UnboundedSender<NotificationEnvelope>,
}

impl BackendAgent {
    pub(super) fn new(
        app: Arc<BackendApp>,
        session_update_tx: mpsc::UnboundedSender<NotificationEnvelope>,
    ) -> Self {
        Self {
            app,
            sessions: SessionRegistry::new(),
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

    #[cfg(feature = "unstable_session_model")]
    async fn current_model_state_for_session(
        &self,
        session_id: ulid::Ulid,
    ) -> Result<acp::SessionModelState, acp::Error> {
        let effective_inference = self
            .app
            .brain
            .effective_inference_for_session(session_id)
            .await
            .map_err(map_brain_error)?;
        let current_model = self
            .app
            .router
            .current_model_id(&effective_inference)
            .map_err(|_| acp::Error::invalid_params())?
            .ok_or_else(acp::Error::invalid_params)?;
        Ok(session_model_state(
            current_model,
            &self.app.router.available_models(),
        ))
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
        let project = resolve_project(&self.app.brain, &arguments.cwd).await?;
        let session = self
            .sessions
            .create_session(&self.app.brain, &project)
            .await?;
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
        let project = resolve_project(&self.app.brain, &arguments.cwd).await?;
        let session = self
            .sessions
            .load_session(&self.app.brain, &project, session_id)
            .await?;
        let messages = self
            .app
            .brain
            .store
            .message_list(session.id)
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
            let project = resolve_project(&self.app.brain, &cwd).await?;
            let cwd = project
                .root
                .clone()
                .ok_or_else(acp::Error::internal_error)?;
            for session in self
                .sessions
                .list_sessions_for_project(&self.app.brain, project.id)
                .await?
            {
                items.push(list_info(&session, cwd.clone()));
            }
        } else {
            for (project, session) in self.sessions.list_all_sessions(&self.app.brain).await? {
                let Some(cwd) = project.root.clone() else {
                    continue;
                };
                items.push(list_info(&session, cwd));
            }
        }

        Ok(acp::ListSessionsResponse::new(items))
    }

    async fn prompt(
        &self,
        arguments: acp::PromptRequest,
    ) -> Result<acp::PromptResponse, acp::Error> {
        let session_id = parse_session_id(&arguments.session_id)?;
        let cancel = self.sessions.start_turn(session_id).await?;
        let stop_reason = async {
            let session = self
                .app
                .brain
                .store
                .session_get(session_id)
                .await
                .map_err(map_brain_error)?;
            let prompt_text = Self::prompt_text(&arguments.prompt);

            if session.title.is_none() {
                let title = Self::session_title_from_prompt(&prompt_text);
                self.app
                    .brain
                    .store
                    .session_update(session.id, brain_core::SessionUpdate::title(title.clone()))
                    .await
                    .map_err(map_brain_error)?;
                let updated_session = self
                    .app
                    .brain
                    .store
                    .session_get(session.id)
                    .await
                    .map_err(map_brain_error)?;
                self.emit(
                    &arguments.session_id,
                    acp::SessionUpdate::SessionInfoUpdate(session_info_update(&updated_session)),
                )
                .await?;
            }

            let mut stream = self
                .app
                .brain
                .turn(session.id, &prompt_text, cancel.clone());
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

        self.sessions.finish_turn(session_id).await;
        stop_reason.map(acp::PromptResponse::new)
    }

    async fn cancel(&self, arguments: acp::CancelNotification) -> Result<(), acp::Error> {
        let session_id = parse_session_id(&arguments.session_id)?;
        self.sessions.cancel_turn(session_id).await;
        Ok(())
    }

    #[cfg(feature = "unstable_session_model")]
    async fn set_session_model(
        &self,
        arguments: acp::SetSessionModelRequest,
    ) -> Result<acp::SetSessionModelResponse, acp::Error> {
        let session_id = parse_session_id(&arguments.session_id)?;
        let resolved = self
            .app
            .router
            .resolve_model(arguments.model_id.0.as_ref())
            .map_err(|_| acp::Error::invalid_params())?;
        let session = self
            .app
            .brain
            .store
            .session_get(session_id)
            .await
            .map_err(map_brain_error)?;
        let mut inference = session.inference.unwrap_or_default();
        inference.provider = Some(resolved.provider);
        inference.model = Some(resolved.model.id);

        self.app
            .brain
            .update_session_inference(session_id, Some(inference))
            .await
            .map_err(map_brain_error)?;

        Ok(acp::SetSessionModelResponse::new())
    }
}
