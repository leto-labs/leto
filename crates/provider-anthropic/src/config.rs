//! Configuration for the Anthropic wire client.

/// Runtime configuration for [`crate::Client`] and [`crate::AnthropicProvider`].
#[derive(Debug, Clone)]
pub struct Config {
    /// API key sent via the `x-api-key` header.
    pub api_key: String,
    /// Base REST URL, defaulting to `https://api.anthropic.com`.
    pub base_url: String,
    /// Default model used when requests omit `model`.
    pub default_model: String,
    /// Anthropic API version sent in the `anthropic-version` header.
    pub version: String,
}

impl Config {
    /// Creates a configuration with Anthropic defaults.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: "https://api.anthropic.com".into(),
            default_model: "claude-haiku-4-5".into(),
            version: "2023-06-01".into(),
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

    /// Overrides the requested Anthropic API version.
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }
}
