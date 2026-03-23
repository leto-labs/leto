use std::sync::Arc;

use brain_core::*;

#[derive(Debug, Clone)]
pub struct ExecRuntimeAuth {
    pub provider: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub api_surface: Option<String>,
}

/// Build the embedded CLI runtime so the CLI depends on `BrainRuntime`
/// directly rather than going through `BrainServer`/`BrainApi`.
pub async fn build_runtime(
    config: &ProjectConfig,
    store: Arc<dyn Store>,
) -> Result<Arc<dyn BrainRuntime>, BrainError> {
    let pool = Arc::new(CredentialPool::new(
        store.clone(),
        Arc::new(Fallback::new()),
    ));
    let (default_provider_name, providers) = discover_providers(config, pool).await;
    let runtime = Arc::new(BrainRuntimeNative::new(
        store,
        default_provider_name,
        "simple",
    ));

    for (provider_name, provider) in providers {
        runtime.set_provider(provider_name, provider)?;
    }
    register_common_runtime_surface(&runtime)?;

    let runtime: Arc<dyn BrainRuntime> = runtime;
    Ok(runtime)
}

pub fn build_exec_runtime(
    config: &ProjectConfig,
    store: Arc<dyn Store>,
    auth: &ExecRuntimeAuth,
) -> Result<Arc<dyn BrainRuntime>, BrainError> {
    let runtime = Arc::new(BrainRuntimeNative::new(
        store,
        auth.provider.clone(),
        "simple",
    ));

    let provider: Arc<dyn Provider> = if auth.provider == "mock" {
        Arc::new(MockProvider::new())
    } else {
        let preset = OpenAiConfigPreset::ALL
            .iter()
            .find(|preset| preset.name == auth.provider)
            .ok_or_else(|| {
                BrainError::Internal(format!("unsupported exec provider '{}'", auth.provider))
            })?;
        let api_key = auth.api_key.clone().ok_or_else(|| {
            BrainError::Internal(format!(
                "api key is required for exec provider '{}'",
                auth.provider
            ))
        })?;

        let mut provider_config = preset.into_config(api_key);
        if let Some(model) = config.agent.inference.model.as_deref() {
            provider_config = provider_config.with_model(model);
        }
        if let Some(base_url) = auth.base_url.as_deref() {
            provider_config = provider_config.with_base_url(base_url);
        }
        if let Some(api_surface) = auth.api_surface.as_deref() {
            provider_config =
                provider_config.with_api_surface_mode(parse_api_surface_mode(api_surface)?);
        }

        Arc::new(OpenAiProvider::new(provider_config))
    };

    runtime.set_provider(auth.provider.clone(), provider)?;
    register_common_runtime_surface(&runtime)?;

    let runtime: Arc<dyn BrainRuntime> = runtime;
    Ok(runtime)
}

fn parse_api_surface_mode(value: &str) -> Result<OpenAiApiMode, BrainError> {
    match value {
        "auto" => Ok(OpenAiApiMode::Auto),
        "responses" => Ok(OpenAiApiMode::Responses),
        "chat-completions" | "chat_completions" => Ok(OpenAiApiMode::ChatCompletions),
        other => Err(BrainError::Internal(format!(
            "unsupported api surface '{other}' (expected auto, responses, or chat-completions)"
        ))),
    }
}

fn register_common_runtime_surface(runtime: &Arc<BrainRuntimeNative>) -> Result<(), BrainError> {
    runtime.set_loop("simple", Arc::new(SimpleLoop))?;
    runtime.set_loop("robust", Arc::new(RobustLoop))?;
    runtime.set_loop("terminus2", Arc::new(Terminus2Loop))?;
    runtime.set_loop("terminus-kira", Arc::new(TerminusKiraLoop))?;
    for tool in native_tools() {
        let name = tool.definition().name.clone();
        runtime.set_tool(name, tool)?;
    }
    Ok(())
}

