use std::collections::HashMap;
use std::sync::Arc;

use async_stream::stream;
use futures::future::BoxFuture;
use tokio::sync::RwLock;

use brain_types::*;
use mistralrs::{
    GgufModelBuilder, Model, Response, TextMessageRole, TextMessages,
};

use super::config::{DevicePreference, MistralRsConfig};

/// In-process LLM provider backed by mistral.rs.
///
/// Supports multiple models with lazy loading: models are downloaded/loaded
/// into memory on first use and cached for subsequent requests.
pub struct MistralRsProvider {
    configs: HashMap<String, MistralRsConfig>,
    models: RwLock<HashMap<String, Arc<Model>>>,
    default_model: String,
}

impl MistralRsProvider {
    pub fn new(
        configs: Vec<(String, MistralRsConfig)>,
        default_model: impl Into<String>,
    ) -> Self {
        Self {
            configs: configs.into_iter().collect(),
            models: RwLock::new(HashMap::new()),
            default_model: default_model.into(),
        }
    }

    /// Register an additional model config. It will be loaded lazily on first use.
    pub fn add(mut self, name: impl Into<String>, config: MistralRsConfig) -> Self {
        self.configs.insert(name.into(), config);
        self
    }

    /// Pre-load a registered model so the first `chat()` call doesn't pay
    /// the download/init cost. Returns an error if the name is unknown or
    /// the model fails to load.
    pub async fn preload(&self, name: &str) -> Result<(), BrainError> {
        self.resolve_model(name).await.map(|_| ())
    }

    async fn resolve_model(&self, name: &str) -> Result<Arc<Model>, BrainError> {
        {
            let cache = self.models.read().await;
            if let Some(model) = cache.get(name) {
                return Ok(model.clone());
            }
        }

        let config = self.configs.get(name).ok_or_else(|| {
            BrainError::Inference(format!("unknown model: {name}"))
        })?;

        let model = load_model(config).await?;
        let model = Arc::new(model);

        let mut cache = self.models.write().await;
        cache.entry(name.to_owned()).or_insert_with(|| model.clone());
        Ok(model)
    }
}

async fn load_model(config: &MistralRsConfig) -> Result<Model, BrainError> {
    let mut builder = GgufModelBuilder::new(
        config.model_id.clone(),
        config.gguf_files.clone(),
    );

    if matches!(config.device, DevicePreference::Cpu) {
        builder = builder.with_force_cpu();
    }

    builder
        .build()
        .await
        .map_err(|e| BrainError::Inference(format!("failed to load model: {e}")))
}

fn to_mistral_role(role: &Role) -> TextMessageRole {
    match role {
        Role::System => TextMessageRole::System,
        Role::User => TextMessageRole::User,
        Role::Assistant => TextMessageRole::Assistant,
        Role::Tool => TextMessageRole::Tool,
    }
}

impl Provider for MistralRsProvider {
    fn info(&self) -> ProviderInfo {
        let models = self.configs.keys().map(|name| ProviderModelInfo {
            id: name.clone(),
            name: name.clone(),
            reasoning: false,
            tool_call: false,
        }).collect();
        ProviderInfo {
            name: "mistralrs".into(),
            default_model: Some(self.default_model.clone()),
            models,
        }
    }

    fn chat<'a>(
        &'a self,
        messages: &'a [Message],
        _tools: &'a [ToolDef],
        config: &'a InferenceConfig,
        _session_id: Option<ulid::Ulid>,
    ) -> BoxFuture<'a, Result<ChatStream, BrainError>> {
        let model_name = config
            .model
            .as_deref()
            .unwrap_or(&self.default_model)
            .to_owned();

        let mut text_messages = TextMessages::new();
        for msg in messages {
            text_messages = text_messages.add_message(
                to_mistral_role(&msg.role),
                &msg.content,
            );
        }

        Box::pin(async move {
            let model = self.resolve_model(&model_name).await?;

            let s = stream! {
                let stream = model
                    .stream_chat_request(text_messages)
                    .await;

                let mut stream = match stream {
                    Ok(s) => s,
                    Err(e) => {
                        yield Err(BrainError::Inference(format!("stream request failed: {e}")));
                        return;
                    }
                };

                while let Some(chunk) = stream.next().await {
                    match chunk {
                        Response::Chunk(c) => {
                            if let Some(choice) = c.choices.first() {
                                if let Some(ref content) = choice.delta.content {
                                    if !content.is_empty() {
                                        yield Ok(ChatChunk::Delta {
                                            content: content.clone(),
                                        });
                                    }
                                }
                            }
                        }
                        Response::Done(d) => {
                            let p = d.usage.prompt_tokens as u32;
                            let c = d.usage.completion_tokens as u32;
                            yield Ok(ChatChunk::Done {
                                usage: Some(TokenUsage {
                                    prompt: p,
                                    completion: c,
                                    total: p + c,
                                }),
                            });
                        }
                        Response::ModelError(msg, _) => {
                            yield Err(BrainError::Inference(msg));
                        }
                        Response::InternalError(e) => {
                            yield Err(BrainError::Inference(e.to_string()));
                        }
                        Response::ValidationError(e) => {
                            yield Err(BrainError::Inference(e.to_string()));
                        }
                        _ => {}
                    }
                }
            };

            Ok(Box::pin(s) as ChatStream)
        })
    }
}
