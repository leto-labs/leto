use agent_client_protocol as acp;

pub const CONFIG_MODEL: &str = "model";
pub const CONFIG_THOUGHT_LEVEL: &str = "thought_level";

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
                acp::ModelInfo::new(model.model.id, model.model.name)
                    .description(format!("Provider: {}", model.provider))
            })
            .collect(),
    )
}

pub fn config_options(
    current_model: &brain_core::ProviderModelInfo,
    effective_inference: &brain_core::InferenceConfig,
    models: &[brain_core::ProviderModelInfo],
) -> Vec<acp::SessionConfigOption> {
    let mut provider_groups = Vec::<(String, Vec<acp::SessionConfigSelectOption>)>::new();
    for model in models {
        let option = acp::SessionConfigSelectOption::new(model.model.id, model.model.name)
            .description(format!("Model ID: {}", model.model.id));
        if let Some((_, options)) = provider_groups
            .iter_mut()
            .find(|(provider, _)| *provider == model.provider)
        {
            options.push(option);
        } else {
            provider_groups.push((model.provider.clone(), vec![option]));
        }
    }

    let mut config_options = vec![
        acp::SessionConfigOption::select(
            CONFIG_MODEL,
            "Model",
            current_model.model.id,
            provider_groups
                .into_iter()
                .map(|(provider, options)| {
                    acp::SessionConfigSelectGroup::new(provider.clone(), provider, options)
                })
                .collect::<Vec<_>>(),
        )
        .description("Selects the active model for this session.")
        .category(acp::SessionConfigOptionCategory::Model),
    ];

    if let Some(levels) = current_model.model.reasoning {
        let current_value = effective_inference
            .reasoning
            .as_deref()
            .filter(|value| levels.contains(value))
            .unwrap_or(levels[0]);
        config_options.push(
            acp::SessionConfigOption::select(
                CONFIG_THOUGHT_LEVEL,
                "Thought Level",
                current_value.to_owned(),
                levels
                    .iter()
                    .map(|level| {
                        acp::SessionConfigSelectOption::new(*level, title_case_words(level))
                    })
                    .collect::<Vec<_>>(),
            )
            .description("Controls how much reasoning effort the current model should use.")
            .category(acp::SessionConfigOptionCategory::ThoughtLevel),
        );
    }

    config_options
}

fn title_case_words(value: &str) -> String {
    value
        .split('_')
        .flat_map(|segment| segment.split('-'))
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            let mut chars = segment.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
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

#[cfg(test)]
mod tests {
    use super::*;

    fn model(
        provider: &str,
        id: &'static str,
        name: &'static str,
        reasoning: Option<&'static [&'static str]>,
    ) -> brain_core::ProviderModelInfo {
        brain_core::ProviderModelInfo {
            provider: provider.into(),
            model: brain_core::ModelInfo {
                id,
                name,
                family: None,
                reasoning,
                tool_call: true,
                attachment: false,
                structured_output: None,
                temperature: None,
                knowledge: None,
                release_date: None,
                last_updated: None,
                open_weights: None,
                input_modalities: &["text"],
                output_modalities: &["text"],
                cost: None,
                limit: None,
                status: None,
            },
        }
    }

    #[test]
    fn config_options_group_models_by_provider_and_add_thought_level_when_supported() {
        let current_model = model(
            "openai",
            "gpt-5.4",
            "GPT-5.4",
            Some(&["low", "medium", "high"]),
        );
        let models = vec![
            current_model.clone(),
            model("openai", "gpt-5.4-mini", "GPT-5.4 Mini", None),
            model(
                "anthropic",
                "claude-4",
                "Claude 4",
                Some(&["medium", "high"]),
            ),
        ];

        let options = config_options(
            &current_model,
            &brain_core::InferenceConfig {
                provider: Some("openai".into()),
                model: Some("gpt-5.4".into()),
                reasoning: Some("high".into()),
                max_tokens: None,
                temperature: None,
            },
            &models,
        );

        assert_eq!(options.len(), 2);
        assert_eq!(options[0].id.0.as_ref(), CONFIG_MODEL);
        assert_eq!(options[1].id.0.as_ref(), CONFIG_THOUGHT_LEVEL);
    }
}
