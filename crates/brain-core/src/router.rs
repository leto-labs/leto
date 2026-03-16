use std::collections::HashMap;
use std::sync::Arc;

use futures::future::BoxFuture;

use brain_types::*;

/// Routes `chat()` calls to different providers based on `InferenceConfig.model`.
///
/// Unknown model names and `None` both fall back to the default provider.
pub struct ProviderRouter {
    routes: HashMap<String, Arc<dyn Provider>>,
    default: Arc<dyn Provider>,
}

impl ProviderRouter {
    pub fn new(default: Arc<dyn Provider>) -> Self {
        Self {
            routes: HashMap::new(),
            default,
        }
    }

    /// Register a provider under one or more model names.
    pub fn add(mut self, name: impl Into<String>, provider: Arc<dyn Provider>) -> Self {
        self.routes.insert(name.into(), provider);
        self
    }

    fn resolve(&self, model: Option<&str>) -> &Arc<dyn Provider> {
        match model {
            Some(name) => self.routes.get(name).unwrap_or(&self.default),
            None => &self.default,
        }
    }
}

impl Provider for ProviderRouter {
    fn info(&self) -> ProviderInfo {
        let mut all_models: Vec<ProviderModelInfo> = Vec::new();

        let default_info = self.default.info();
        let default_model = default_info.default_model.clone();
        all_models.extend(default_info.models);

        for (_route_name, provider) in &self.routes {
            let info = provider.info();
            for model in info.models {
                if !all_models.iter().any(|m| m.id == model.id) {
                    all_models.push(model);
                }
            }
        }

        ProviderInfo {
            name: "router".into(),
            default_model,
            models: all_models,
        }
    }

    fn chat<'a>(
        &'a self,
        messages: &'a [Message],
        tools: &'a [ToolDef],
        config: &'a InferenceConfig,
        session_id: Option<ulid::Ulid>,
    ) -> BoxFuture<'a, Result<ChatStream, BrainError>> {
        let provider = self.resolve(config.model.as_deref());
        provider.chat(messages, tools, config, session_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brain_providers::MockProvider;
    use futures::StreamExt;

    #[tokio::test]
    async fn routes_to_default_when_no_model() {
        let default = Arc::new(MockProvider::new());
        let router = ProviderRouter::new(default);

        let msgs = [Message::user("hello")];
        let config = InferenceConfig::default();
        let mut stream = router.chat(&msgs, &[], &config, None).await.unwrap();

        let mut text = String::new();
        while let Some(chunk) = stream.next().await {
            if let ChatChunk::Delta { content } = chunk.unwrap() {
                text.push_str(&content);
            }
        }
        assert_eq!(text.trim(), "hello");
    }

    #[tokio::test]
    async fn routes_to_registered_provider() {
        let default = Arc::new(MockProvider::new());
        let special = Arc::new(MockProvider::new());
        let router = ProviderRouter::new(default).add("special-model", special);

        let msgs = [Message::user("routed")];
        let config = InferenceConfig {
            model: Some("special-model".into()),
            ..InferenceConfig::default()
        };
        let mut stream = router.chat(&msgs, &[], &config, None).await.unwrap();

        let mut text = String::new();
        while let Some(chunk) = stream.next().await {
            if let ChatChunk::Delta { content } = chunk.unwrap() {
                text.push_str(&content);
            }
        }
        assert_eq!(text.trim(), "routed");
    }

    #[tokio::test]
    async fn unknown_model_falls_back_to_default() {
        let default = Arc::new(MockProvider::new());
        let router = ProviderRouter::new(default);

        let msgs = [Message::user("fallback")];
        let config = InferenceConfig {
            model: Some("nonexistent".into()),
            ..InferenceConfig::default()
        };
        let mut stream = router.chat(&msgs, &[], &config, None).await.unwrap();

        let mut text = String::new();
        while let Some(chunk) = stream.next().await {
            if let ChatChunk::Delta { content } = chunk.unwrap() {
                text.push_str(&content);
            }
        }
        assert_eq!(text.trim(), "fallback");
    }
}