async fn discover_providers(
    config: &ProjectConfig,
    pool: Arc<CredentialPool>,
) -> (String, Vec<(String, Arc<dyn Provider>)>) {
    let requested_default_name = config
        .agent
        .inference
        .provider
        .as_deref()
        .unwrap_or("openai");
    let model_override = config.agent.inference.model.as_deref();
    let mut providers: Vec<(String, Arc<dyn Provider>, bool)> = Vec::new();

    discover_api_providers(
        requested_default_name,
        model_override,
        &pool,
        &mut providers,
    )
    .await;
    discover_oauth_providers(
        requested_default_name,
        model_override,
        &pool,
        &mut providers,
    )
    .await;
    discover_local_providers(
        requested_default_name,
        model_override,
        config,
        &mut providers,
    )
    .await;

    if providers.is_empty() {
        tracing::warn!("no providers configured — run `brain credentials add <provider> <key>`");
        return (
            "mock".to_owned(),
            vec![(
                "mock".to_owned(),
                Arc::new(MockProvider::new().with_delay(30)),
            )],
        );
    }

    let default_name = providers
        .iter()
        .find(|(_, _, is_default)| *is_default)
        .or(providers.first())
        .map(|(name, _, _)| name.clone())
        .expect("providers should not be empty");

    let mut registered = Vec::with_capacity(providers.len());
    for (provider_name, provider, is_default) in providers {
        let tag = if is_default { "default" } else { "provider" };
        tracing::info!("{tag}: {provider_name}");
        registered.push((provider_name, provider));
    }

    (default_name, registered)
}

async fn discover_api_providers(
    default_name: &str,
    model_override: Option<&str>,
    pool: &Arc<CredentialPool>,
    providers: &mut Vec<(String, Arc<dyn Provider>, bool)>,
) {
    for preset in OpenAiConfigPreset::ALL {
        let has_credentials = pool.resolve(preset.name, None).await.is_ok();
        if !has_credentials {
            continue;
        }

        let mut config = preset.into_config(String::new());
        let is_default = preset.name == default_name;
        if is_default && let Some(model) = model_override {
            config = config.with_model(model);
        }

        let provider: Arc<dyn Provider> = Arc::new(OpenAiProvider::with_pool(config, pool.clone()));
        providers.push((preset.name.to_owned(), provider, is_default));
    }
}

#[cfg(feature = "openai-oauth")]
async fn discover_oauth_providers(
    default_name: &str,
    _model_override: Option<&str>,
    pool: &Arc<CredentialPool>,
    providers: &mut Vec<(String, Arc<dyn Provider>, bool)>,
) {
    let preset = &OpenAiOAuthPreset::OPENAI;
    let has_credentials = pool.resolve(preset.name, None).await.is_ok();
    if !has_credentials {
        return;
    }

    let is_default = preset.name == default_name;
    let provider: Arc<dyn Provider> =
        Arc::new(OpenAiOAuthProvider::with_pool(pool.clone(), preset));
    tracing::info!("provider: {} (OAuth)", preset.name);
    providers.push((preset.name.to_owned(), provider, is_default));
}

#[cfg(not(feature = "openai-oauth"))]
async fn discover_oauth_providers(
    _default_name: &str,
    _model_override: Option<&str>,
    _pool: &Arc<CredentialPool>,
    _providers: &mut Vec<(String, Arc<dyn Provider>, bool)>,
) {
}

#[cfg(all(feature = "llamacpp", not(feature = "mistralrs")))]
async fn discover_local_providers(
    default_name: &str,
    _model_override: Option<&str>,
    config: &ProjectConfig,
    providers: &mut Vec<(String, Arc<dyn Provider>, bool)>,
) {
    let default_model = config
        .agent
        .inference
        .model
        .as_deref()
        .unwrap_or("qwen3.5-0.8b")
        .to_owned();

    let configs: Vec<(String, LlamaCppConfig)> = LlamaCppModelPreset::ALL
        .iter()
        .map(|preset| (preset.name.to_owned(), preset.into_config()))
        .collect();

    let Ok(provider) = LlamaCppProvider::new(configs, &default_model) else {
        tracing::warn!("failed to init llama.cpp backend");
        return;
    };

    let is_default = default_name == "llamacpp";
    tracing::info!("backend: llama.cpp | default model: {default_model}");
    providers.push(("llamacpp".to_owned(), Arc::new(provider), is_default));
}

