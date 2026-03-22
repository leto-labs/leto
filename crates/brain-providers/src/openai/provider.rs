use std::sync::Arc;

use futures::future::BoxFuture;

use brain_types::*;

use super::config::{OpenAiApiSurface, OpenAiConfig};
use crate::codex_sse::{
    build_responses_request, stream_from_response as stream_responses_from_response,
};
use crate::openai_sse::*;
use crate::pool::CredentialPool;

pub struct OpenAiProvider {
    config: OpenAiConfig,
    pool: Option<Arc<CredentialPool>>,
    client: reqwest::Client,
}

impl OpenAiProvider {
    pub fn new(config: OpenAiConfig) -> Self {
        Self {
            config,
            pool: None,
            client: reqwest::Client::new(),
        }
    }

    pub fn with_pool(config: OpenAiConfig, pool: Arc<CredentialPool>) -> Self {
        Self {
            config,
            pool: Some(pool),
            client: reqwest::Client::new(),
        }
    }

    async fn resolve_token(&self, session_id: Option<ulid::Ulid>) -> Result<String, BrainError> {
        if let Some(pool) = &self.pool {
            let entry = pool.resolve(&self.config.name, session_id).await?;
            match &entry.credential {
                ProviderCredential::ApiKey { api_key } => Ok(api_key.clone()),
                ProviderCredential::OAuth(oauth) => Ok(oauth.access_token.clone()),
            }
        } else {
            Ok(self.config.api_key.clone())
        }
    }
}

impl Provider for OpenAiProvider {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: self.config.name.clone(),
            default_model: Some(self.config.default_model.clone()),
            models: self.config.models.to_vec(),
        }
    }

    fn chat<'a>(
        &'a self,
        messages: &'a [Message],
        tools: &'a [ToolDef],
        config: &'a InferenceConfig,
        session_id: Option<ulid::Ulid>,
    ) -> BoxFuture<'a, Result<ChatStream, BrainError>> {
        Box::pin(async move {
            let token = self.resolve_token(session_id).await?;

            let model = config
                .model
                .as_deref()
                .unwrap_or(&self.config.default_model)
                .to_string();

            let api_surface = self.config.resolved_api_surface().ok_or_else(|| {
                BrainError::Internal(format!(
                    "provider '{}' does not support the requested API surface",
                    self.config.name
                ))
            })?;

            let response = match api_surface {
                OpenAiApiSurface::Responses => {
                    let body = build_responses_request(model, messages, tools, config);
                    let url = format!("{}/responses", self.config.base_url.trim_end_matches('/'));

                    self.client
                        .post(&url)
                        .header("Authorization", format!("Bearer {token}"))
                        .header("originator", "brain")
                        .header("accept", "text/event-stream")
                        .json(&body)
                        .send()
                        .await
                        .map_err(|e| BrainError::Inference(e.to_string()))?
                }
                OpenAiApiSurface::ChatCompletions => {
                    let body = ChatCompletionRequest {
                        model,
                        messages: messages.iter().map(to_oai_message).collect(),
                        tools: tools.iter().map(to_oai_tool).collect(),
                        stream: true,
                        stream_options: Some(StreamOptions {
                            include_usage: true,
                        }),
                        max_tokens: config.max_tokens,
                        temperature: config.temperature,
                    };

                    let url = format!("{}/chat/completions", self.config.base_url);

                    self.client
                        .post(&url)
                        .header("Authorization", format!("Bearer {token}"))
                        .json(&body)
                        .send()
                        .await
                        .map_err(|e| BrainError::Inference(e.to_string()))?
                }
            };

            if !response.status().is_success() {
                let status = response.status();
                let text = response.text().await.unwrap_or_default();
                return Err(BrainError::Inference(format!("{status}: {text}")));
            }

            Ok(match api_surface {
                OpenAiApiSurface::Responses => stream_responses_from_response(response),
                OpenAiApiSurface::ChatCompletions => stream_from_response(response),
            })
        })
    }
}
