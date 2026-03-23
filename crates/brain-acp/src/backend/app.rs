use std::sync::Arc;

use agent_client_protocol as acp;
use brain_core::*;
use brain_tools::{FileReadDriverAcp, FileWriteDriverAcp};

use super::errors::internal_error;

pub struct BackendApp {
    pub runtime: Arc<dyn BrainRuntime>,
}

impl BackendApp {
    pub fn new(runtime: Arc<dyn BrainRuntime>) -> Self {
        Self { runtime }
    }
}

pub async fn build_default_app() -> Result<BackendApp, acp::Error> {
    let store: Arc<dyn Store> =
        Arc::new(FileStore::new(brain_home()).await.map_err(internal_error)?);

    let pool = Arc::new(CredentialPool::new(
        store.clone(),
        Arc::new(Fallback::new()),
    ));
    let (default_provider_name, providers) = discover_providers(pool).await;
    let runtime = Arc::new(BrainRuntimeNative::new(
        store,
        default_provider_name,
        "simple",
    ));
    for (provider_name, provider) in providers {
        runtime
            .set_provider(provider_name, provider)
            .map_err(internal_error)?;
    }
    runtime
        .set_loop("simple", Arc::new(SimpleLoop))
        .map_err(internal_error)?;
    runtime
        .set_loop("robust", Arc::new(RobustLoop))
        .map_err(internal_error)?;
    runtime
        .set_loop("terminus2", Arc::new(Terminus2Loop))
        .map_err(internal_error)?;
    runtime
        .set_loop("terminus-kira", Arc::new(TerminusKiraLoop))
        .map_err(internal_error)?;
    for tool in native_tools() {
        let name = tool.definition().name.clone();
        runtime.set_tool(name, tool).map_err(internal_error)?;
    }
    runtime
        .set_tool("file_read", Arc::new(FileReadTool::new(FileReadDriverAcp)))
        .map_err(internal_error)?;
    runtime
        .set_tool(
            "file_write",
            Arc::new(FileWriteTool::new(FileWriteDriverAcp)),
        )
        .map_err(internal_error)?;

    let runtime: Arc<dyn BrainRuntime> = runtime;
    Ok(BackendApp::new(runtime))
}

async fn discover_providers(
    pool: Arc<CredentialPool>,
) -> (String, Vec<(String, Arc<dyn Provider>)>) {
    let mut providers: Vec<(String, Arc<dyn Provider>, bool)> = Vec::new();

    discover_api_providers("openai", None, &pool, &mut providers).await;
    discover_oauth_providers("openai", None, &pool, &mut providers).await;
    discover_local_providers("openai", None, &mut providers).await;

    if providers.is_empty() {
        tracing::warn!("no providers configured — falling back to MockProvider for ACP backend");
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
    let default = providers
        .iter()
        .find(|(_, _, is_default)| *is_default)
        .or(providers.first())
        .map(|(_, provider, _)| provider.clone())
        .expect("providers should not be empty");
    let mut registered = vec![(default_name.clone(), default)];
    for (provider_name, provider, _) in providers {
        if provider_name == default_name {
            continue;
        }
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

    let provider: Arc<dyn Provider> =
        Arc::new(OpenAiOAuthProvider::with_pool(pool.clone(), preset));
    let is_default = preset.name == default_name;
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

#[cfg(feature = "mistralrs")]
async fn discover_local_providers(
    default_name: &str,
    _model_override: Option<&str>,
    providers: &mut Vec<(String, Arc<dyn Provider>, bool)>,
) {
    let default_model = "qwen3-0.6b".to_owned();
    let configs: Vec<(String, MistralRsConfig)> = MistralRsModelPreset::ALL
        .iter()
        .map(|preset| (preset.name.to_owned(), preset.into_config()))
        .collect();

    let provider = MistralRsProvider::new(configs, &default_model);
    let is_default = default_name == "mistralrs";
    providers.push(("mistralrs".to_owned(), Arc::new(provider), is_default));
}

#[cfg(all(feature = "llamacpp", not(feature = "mistralrs")))]
async fn discover_local_providers(
    default_name: &str,
    _model_override: Option<&str>,
    providers: &mut Vec<(String, Arc<dyn Provider>, bool)>,
) {
    let default_model = "qwen3.5-0.8b".to_owned();
    let configs: Vec<(String, LlamaCppConfig)> = LlamaCppModelPreset::ALL
        .iter()
        .map(|preset| (preset.name.to_owned(), preset.into_config()))
        .collect();

    let Ok(provider) = LlamaCppProvider::new(configs, &default_model) else {
        tracing::warn!("failed to initialize llama.cpp backend");
        return;
    };

    let is_default = default_name == "llamacpp";
    providers.push(("llamacpp".to_owned(), Arc::new(provider), is_default));
}

#[cfg(not(any(feature = "mistralrs", feature = "llamacpp")))]
async fn discover_local_providers(
    _default_name: &str,
    _model_override: Option<&str>,
    _providers: &mut Vec<(String, Arc<dyn Provider>, bool)>,
) {
}
