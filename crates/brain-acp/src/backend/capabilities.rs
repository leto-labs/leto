use agent_client_protocol as acp;

pub fn initialize_response(protocol_version: acp::ProtocolVersion) -> acp::InitializeResponse {
    let capabilities = acp::AgentCapabilities::new()
        .load_session(true)
        .session_capabilities(
            acp::SessionCapabilities::new().list(acp::SessionListCapabilities::new()),
        );

    acp::InitializeResponse::new(protocol_version)
        .agent_capabilities(capabilities)
        .agent_info(
            acp::Implementation::new("brain-acp", env!("CARGO_PKG_VERSION")).title("Brain ACP"),
        )
}

pub fn session_info_update(session: &brain_core::Session) -> acp::SessionInfoUpdate {
    let update = acp::SessionInfoUpdate::new().updated_at(session.updated_at.to_rfc3339());
    if let Some(title) = &session.title {
        update.title(title.clone())
    } else {
        update
    }
}

#[cfg(feature = "unstable_session_model")]
pub fn session_model_state(
    current_model_id: impl Into<acp::ModelId>,
    models: &[brain_core::ProviderModelInfo],
) -> acp::SessionModelState {
    acp::SessionModelState::new(
        current_model_id,
        models
            .iter()
            .map(|model| {
                let description = model
                    .provider
                    .as_ref()
                    .map(|provider| format!("Provider: {provider}"));
                let info = acp::ModelInfo::new(model.id.clone(), model.name.clone());
                if let Some(description) = description {
                    info.description(description)
                } else {
                    info
                }
            })
            .collect(),
    )
}

pub fn list_info(session: &brain_core::Session, cwd: std::path::PathBuf) -> acp::SessionInfo {
    let info = acp::SessionInfo::new(super::ids::session_id_from_ulid(session.id), cwd)
        .updated_at(session.updated_at.to_rfc3339());
    if let Some(title) = &session.title {
        info.title(title.clone())
    } else {
        info
    }
}
