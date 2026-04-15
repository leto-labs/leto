//! ACP backend adapter wrapping AgentCore.

use agent_client_protocol as acp;
use agent_core::AgentCore;
use futures::StreamExt;
use provider::Message;
use tokio::sync::{mpsc, oneshot};

use crate::capabilities::{
    COMMAND_PLAN, COMMAND_PTY, COMMAND_SUBAGENTS, COMMAND_WORKTREE, CONFIG_MODEL,
    CONFIG_THOUGHT_LEVEL, available_commands_update, current_mode_update, initialize_response,
    session_mode_state,
};
use crate::errors::{internal_error, map_store_error};
use crate::event_mapper::{EventMapper, MappedEvent};
use crate::file_bridge::AcpFileBridge;
use crate::history_replay::replay_updates;
use crate::ids::{parse_session_id, session_id_from_ulid};

pub(super) type NotificationEnvelope = (acp::SessionNotification, oneshot::Sender<()>);

/// ACP backend that wraps an AgentCore implementation.
pub struct AgentCoreAcpBackend<C> {
    core: C,
    file_bridge: AcpFileBridge,
    session_update_tx: mpsc::UnboundedSender<NotificationEnvelope>,
}

impl<C: AgentCore> AgentCoreAcpBackend<C> {
    /// Creates a new ACP backend wrapping the given core.
    pub fn new(core: C) -> Self {
        let (tx, _rx) = mpsc::unbounded_channel();
        Self::with_file_bridge(core, AcpFileBridge::new(), tx)
    }

    /// Creates a new ACP backend using the provided notification channel.
    pub fn with_notification_sender(
        core: C,
        session_update_tx: mpsc::UnboundedSender<NotificationEnvelope>,
    ) -> Self {
        Self::with_file_bridge(core, AcpFileBridge::new(), session_update_tx)
    }

