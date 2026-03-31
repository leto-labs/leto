//! Embedded runtime-backed ACP launch helpers.

use std::path::PathBuf;
use std::sync::Arc;

use agent_core::{AgentCoreNative, CoreError};
use agent_store::{FileStore, Store};
use anyhow::{Context, Result};
use dirs::home_dir;
use provider::MockProvider;
use provider_openai::{OpenAiApiMode, OpenAiConfigPreset, OpenAiProvider};

use crate::file_bridge::{AcpFileBridge, tool_executor, wrap_provider};
use crate::run_stdio;

/// Runs the embedded runtime-backed ACP stdio server using local store state.
pub async fn run_embedded_stdio() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .try_init();

    let store: Arc<dyn Store> = Arc::new(
        FileStore::new(resolve_agent_home())
            .await
            .context("failed to initialize file store")?,
    );
    let core = build_embedded_core(store)
        .await
        .context("failed to initialize embedded AgentCore")?;
    run_stdio(core)
        .await
        .context("failed to run ACP stdio server")
}

fn resolve_agent_home() -> PathBuf {
    std::env::var_os("AGENT_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("BRAIN_HOME").map(PathBuf::from))
        .unwrap_or_else(|| {
            home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".brain")
        })
}

async fn build_embedded_core(store: Arc<dyn Store>) -> Result<AgentCoreNative, CoreError> {
    if let Some(core) = build_env_override_core(store.clone()).await? {
        return Ok(core);
    }

    match AgentCoreNative::build_default_local(store.clone()).await {
        Ok(core) => Ok(core),
        Err(CoreError::NoProvidersRegistered) => Ok(AgentCoreNative::builder(store)
            .with_provider("mock", wrap_provider(Arc::new(MockProvider::new())))
            .with_tools(tool_executor(AcpFileBridge::new()))
            .default_provider("mock")
            .build()
            .await?),
        Err(error) => Err(error),
    }
}

async fn build_env_override_core(
    store: Arc<dyn Store>,
) -> Result<Option<AgentCoreNative>, CoreError> {
    let Some(provider_name) = std::env::var("AGENT_PROVIDER").ok() else {
        return Ok(None);
    };

    let model = std::env::var("AGENT_MODEL").map_err(|_| {
        CoreError::Internal("AGENT_MODEL is required when AGENT_PROVIDER is set".to_owned())
    })?;

    if provider_name == "mock" {
        return Ok(Some(
            AgentCoreNative::builder(store)
                .without_credential_discovery()
                .with_provider("mock", wrap_provider(Arc::new(MockProvider::new())))
                .with_tools(tool_executor(AcpFileBridge::new()))
                .default_provider("mock")
                .default_loop(
                    std::env::var("AGENT_DEFAULT_LOOP").unwrap_or_else(|_| "simple".to_owned()),
                )
                .build()
                .await?,
        ));
    }

    let api_key = std::env::var("AGENT_API_KEY").map_err(|_| {
        CoreError::Internal(format!(
            "AGENT_API_KEY is required when AGENT_PROVIDER is set to '{provider_name}'"
        ))
    })?;

    let preset = OpenAiConfigPreset::ALL
        .iter()
        .find(|preset| preset.name == provider_name)
        .ok_or_else(|| CoreError::ProviderNotRegistered(provider_name.clone()))?;

    let mut config = preset.into_config(api_key).with_model(&model);
    if let Ok(base_url) = std::env::var("AGENT_BASE_URL") {
        if !base_url.is_empty() {
            config = config.with_base_url(base_url);
        }
    }
    if let Ok(api_surface) = std::env::var("AGENT_API_SURFACE") {
        if !api_surface.is_empty() {
            config = config.with_api_surface_mode(parse_api_surface_mode(&api_surface)?);
        }
    }

    Ok(Some(
        AgentCoreNative::builder(store)
            .without_credential_discovery()
            .with_provider(
                provider_name.clone(),
                wrap_provider(Arc::new(OpenAiProvider::new(config))),
            )
            .with_tools(tool_executor(AcpFileBridge::new()))
            .default_provider(provider_name)
            .default_loop(
                std::env::var("AGENT_DEFAULT_LOOP").unwrap_or_else(|_| "simple".to_owned()),
            )
            .build()
            .await?,
    ))
}

fn parse_api_surface_mode(value: &str) -> Result<OpenAiApiMode, CoreError> {
    match value {
        "auto" => Ok(OpenAiApiMode::Auto),
        "responses" => Ok(OpenAiApiMode::Responses),
        "chat-completions" | "chat_completions" => Ok(OpenAiApiMode::ChatCompletions),
        other => Err(CoreError::Internal(format!(
            "unsupported api surface '{other}' (expected auto, responses, or chat-completions)"
        ))),
    }
}
