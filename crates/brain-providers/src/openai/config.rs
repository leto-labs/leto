use brain_types::ModelInfo;

use super::presets::OpenAiConfigPreset;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenAiApiSurface {
    Responses,
    ChatCompletions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenAiApiMode {
    Auto,
    Responses,
    ChatCompletions,
}

/// Configuration for the OpenAI-compatible provider.
/// The caller owns this data -- the library never reads env vars.
#[derive(Debug, Clone)]
pub struct OpenAiConfig {
    pub name: String,
    pub api_key: String,
    pub base_url: String,
    pub default_model: String,
    pub models: &'static [ModelInfo],
    pub supported_api_surfaces: &'static [OpenAiApiSurface],
    pub api_surface_mode: OpenAiApiMode,
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

    pub fn with_api_surface_mode(mut self, mode: OpenAiApiMode) -> Self {
        self.api_surface_mode = mode;
        self
    }

    pub fn supports_api_surface(&self, surface: OpenAiApiSurface) -> bool {
        self.supported_api_surfaces.contains(&surface)
    }

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
