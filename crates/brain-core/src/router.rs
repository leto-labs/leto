use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use futures::future::BoxFuture;

use brain_types::*;

struct RegisteredProvider {
    provider: Arc<dyn Provider>,
    info: ProviderInfo,
}

/// Routes `chat()` calls to registered providers using explicit provider selection first,
/// then falling back to unambiguous model ownership, and finally the configured default provider.
pub struct ProviderRouter {
    providers: HashMap<String, RegisteredProvider>,
    provider_order: Vec<String>,
    model_routes: HashMap<String, String>,
    ambiguous_models: HashSet<String>,
    default_provider: String,
}

impl ProviderRouter {
    pub fn new(default_name: impl Into<String>, default: Arc<dyn Provider>) -> Self {
        let default_name = default_name.into();
        let mut router = Self {
            providers: HashMap::new(),
            provider_order: Vec::new(),
            model_routes: HashMap::new(),
            ambiguous_models: HashSet::new(),
            default_provider: default_name.clone(),
        };
        router.insert_provider(default_name, default);
        router
    }

    pub fn add_provider(mut self, name: impl Into<String>, provider: Arc<dyn Provider>) -> Self {
        self.insert_provider(name.into(), provider);
        self
    }

    fn insert_provider(&mut self, name: String, provider: Arc<dyn Provider>) {
        let info = provider.info();

        for model in &info.models {
            let model_id = model.id.to_owned();
            if let Some(existing) = self.model_routes.get(&model_id) {
                if existing != &name {
                    self.model_routes.remove(&model_id);
                    self.ambiguous_models.insert(model_id);
                }
            } else if !self.ambiguous_models.contains(&model_id) {
                self.model_routes.insert(model_id, name.clone());
            }
        }

        if !self.providers.contains_key(&name) {
            self.provider_order.push(name.clone());
        }

        self.providers
            .insert(name, RegisteredProvider { provider, info });
    }

    pub fn default_provider_name(&self) -> &str {
        &self.default_provider
    }

    pub fn provider_infos(&self) -> Vec<ProviderInfo> {
        let mut providers = self
            .providers
            .values()
            .map(|provider| provider.info.clone())
            .collect::<Vec<_>>();
        providers.sort_by(|left, right| left.name.cmp(&right.name));
        providers
    }

    pub fn available_models(&self) -> Vec<ProviderModelInfo> {
        let mut seen = HashSet::new();
        let mut models = Vec::new();

        for provider_name in &self.provider_order {
            let Some(provider) = self.providers.get(provider_name) else {
                continue;
            };

            for model in &provider.info.models {
                if self.ambiguous_models.contains(model.id) || !seen.insert(model.id.to_owned()) {
                    continue;
                }
                models.push(ProviderModelInfo {
                    provider: provider_name.clone(),
                    model: *model,
                });
            }
        }

        models
    }

    pub fn resolve_model(&self, model_id: &str) -> Result<ResolvedModel, BrainError> {
        let provider_name = self.model_routes.get(model_id).cloned().ok_or_else(|| {
            BrainError::Internal(format!("unknown or ambiguous model: {model_id}"))
        })?;
        let provider = self.providers.get(&provider_name).ok_or_else(|| {
            BrainError::Internal(format!(
                "provider not found for model {model_id}: {provider_name}"
            ))
        })?;
        let model = provider
            .info
            .models
            .iter()
            .find(|model| model.id == model_id)
            .copied()
            .ok_or_else(|| BrainError::Internal(format!("model metadata not found: {model_id}")))?;

        Ok(ResolvedModel {
            provider: provider_name,
            model,
        })
    }

    pub fn current_model_id(&self, config: &InferenceConfig) -> Result<Option<String>, BrainError> {
        let provider_name = self.resolve_provider_name(config)?;
        let provider = self.providers.get(&provider_name).ok_or_else(|| {
            BrainError::Internal(format!("provider not registered: {provider_name}"))
        })?;

        Ok(config
            .model
            .clone()
            .or_else(|| provider.info.default_model.clone()))
    }

    fn resolve_provider_name(&self, config: &InferenceConfig) -> Result<String, BrainError> {
        if let Some(provider_name) = config.provider.as_deref() {
            let provider = self.providers.get(provider_name).ok_or_else(|| {
                BrainError::Internal(format!("unknown provider: {provider_name}"))
            })?;

            if let Some(model_id) = config.model.as_deref()
                && !provider.info.models.is_empty()
                && !provider
                    .info
                    .models
                    .iter()
                    .any(|model| model.id == model_id)
            {
                return Err(BrainError::Internal(format!(
                    "model {model_id} is not available for provider {provider_name}"
                )));
            }

            return Ok(provider_name.to_owned());
        }

        if let Some(model_id) = config.model.as_deref() {
            return self.model_routes.get(model_id).cloned().ok_or_else(|| {
                BrainError::Internal(format!("unknown or ambiguous model: {model_id}"))
            });
        }

        Ok(self.default_provider.clone())
    }

    fn resolve_provider(&self, config: &InferenceConfig) -> Result<&Arc<dyn Provider>, BrainError> {
        let provider_name = self.resolve_provider_name(config)?;
        self.providers
            .get(&provider_name)
            .map(|provider| &provider.provider)
            .ok_or_else(|| {
                BrainError::Internal(format!("provider not registered: {provider_name}"))
            })
    }
}

