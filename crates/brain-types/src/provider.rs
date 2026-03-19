use std::pin::Pin;

use futures::Stream;
use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};

use ulid::Ulid;

use crate::{BrainError, ChatChunk, InferenceConfig, Message, ModelInfo, ToolDef};

pub type ChatStream = Pin<Box<dyn Stream<Item = Result<ChatChunk, BrainError>> + Send>>;

/// Metadata about a provider instance, returned by `Provider::info()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub name: String,
    pub default_model: Option<String>,
    pub models: Vec<ProviderModelInfo>,
}

/// Lightweight serializable model summary derived from `ModelInfo`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderModelInfo {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    pub reasoning: bool,
    pub tool_call: bool,
}

impl From<&ModelInfo> for ProviderModelInfo {
    fn from(m: &ModelInfo) -> Self {
        Self {
            id: m.id.to_owned(),
            name: m.name.to_owned(),
            provider: None,
            reasoning: m.reasoning.is_some(),
            tool_call: m.tool_call,
        }
    }
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
