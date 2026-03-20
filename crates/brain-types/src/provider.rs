use std::pin::Pin;

use futures::Stream;
use futures::future::BoxFuture;
use serde::Serialize;

use ulid::Ulid;

use crate::{BrainError, ChatChunk, InferenceConfig, Message, ModelInfo, ToolDef};

pub type ChatStream = Pin<Box<dyn Stream<Item = Result<ChatChunk, BrainError>> + Send>>;

/// Metadata about a provider instance, returned by `Provider::info()`.
#[derive(Debug, Clone, Serialize)]
pub struct ProviderInfo {
    pub name: String,
    pub default_model: Option<String>,
    pub models: Vec<ModelInfo>,
}

/// Provider-qualified model metadata used by runtime-level merged model listings.
#[derive(Debug, Clone, Serialize)]
pub struct ProviderModelInfo {
    pub provider: String,
    pub model: ModelInfo,
}

pub trait Provider: Send + Sync {
    fn chat<'a>(
        &'a self,
        messages: &'a [Message],
        tools: &'a [ToolDef],
        config: &'a InferenceConfig,
        session_id: Option<Ulid>,
    ) -> BoxFuture<'a, Result<ChatStream, BrainError>>;

    fn info(&self) -> ProviderInfo;
}