    pub(crate) fn with_file_bridge(
        core: C,
        file_bridge: AcpFileBridge,
        session_update_tx: mpsc::UnboundedSender<NotificationEnvelope>,
    ) -> Self {
        Self {
            core,
            file_bridge,
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

        Ok(options)
    }

    #[cfg(feature = "unstable_session_model")]
    async fn session_models_for_session(
        &self,
        session_id: agent_store::SessionId,
    ) -> Result<Option<acp::SessionModelState>, acp::Error> {
        let current_model_id = self
            .core
            .current_model_id_for_session(session_id)
            .await
            .map_err(map_store_error)?;
        let mut available_models: Vec<_> = self
            .core
            .list_models()
            .into_iter()
            .map(|model| {
                let mut info =
                    acp::ModelInfo::new(model.model.id.to_string(), model.model.name.to_string());
                if let Some(family) = model.model.family.as_ref() {
                    info = info.description(format!(
                        "{} model from provider `{}`",
                        family, model.provider_name
                    ));
                }
                info
            })
            .collect();

        let Some(current_model_id) = current_model_id.or_else(|| {
            available_models
                .first()
                .map(|model| model.model_id.0.to_string())
        }) else {
            return Ok(None);
        };

        if !available_models
            .iter()
            .any(|model| model.model_id.0.as_ref() == current_model_id)
        {
            available_models.push(acp::ModelInfo::new(
                current_model_id.clone(),
                current_model_id.clone(),
            ));
        }

        Ok(Some(acp::SessionModelState::new(
            current_model_id,
            available_models,
        )))
    }

    async fn session_modes_for_session(
        &self,
        session_id: agent_store::SessionId,
    ) -> Result<Option<acp::SessionModeState>, acp::Error> {
        let current_loop_name = self
            .core
            .current_loop_name_for_session(session_id)
            .await
            .map_err(map_store_error)?;
        let loops = self.core.loop_names();
        Ok(session_mode_state(current_loop_name, &loops))
    }

    async fn set_session_mode_by_loop(
        &self,
        session_id: agent_store::SessionId,
        mode_id: &str,
    ) -> Result<(), acp::Error> {
        let loops = self.core.loop_names();
        if !loops.iter().any(|loop_name| loop_name == mode_id) {
            return Err(acp::Error::invalid_params());
        }

        self.core
            .update_session(
                session_id,
                agent_store::SessionUpdate {
                    loop_name: Some(Some(mode_id.to_owned())),
                    ..agent_store::SessionUpdate::default()
                },
            )
            .await
            .map_err(map_store_error)?;

        Ok(())
    }

    fn session_info_update(session: &agent_store::Session) -> acp::SessionUpdate {
        acp::SessionUpdate::SessionInfoUpdate(
            acp::SessionInfoUpdate::new()
                .title(session.title.clone())
                .updated_at(session.updated_at.to_rfc3339()),
        )
    }

    async fn emit_config_option_update(
        &self,
        session_id: &acp::SessionId,
        store_session_id: agent_store::SessionId,
    ) -> Result<(), acp::Error> {
        self.emit(
            session_id,
            acp::SessionUpdate::ConfigOptionUpdate(acp::ConfigOptionUpdate::new(
                self.config_options_for_session(store_session_id).await?,
            )),
        )
        .await
    }

    async fn session_info(
        &self,
        session: &agent_store::Session,
    ) -> Result<acp::SessionInfo, acp::Error> {
        let project = self
            .core
            .project(session.project_id)
            .await
            .map_err(map_store_error)?;
        let Some(cwd) = project.root.filter(|root| root.is_absolute()) else {
            return Err(acp::Error::internal_error().data(format!(
                "session {} has no absolute project root",
                session.id
            )));
        };

        Ok(acp::SessionInfo::new(session_id_from_ulid(session.id), cwd)
            .title(session.title.clone())
            .updated_at(Some(session.updated_at.to_rfc3339())))
    }

    fn forked_messages(
        source_messages: Vec<agent_store::StoredMessage>,
        target_session_id: agent_store::SessionId,
    ) -> Vec<agent_store::StoredMessage> {
        source_messages
            .into_iter()
            .map(|message| agent_store::StoredMessage {
                id: ulid::Ulid::new(),
                session_id: target_session_id,
                ordinal: message.ordinal,
                created_at: message.created_at,
                message: message.message,
            })
            .collect()
    }

    fn rewrite_prompt_command(prompt_text: &str) -> String {
        let trimmed = prompt_text.trim();
        let Some(rest) = trimmed.strip_prefix('/') else {
            return prompt_text.to_owned();
        };

        let (command, remainder) = match rest.split_once(char::is_whitespace) {
            Some((command, remainder)) => (command, remainder.trim()),
            None => (rest, ""),
        };

        let instruction = match command {
            COMMAND_PLAN => {
                "Before you act, write a concise execution plan and then continue with the user's request."
            }
            COMMAND_PTY => {
                "Prefer the PTY runtime tools when helpful, especially `open_pty`, `write_pty_input`, `capture_pty`, and `close_pty`."
            }
            COMMAND_WORKTREE => {
                "Prefer the git worktree runtime tools when helpful, especially `create_worktree`, `bind_worktree`, and `remove_worktree`."
            }
            COMMAND_SUBAGENTS => {
                "Prefer the child-agent tools when helpful, especially `spawn_agent` and follow-up child-session interactions."
            }
            _ => return prompt_text.to_owned(),
        };

        if remainder.is_empty() {
            instruction.to_owned()
        } else {
            format!("{instruction}\n\nUser request: {remainder}")
        }
    }
}

#[async_trait::async_trait(?Send)]
impl<C: AgentCore> acp::Agent for AgentCoreAcpBackend<C> {
    async fn initialize(
        &self,
        arguments: acp::InitializeRequest,
    ) -> Result<acp::InitializeResponse, acp::Error> {
        self.file_bridge
            .set_client_capabilities(&arguments.client_capabilities)
            .await;
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
        self.file_bridge
            .remember_session_cwd(&session_id, &arguments.cwd)
            .await;

        self.emit(&session_id, Self::session_info_update(&session))
            .await?;
        self.emit(&session_id, available_commands_update()).await?;

        let response = acp::NewSessionResponse::new(session_id)
            .modes(self.session_modes_for_session(session.id).await?)
            .config_options(self.config_options_for_session(session.id).await?);
        #[cfg(feature = "unstable_session_model")]
        let response = response.models(self.session_models_for_session(session.id).await?);

        Ok(response)
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
        self.file_bridge
            .remember_session_cwd(&acp_session_id, &arguments.cwd)
            .await;

        for update in replay_updates(&session, &messages) {
            self.emit(&acp_session_id, update).await?;
        }
        self.emit(&acp_session_id, available_commands_update())
            .await?;

        let response = acp::LoadSessionResponse::new()
            .modes(self.session_modes_for_session(session.id).await?)
            .config_options(self.config_options_for_session(session.id).await?);
        #[cfg(feature = "unstable_session_model")]
        let response = response.models(self.session_models_for_session(session.id).await?);

        Ok(response)
    }