pub struct ResolvedModel {
    pub provider: String,
    pub model: ModelInfo,
}

impl Provider for ProviderRouter {
    fn info(&self) -> ProviderInfo {
        let default_provider = self
            .providers
            .get(&self.default_provider)
            .expect("default provider should always be registered");

        ProviderInfo {
            name: "router".into(),
            default_model: default_provider.info.default_model.clone(),
            models: self
                .available_models()
                .into_iter()
                .map(|model| model.model)
                .collect(),
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
            let provider = self.resolve_provider(config)?;
            provider.chat(messages, tools, config, session_id).await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brain_providers::MockProvider;
    use futures::StreamExt;

    struct NamedMockProvider {
        name: &'static str,
        model: &'static str,
        inner: MockProvider,
    }

    impl NamedMockProvider {
        fn new(name: &'static str, model: &'static str) -> Self {
            Self {
                name,
                model,
                inner: MockProvider::new(),
            }
        }
    }

    impl Provider for NamedMockProvider {
        fn info(&self) -> ProviderInfo {
            ProviderInfo {
                name: self.name.into(),
                default_model: Some(self.model.into()),
                models: vec![ModelInfo {
                    id: self.model,
                    name: self.model,
                    family: None,
                    reasoning: None,
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
                }],
            }
        }

        fn chat<'a>(
            &'a self,
            messages: &'a [Message],
            tools: &'a [ToolDef],
            config: &'a InferenceConfig,
            session_id: Option<ulid::Ulid>,
        ) -> BoxFuture<'a, Result<ChatStream, BrainError>> {
            self.inner.chat(messages, tools, config, session_id)
        }
    }

    struct MultiModelProvider {
        name: &'static str,
        default_model: &'static str,
        models: &'static [&'static str],
        inner: MockProvider,
    }

    impl MultiModelProvider {
        fn new(
            name: &'static str,
            default_model: &'static str,
            models: &'static [&'static str],
        ) -> Self {
            Self {
                name,
                default_model,
                models,
                inner: MockProvider::new(),
            }
        }
    }

    impl Provider for MultiModelProvider {
        fn info(&self) -> ProviderInfo {
            ProviderInfo {
                name: self.name.into(),
                default_model: Some(self.default_model.into()),
                models: self
                    .models
                    .iter()
                    .map(|model| ModelInfo {
                        id: model,
                        name: model,
                        family: None,
                        reasoning: None,
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
                    })
                    .collect(),
            }
        }

        fn chat<'a>(
            &'a self,
            messages: &'a [Message],
            tools: &'a [ToolDef],
            config: &'a InferenceConfig,
            session_id: Option<ulid::Ulid>,
        ) -> BoxFuture<'a, Result<ChatStream, BrainError>> {
            self.inner.chat(messages, tools, config, session_id)
        }
    }

    #[tokio::test]
    async fn routes_to_default_when_no_provider_or_model() {
        let default = Arc::new(MockProvider::new());
        let router = ProviderRouter::new("mock", default);

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
    async fn resolves_provider_by_model_when_unambiguous() {
        let default = Arc::new(MockProvider::new());
        let special = Arc::new(NamedMockProvider::new("special", "special-model"));
        let router = ProviderRouter::new("mock", default).add_provider("special", special);

        let resolved = router.resolve_model("special-model").unwrap();
        assert_eq!(resolved.provider, "special");

        let resolved_provider = router
            .resolve_provider_name(&InferenceConfig {
                provider: None,
                model: Some("special-model".into()),
                reasoning: None,
                max_tokens: None,
                temperature: None,
            })
            .unwrap();
        assert_eq!(resolved_provider, "special");
    }

    #[tokio::test]
    async fn explicit_provider_must_exist() {
        let default = Arc::new(MockProvider::new());
        let router = ProviderRouter::new("mock", default);

        let error = router
            .resolve_provider_name(&InferenceConfig {
                provider: Some("missing".into()),
                model: None,
                reasoning: None,
                max_tokens: None,
                temperature: None,
            })
            .unwrap_err();

        assert!(error.to_string().contains("unknown provider"));
    }

    #[tokio::test]
    async fn current_model_defaults_from_selected_provider() {
        let default = Arc::new(MockProvider::new());
        let router = ProviderRouter::new("mock", default);

        let current = router
            .current_model_id(&InferenceConfig::default())
            .unwrap();
        assert_eq!(current.as_deref(), Some("mock-echo"));
    }

    #[tokio::test]
    async fn available_models_preserves_provider_declared_order() {
        let default = Arc::new(MultiModelProvider::new(
            "default",
            "gpt-5.4",
            &["gpt-5.4", "gpt-5.2", "gpt-5.1"],
        ));
        let extra = Arc::new(MultiModelProvider::new(
            "extra",
            "llama-3.3-70b-versatile",
            &["llama-3.3-70b-versatile", "llama-3.1-8b-instant"],
        ));
        let router = ProviderRouter::new("default", default).add_provider("extra", extra);

        let models = router
            .available_models()
            .into_iter()
            .map(|model| model.model.id)
            .collect::<Vec<_>>();

        assert_eq!(
            models,
            vec![
                "gpt-5.4",
                "gpt-5.2",
                "gpt-5.1",
                "llama-3.3-70b-versatile",
                "llama-3.1-8b-instant",
            ]
        );
    }
}
