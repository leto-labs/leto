use std::sync::Arc;

use brain_core::*;

/// Build a unified `ProviderRouter` from stored credentials and config.
///
/// API providers are discovered from the credential store — each preset
/// that has at least one stored credential gets registered. The default
/// provider is determined by `config.agent.inference.provider` (falling
/// back to "openai"). Local providers are added when their feature is
/// enabled.
pub async fn build_provider(
    config: &ProjectConfig,
    pool: Arc<CredentialPool>,
) -> Arc<dyn Provider> {
    let default_name = config
        .agent
        .inference
        .provider
        .as_deref()
        .unwrap_or("openai");

    let model_override = config.agent.inference.model.as_deref();

    let mut providers: Vec<(String, Arc<dyn Provider>, bool)> = Vec::new();

    discover_api_providers(default_name, model_override, &pool, &mut providers).await;
    discover_oauth_providers(default_name, model_override, &pool, &mut providers).await;
    discover_local_providers(default_name, model_override, config, &mut providers).await;

    if providers.is_empty() {
        tracing::warn!("no providers configured — run `brain credentials add <provider> <key>`");
        return Arc::new(MockProvider::new().with_delay(30));
    }

    let default = providers
        .iter()
        .find(|(_, _, is_default)| *is_default)
        .or(providers.first())
        .map(|(_, p, _)| p.clone())
        .unwrap();

    let mut router = ProviderRouter::new(default);
    for (model_name, provider, is_default) in &providers {
        let tag = if *is_default { "default" } else { "  route" };
        tracing::info!("{tag}: {model_name}");
        router = router.add(model_name.clone(), provider.clone());
    }

    Arc::new(router)
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
        if is_default {
            if let Some(model) = model_override {
                config = config.with_model(model);
            }
        }
        let model_name = config.default_model.clone();
        let provider: Arc<dyn Provider> = Arc::new(OpenAiProvider::with_pool(config, pool.clone()));
        providers.push((model_name, provider, is_default));
    }
}

#[cfg(feature = "openai-oauth")]
async fn discover_oauth_providers(
    default_name: &str,
    model_override: Option<&str>,
    pool: &Arc<CredentialPool>,
    providers: &mut Vec<(String, Arc<dyn Provider>, bool)>,
) {
    let preset = &OpenAiOAuthPreset::OPENAI;

    let has_credentials = pool.resolve(preset.name, None).await.is_ok();
    if !has_credentials {
        return;
    }

    let is_default = preset.name == default_name;
    let model = if is_default {
        model_override.unwrap_or(preset.default_model)
    } else {
        preset.default_model
    };

    let provider: Arc<dyn Provider> =
        Arc::new(OpenAiOAuthProvider::with_pool(pool.clone(), preset));
    tracing::info!("provider: {} (OAuth) | model: {model}", preset.name);
    providers.push((model.to_owned(), provider, is_default));
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
    model_override: Option<&str>,
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
        .map(|p| (p.name.to_owned(), p.into_config()))
        .collect();

    let Ok(provider) = LlamaCppProvider::new(configs, &default_model) else {
        tracing::warn!("failed to init llama.cpp backend");
        return;
    };

    let is_default = default_name == "llamacpp";
    let model = if is_default {
        model_override.unwrap_or(&default_model).to_owned()
    } else {
        default_model
    };
    tracing::info!("backend: llama.cpp | default model: {model}");
    providers.push((model, Arc::new(provider), is_default));
}

#[cfg(feature = "mistralrs")]
async fn discover_local_providers(
    default_name: &str,
    model_override: Option<&str>,
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
        .map(|p| (p.name.to_owned(), p.into_config()))
        .collect();

    let provider = MistralRsProvider::new(configs, &default_model);

    let is_default = default_name == "mistralrs";
    let model = if is_default {
        model_override.unwrap_or(&default_model).to_owned()
    } else {
        default_model
    };
    tracing::info!("backend: mistral.rs | default model: {model}");
    providers.push((model, Arc::new(provider), is_default));
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

    #[tokio::test]
    async fn build_provider_returns_mock_when_no_credentials() {
        let store: Arc<dyn Store> = Arc::new(InMemoryStore::new());
        let pool = Arc::new(CredentialPool::new(
            store as Arc<dyn CredentialStore>,
            Arc::new(Fallback::new()),
        ));
        let config = ProjectConfig::default();

        let provider = build_provider(&config, pool).await;
        let info = provider.info();
        assert_eq!(info.name, "mock");
    }

    #[tokio::test]
    async fn build_provider_discovers_stored_credential() {
        let store: Arc<dyn Store> = Arc::new(InMemoryStore::new());
        store
            .credential_save("openai", &CredentialEntry::api_key("default", "sk-test"))
            .await
            .unwrap();

        let pool = Arc::new(CredentialPool::new(
            store as Arc<dyn CredentialStore>,
            Arc::new(Fallback::new()),
        ));
        let config = ProjectConfig::default();

        let provider = build_provider(&config, pool).await;
        let info = provider.info();
        assert_ne!(info.name, "mock", "should have found the openai credential");
    }

    #[tokio::test]
    async fn config_provider_field_sets_default() {
        let store: Arc<dyn Store> = Arc::new(InMemoryStore::new());
        store
            .credential_save("groq", &CredentialEntry::api_key("default", "gsk-test"))
            .await
            .unwrap();
        store
            .credential_save("openai", &CredentialEntry::api_key("default", "sk-test"))
            .await
            .unwrap();

        let pool = Arc::new(CredentialPool::new(
            store as Arc<dyn CredentialStore>,
            Arc::new(Fallback::new()),
        ));

        let mut config = ProjectConfig::default();
        config.agent.inference.provider = Some("groq".to_owned());

        let provider = build_provider(&config, pool).await;
        let info = provider.info();
        assert_eq!(
            info.default_model,
            Some("llama-3.3-70b-versatile".to_owned())
        );
    }
}