    #[cfg(feature = "unstable_session_resume")]
    async fn resume_session(
        &self,
        arguments: acp::ResumeSessionRequest,
    ) -> Result<acp::ResumeSessionResponse, acp::Error> {
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

        self.file_bridge
            .remember_session_cwd(&arguments.session_id, &arguments.cwd)
            .await;
        self.emit(&arguments.session_id, Self::session_info_update(&session))
            .await?;
        self.emit(&arguments.session_id, available_commands_update())
            .await?;

        let response = acp::ResumeSessionResponse::new()
            .modes(self.session_modes_for_session(session.id).await?)
            .config_options(self.config_options_for_session(session.id).await?);
        #[cfg(feature = "unstable_session_model")]
        let response = response.models(self.session_models_for_session(session.id).await?);

        Ok(response)
    }

    #[cfg(feature = "unstable_session_fork")]
    async fn fork_session(
        &self,
        arguments: acp::ForkSessionRequest,
    ) -> Result<acp::ForkSessionResponse, acp::Error> {
        let source_session_id = parse_session_id(&arguments.session_id)?;
        let project = self.resolve_project(&arguments.cwd).await?;
        let source_session = self
            .core
            .session(source_session_id)
            .await
            .map_err(map_store_error)?;

        if source_session.project_id != project.id {
            return Err(acp::Error::invalid_params());
        }

        let forked_session = self
            .core
            .create_session(project.id)
            .await
            .map_err(map_store_error)?;
        let forked_session = self
            .core
            .update_session(
                forked_session.id,
                agent_store::SessionUpdate {
                    title: source_session.title.clone(),
                    provider: Some(source_session.provider.clone()),
                    model: Some(source_session.model.clone()),
                    loop_name: Some(source_session.loop_name.clone()),
                    request: Some(source_session.request.clone()),
                },
            )
            .await
            .map_err(map_store_error)?;

        let source_messages = self
            .core
            .messages(source_session_id)
            .await
            .map_err(map_store_error)?;
        self.core
            .store()
            .messages()
            .replace_for_session(
                forked_session.id,
                Self::forked_messages(source_messages, forked_session.id),
            )
            .await
            .map_err(map_store_error)?;

        if let Some(trajectory) = self
            .core
            .trajectory(source_session_id)
            .await
            .map_err(map_store_error)?
        {
            self.core
                .upsert_trajectory(forked_session.id, trajectory)
                .await
                .map_err(map_store_error)?;
        }

        let forked_acp_session_id = session_id_from_ulid(forked_session.id);
        self.file_bridge
            .remember_session_cwd(&forked_acp_session_id, &arguments.cwd)
            .await;
        self.emit(
            &forked_acp_session_id,
            Self::session_info_update(&forked_session),
        )
        .await?;
        self.emit(&forked_acp_session_id, available_commands_update())
            .await?;

        let response = acp::ForkSessionResponse::new(forked_acp_session_id)
            .modes(self.session_modes_for_session(forked_session.id).await?)
            .config_options(self.config_options_for_session(forked_session.id).await?);
        #[cfg(feature = "unstable_session_model")]
        let response = response.models(self.session_models_for_session(forked_session.id).await?);

        Ok(response)
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
            for session in sessions {
                items.push(self.session_info(&session).await?);
            }
        } else {
            let sessions = self
                .core
                .store()
                .sessions()
                .list()
                .await
                .map_err(map_store_error)?;
            for session in sessions {
                if let Ok(info) = self.session_info(&session).await {
                    items.push(info);
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
        let prompt_text = Self::prompt_text(&arguments.prompt);
        let runtime_prompt_text = Self::rewrite_prompt_command(&prompt_text);

        // Update session title if not set
        let session = self
            .core
            .session(session_id)
            .await
            .map_err(map_store_error)?;
        if session.title.is_none() {
            let title = Self::session_title_from_prompt(&prompt_text);
            let updated_session = self
                .core
                .update_session(
                    session_id,
                    agent_store::SessionUpdate {
                        title: Some(title),
                        ..agent_store::SessionUpdate::default()
                    },
                )
                .await
                .map_err(map_store_error)?;
            self.emit(
                &arguments.session_id,
                Self::session_info_update(&updated_session),
            )
            .await?;
        }

        // Run the turn
        let messages = vec![Message::user_text(&runtime_prompt_text)];
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
                MappedEvent::Failed(message) => {
                    tracing::error!("agent-acp prompt failed: {message}");
                    return Err(internal_error(message));
                }
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

    #[cfg(feature = "unstable_session_close")]
    async fn close_session(
        &self,
        arguments: acp::CloseSessionRequest,
    ) -> Result<acp::CloseSessionResponse, acp::Error> {
        let session_id = parse_session_id(&arguments.session_id)?;
        self.core
            .session(session_id)
            .await
            .map_err(map_store_error)?;
        self.core
            .cancel_turn(session_id)
            .await
            .map_err(map_store_error)?;
        self.file_bridge.forget_session(&arguments.session_id).await;
        Ok(acp::CloseSessionResponse::new())
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
            _ => return Err(acp::Error::invalid_params()),
        }

        let config_options = self.config_options_for_session(session_id).await?;
        self.emit_config_option_update(&arguments.session_id, session_id)
            .await?;

        Ok(acp::SetSessionConfigOptionResponse::new(config_options))
    }

    async fn set_session_mode(
        &self,
        arguments: acp::SetSessionModeRequest,
    ) -> Result<acp::SetSessionModeResponse, acp::Error> {
        let session_id = parse_session_id(&arguments.session_id)?;
        self.set_session_mode_by_loop(session_id, arguments.mode_id.0.as_ref())
            .await?;
        self.emit(
            &arguments.session_id,
            current_mode_update(arguments.mode_id.0.to_string()),
        )
        .await?;
        Ok(acp::SetSessionModeResponse::new())
    }

    #[cfg(feature = "unstable_session_model")]
    async fn set_session_model(
        &self,
        arguments: acp::SetSessionModelRequest,
    ) -> Result<acp::SetSessionModelResponse, acp::Error> {
        let session_id = parse_session_id(&arguments.session_id)?;
        let models = self
            .session_models_for_session(session_id)
            .await?
            .ok_or_else(acp::Error::invalid_params)?;
        if !models
            .available_models
            .iter()
            .any(|model| model.model_id == arguments.model_id)
        {
            return Err(acp::Error::invalid_params());
        }

        self.core
            .update_session(
                session_id,
                agent_store::SessionUpdate {
                    model: Some(Some(arguments.model_id.0.to_string())),
                    ..agent_store::SessionUpdate::default()
                },
            )
            .await
            .map_err(map_store_error)?;
        self.emit_config_option_update(&arguments.session_id, session_id)
            .await?;

        Ok(acp::SetSessionModelResponse::new())
    }
}