#[cfg(feature = "mistralrs")]
async fn discover_local_providers(
    default_name: &str,
    _model_override: Option<&str>,
    config: &ProjectConfig,
    providers: &mut Vec<(String, Arc<dyn Provider>, bool)>,
) {
    let default_model = config
        .agent
        .inference
        .model
        .as_deref()
        .unwrap_or("qwen3-0.6b")
        .to_owned();

    let configs: Vec<(String, MistralRsConfig)> = MistralRsModelPreset::ALL
        .iter()
        .map(|preset| (preset.name.to_owned(), preset.into_config()))
        .collect();

    let provider = MistralRsProvider::new(configs, &default_model);
    let is_default = default_name == "mistralrs";
    tracing::info!("backend: mistral.rs | default model: {default_model}");
    providers.push(("mistralrs".to_owned(), Arc::new(provider), is_default));
}

#[cfg(not(any(feature = "mistralrs", feature = "llamacpp")))]
async fn discover_local_providers(
    _default_name: &str,
    _model_override: Option<&str>,
    _config: &ProjectConfig,
    _providers: &mut Vec<(String, Arc<dyn Provider>, bool)>,
) {
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_api_surface_mode_accepts_known_values() {
        assert!(matches!(
            parse_api_surface_mode("auto").unwrap(),
            OpenAiApiMode::Auto
        ));
        assert!(matches!(
            parse_api_surface_mode("responses").unwrap(),
            OpenAiApiMode::Responses
        ));
        assert!(matches!(
            parse_api_surface_mode("chat-completions").unwrap(),
            OpenAiApiMode::ChatCompletions
        ));
        assert!(matches!(
            parse_api_surface_mode("chat_completions").unwrap(),
            OpenAiApiMode::ChatCompletions
        ));
    }

    #[test]
    fn parse_api_surface_mode_rejects_unknown_values() {
        let error = parse_api_surface_mode("bogus").unwrap_err();
        assert!(error.to_string().contains("unsupported api surface"));
    }

    #[tokio::test]
    async fn build_runtime_registers_mock_when_no_credentials() {
        let store: Arc<dyn Store> = Arc::new(InMemoryStore::new());
        let config = ProjectConfig::default();

        let runtime = build_runtime(&config, store).await.unwrap();
        assert!(runtime.providers().get("mock").is_some());
    }

    #[tokio::test]
    async fn build_runtime_discovers_stored_credential() {
        let store: Arc<dyn Store> = Arc::new(InMemoryStore::new());
        store
            .credentials()
            .create(
                ("openai".into(), "default".into()),
                CredentialEntry::api_key("default", "sk-test"),
            )
            .await
            .unwrap();

        let config = ProjectConfig::default();
        let runtime = build_runtime(&config, store).await.unwrap();

        assert!(runtime.providers().get("openai").is_some());
        assert!(runtime.providers().get("mock").is_none());
    }

    #[tokio::test]
    async fn config_provider_field_sets_runtime_default_provider() {
        let store: Arc<dyn Store> = Arc::new(InMemoryStore::new());
        store
            .credentials()
            .create(
                ("groq".into(), "default".into()),
                CredentialEntry::api_key("default", "gsk-test"),
            )
            .await
            .unwrap();
        store
            .credentials()
            .create(
                ("openai".into(), "default".into()),
                CredentialEntry::api_key("default", "sk-test"),
            )
            .await
            .unwrap();

        let mut config = ProjectConfig::default();
        config.agent.inference.provider = Some("groq".to_owned());

        let runtime = build_runtime(&config, store.clone()).await.unwrap();

        let project = Project::new(Some("test".into()), None, ProjectConfig::default());
        let project_id = project.id;
        store.projects().create(project_id, project).await.unwrap();
        let session = Session::new(project_id);
        let session = store.sessions().create(session.id, session).await.unwrap();

        let current_model = runtime
            .current_model_id_for_session(session.id)
            .await
            .unwrap();
        assert_eq!(current_model.as_deref(), Some("llama-3.3-70b-versatile"));
    }
}
