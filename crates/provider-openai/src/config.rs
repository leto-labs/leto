//! Configuration for the OpenAI wire client.

/// Runtime configuration for [`crate::Client`] and [`crate::OpenAiProvider`].
#[derive(Debug, Clone)]
pub struct Config {
    /// API key sent as a bearer token.
    pub api_key: String,
    /// Base REST URL, defaulting to `https://api.openai.com/v1`.
    pub base_url: String,
    /// Model used when requests omit `model`.
    pub default_model: String,
}

impl Config {
    /// Creates a configuration with the default OpenAI base URL and model.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: "https://api.openai.com/v1".into(),
            default_model: "gpt-5.4-mini".into(),
        }
    }

    /// Overrides the base URL.
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Overrides the default model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }
}
