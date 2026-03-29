//! ACP backend adapter wrapping AgentCore.

use agent_client_protocol as acp;
use agent_core::AgentCore;
use futures::StreamExt;
use tokio::sync::{mpsc, oneshot};

use crate::capabilities::{CONFIG_LOOP, CONFIG_MODEL, CONFIG_THOUGHT_LEVEL, initialize_response};
use crate::errors::{internal_error, map_store_error};
use crate::event_mapper::{EventMapper, MappedEvent};
use crate::history_replay::replay_updates;
use crate::ids::{parse_session_id, session_id_from_ulid};

pub(super) type NotificationEnvelope = (acp::SessionNotification, oneshot::Sender<()>);

/// ACP backend that wraps an AgentCore implementation.
pub struct AgentCoreAcpBackend<C> {
    core: C,
    session_update_tx: mpsc::UnboundedSender<NotificationEnvelope>,
}

impl<C: AgentCore> AgentCoreAcpBackend<C> {
    /// Creates a new ACP backend wrapping the given core.
    pub fn new(core: C) -> Self {
        let (tx, _rx) = mpsc::unbounded_channel();
        Self::with_notification_sender(core, tx)
    }

    /// Creates a new ACP backend using the provided notification channel.
    pub fn with_notification_sender(
        core: C,
        session_update_tx: mpsc::UnboundedSender<NotificationEnvelope>,
    ) -> Self {
        Self {
            core,
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
    ) -> Result<agent_store::Project, acp::Error> {
        self.core
            .resolve_or_create_project(cwd.to_path_buf())
            .await
            .map_err(map_store_error)
    }

    async fn config_options_for_session(
        &self,
        session_id: agent_store::SessionId,
    ) -> Result<Vec<acp::SessionConfigOption>, acp::Error> {
        let current_model = self
            .core
            .current_model_id_for_session(session_id)
            .await
            .map_err(map_store_error)?;
        let current_loop_name = self
            .core
            .current_loop_name_for_session(session_id)
            .await
            .map_err(map_store_error)?;
        let runtime_config = self
            .core
            .effective_runtime_config(session_id)
            .await
            .map_err(map_store_error)?;
        let current_thought_level = runtime_config
            .request
            .reasoning
            .as_ref()
            .and_then(|reasoning| reasoning.effort.clone())
            .unwrap_or_else(|| "medium".to_owned());
        let models = self.core.list_models();
        let loops = self.core.loop_names();

        let mut options = Vec::new();

        // Model config
        if let Some(model_id) = current_model {
            let mut model_options: Vec<_> = models
                .into_iter()
                .map(|model| {
                    acp::SessionConfigSelectOption::new(
                        model.model.id.to_string(),
                        model.model.name.to_string(),
                    )
                })
                .collect();
            if !model_options
                .iter()
                .any(|option| option.value.0.as_ref() == model_id)
            {
                model_options.push(acp::SessionConfigSelectOption::new(
                    model_id.clone(),
                    model_id.clone(),
                ));
            }
            options.push(
                acp::SessionConfigOption::select(CONFIG_MODEL, "Model", model_id, model_options)
                    .category(acp::SessionConfigOptionCategory::Model),
            );
        }

        options.push(
            acp::SessionConfigOption::select(
                CONFIG_THOUGHT_LEVEL,
                "Thought Level",
                current_thought_level,
                vec![
                    acp::SessionConfigSelectOption::new("low", "Low"),
                    acp::SessionConfigSelectOption::new("medium", "Medium"),
                    acp::SessionConfigSelectOption::new("high", "High"),
                ],
            )
            .category(acp::SessionConfigOptionCategory::ThoughtLevel),
        );

        // Loop config
        if !loops.is_empty() {
            let loop_options: Vec<_> = loops
                .into_iter()
                .map(|name| acp::SessionConfigSelectOption::new(name.clone(), name))
                .collect();
            options.push(
                acp::SessionConfigOption::select(
                    CONFIG_LOOP,
                    "Loop",
                    current_loop_name,
                    loop_options,
                )
                .category(acp::SessionConfigOptionCategory::Mode),
            );
        }

        Ok(options)
    }
}

#[async_trait::async_trait(?Send)]
impl<C: AgentCore> acp::Agent for AgentCoreAcpBackend<C> {
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
        let session = self
            .core
            .create_session(project.id)
            .await
            .map_err(map_store_error)?;
        let session_id = session_id_from_ulid(session.id);

        self.emit(
            &session_id,
            acp::SessionUpdate::SessionInfoUpdate(
                acp::SessionInfoUpdate::new().updated_at(session.updated_at.to_rfc3339()),
            ),
        )
        .await?;

        Ok(acp::NewSessionResponse::new(session_id)
            .config_options(self.config_options_for_session(session.id).await?))
    }

