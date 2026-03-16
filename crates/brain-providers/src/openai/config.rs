use brain_types::ModelInfo;

use super::presets::OpenAiConfigPreset;

/// Configuration for the OpenAI-compatible provider.
/// The caller owns this data -- the library never reads env vars.
#[derive(Debug, Clone)]
pub struct OpenAiConfig {
    pub name: String,
    pub api_key: String,
    pub base_url: String,
    pub default_model: String,
    pub models: &'static [ModelInfo],
}

impl OpenAiConfig {
    /// Creates a config with OpenAI defaults. Use `OpenAiConfigPreset` for other providers.
    pub fn new(api_key: impl Into<String>) -> Self {
        OpenAiConfigPreset::OPENAI.into_config(api_key)
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }
}
