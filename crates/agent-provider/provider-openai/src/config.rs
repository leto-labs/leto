//! Configuration for the OpenAI wire client.

use std::collections::BTreeMap;

use provider::ModelInfo;

use crate::presets::OpenAiConfigPreset;

/// Supported modern OpenAI-compatible API surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenAiApiSurface {
    /// The Responses API surface.
    Responses,
    /// The Chat Completions API surface.
    ChatCompletions,
}

/// Selection mode for choosing an OpenAI-compatible API surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenAiApiMode {
    /// Automatically select the best supported surface.
    Auto,
    /// Force the Responses API surface.
    Responses,
    /// Force the Chat Completions API surface.
    ChatCompletions,
}

/// Runtime configuration for [`crate::Client`] and [`crate::OpenAiProvider`].
#[derive(Debug, Clone)]
pub struct Config {
    /// Stable provider/display name.
    pub name: String,
    /// API key sent as a bearer token.
    pub api_key: String,
    /// Base REST URL, defaulting to `https://api.openai.com/v1`.
    pub base_url: String,
    /// Model used when requests omit `model`.
    pub default_model: String,
    /// Advertised model catalog for this provider configuration.
    pub models: &'static [ModelInfo],
    /// Supported modern API surfaces for this provider configuration.
    pub supported_api_surfaces: &'static [OpenAiApiSurface],
    /// Surface selection mode used by higher-level integrations.
    pub api_surface_mode: OpenAiApiMode,
    /// Extra headers added to every outbound request.
    pub default_headers: BTreeMap<String, String>,
}

impl Config {
    /// Creates a configuration with the default OpenAI base URL and model.
    pub fn new(api_key: impl Into<String>) -> Self {
        OpenAiConfigPreset::OPENAI.into_config(api_key)
    }

    /// Overrides the bearer token or API key.
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = api_key.into();
        self
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

    /// Overrides the provider/display name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Overrides the API surface selection mode.
    pub fn with_api_surface_mode(mut self, mode: OpenAiApiMode) -> Self {
        self.api_surface_mode = mode;
        self
    }

    /// Adds or replaces a default outbound header.
    pub fn with_default_header(
        mut self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.default_headers.insert(name.into(), value.into());
        self
    }

    /// Returns true when the configuration supports the requested surface.
    pub fn supports_api_surface(&self, surface: OpenAiApiSurface) -> bool {
        self.supported_api_surfaces.contains(&surface)
    }

    /// Resolves the effective API surface according to the configured mode.
    pub fn resolved_api_surface(&self) -> Option<OpenAiApiSurface> {
        match self.api_surface_mode {
            OpenAiApiMode::Auto => {
                if self.supports_api_surface(OpenAiApiSurface::Responses) {
                    Some(OpenAiApiSurface::Responses)
                } else if self.supports_api_surface(OpenAiApiSurface::ChatCompletions) {
                    Some(OpenAiApiSurface::ChatCompletions)
                } else {
                    None
                }
            }
            OpenAiApiMode::Responses => self
                .supports_api_surface(OpenAiApiSurface::Responses)
                .then_some(OpenAiApiSurface::Responses),
            OpenAiApiMode::ChatCompletions => self
                .supports_api_surface(OpenAiApiSurface::ChatCompletions)
                .then_some(OpenAiApiSurface::ChatCompletions),
        }
    }
}