    async fn load_session(
        &self,
        arguments: acp::LoadSessionRequest,
    ) -> Result<acp::LoadSessionResponse, acp::Error> {
        let session_id = parse_session_id(&arguments.session_id)?;
        let project = self.resolve_project(&arguments.cwd).await?;
        let session = self
            .core
            .session(session_id)
            .await
            .map_err(map_store_error)?;

        if session.project_id != project.id {
            return Err(acp::Error::invalid_params());
        }

        let messages = self
            .core
            .messages(session_id)
            .await
            .map_err(map_store_error)?;
        let acp_session_id = session_id_from_ulid(session.id);

        for update in replay_updates(&session, &messages) {
            self.emit(&acp_session_id, update).await?;
        }

        Ok(acp::LoadSessionResponse::new()
            .config_options(self.config_options_for_session(session.id).await?))
    }

    async fn list_sessions(
        &self,
        arguments: acp::ListSessionsRequest,
    ) -> Result<acp::ListSessionsResponse, acp::Error> {
        let mut items = Vec::new();

        if let Some(cwd) = arguments.cwd {
            let project = self.resolve_project(&cwd).await?;
            let sessions = self
                .core
                .sessions_for_project(project.id)
                .await
                .map_err(map_store_error)?;
            let cwd = project.root.unwrap_or(cwd);
            for session in sessions {
                items.push(
                    acp::SessionInfo::new(session_id_from_ulid(session.id), cwd.clone())
                        .title(session.title.clone())
                        .updated_at(Some(session.updated_at.to_rfc3339())),
                );
            }
        }

        Ok(acp::ListSessionsResponse::new(items))
    }

    async fn prompt(
        &self,
        arguments: acp::PromptRequest,
    ) -> Result<acp::PromptResponse, acp::Error> {
        let session_id = parse_session_id(&arguments.session_id)?;
        let prompt_text = Self::prompt_text(&arguments.prompt);

        // Update session title if not set
        let session = self
            .core
            .session(session_id)
            .await
            .map_err(map_store_error)?;
        if session.title.is_none() {
            let title = Self::session_title_from_prompt(&prompt_text);
            self.core
                .update_session(
                    session_id,
                    agent_store::SessionUpdate {
                        title: Some(title),
                        ..agent_store::SessionUpdate::default()
                    },
                )
                .await
                .map_err(map_store_error)?;
        }

        // Run the turn
        let messages = vec![provider::Message::user_text(&prompt_text)];
        let mut stream = self
            .core
            .turn(session_id, messages)
            .await
            .map_err(map_store_error)?;
        let mut mapper = EventMapper::new();

        while let Some(event) = stream.next().await {
            match mapper.map(event) {
                MappedEvent::Updates(updates) => {
                    for update in updates {
                        self.emit(&arguments.session_id, update).await?;
                    }
                }
                MappedEvent::Cancelled => {
                    return Ok(acp::PromptResponse::new(acp::StopReason::Cancelled));
                }
                MappedEvent::Failed(message) => return Err(internal_error(message)),
                MappedEvent::TurnComplete(stop_reason) => {
                    return Ok(acp::PromptResponse::new(stop_reason));
                }
            }
        }

        Ok(acp::PromptResponse::new(acp::StopReason::EndTurn))
    }

    async fn cancel(&self, arguments: acp::CancelNotification) -> Result<(), acp::Error> {
        let session_id = parse_session_id(&arguments.session_id)?;
        self.core
            .cancel_turn(session_id)
            .await
            .map_err(map_store_error)
    }

    async fn set_session_config_option(
        &self,
        arguments: acp::SetSessionConfigOptionRequest,
    ) -> Result<acp::SetSessionConfigOptionResponse, acp::Error> {
        let session_id = parse_session_id(&arguments.session_id)?;

        match arguments.config_id.0.as_ref() {
            CONFIG_MODEL => {
                self.core
                    .update_session(
                        session_id,
                        agent_store::SessionUpdate {
                            model: Some(Some(arguments.value.0.to_string())),
                            ..agent_store::SessionUpdate::default()
                        },
                    )
                    .await
                    .map_err(map_store_error)?;
            }
            CONFIG_THOUGHT_LEVEL => {
                let session = self
                    .core
                    .session(session_id)
                    .await
                    .map_err(map_store_error)?;
                let mut request = session.request;
                let mut reasoning = request.reasoning.unwrap_or_default();
                reasoning.effort = Some(arguments.value.0.to_string());
                request.reasoning = Some(reasoning);
                self.core
                    .update_session(
                        session_id,
                        agent_store::SessionUpdate {
                            request: Some(request),
                            ..agent_store::SessionUpdate::default()
                        },
                    )
                    .await
                    .map_err(map_store_error)?;
            }
            CONFIG_LOOP => {
                self.core
                    .update_session(
                        session_id,
                        agent_store::SessionUpdate {
                            loop_name: Some(Some(arguments.value.0.to_string())),
                            ..agent_store::SessionUpdate::default()
                        },
                    )
                    .await
                    .map_err(map_store_error)?;
            }
            _ => return Err(acp::Error::invalid_params()),
        }

        Ok(acp::SetSessionConfigOptionResponse::new(
            self.config_options_for_session(session_id).await?,
        ))
    }
}
