use provider::{ModelInfo, ProviderCapabilities};

/// Runtime configuration for a single llama.cpp-backed model entry.
#[derive(Debug, Clone)]
pub struct LlamaCppConfig {
    /// Hugging Face repo id or local directory used to resolve model artifacts.
    pub model_id: String,
    /// Primary GGUF filename to load from `model_id`.
    pub gguf_file: String,
    /// Optional multimodal projector filename required for vision-capable models.
    pub mmproj_file: Option<String>,
    /// Optional number of layers to offload to GPU when supported by the host.
    pub n_gpu_layers: Option<u32>,
    /// Context window requested when building llama.cpp contexts.
    pub context_size: u32,
    /// Optional shared provider-facing metadata for this configured model.
    pub model_info: Option<ModelInfo>,
    /// Optional shared capability overrides for this configured model.
    pub capabilities: Option<ProviderCapabilities>,
}

impl LlamaCppConfig {
    /// Creates a config from a model source identifier and GGUF filename.
    pub fn new(model_id: impl Into<String>, gguf_file: impl Into<String>) -> Self {
        Self {
            model_id: model_id.into(),
            gguf_file: gguf_file.into(),
            mmproj_file: None,
            n_gpu_layers: None,
            context_size: 4096,
            model_info: None,
            capabilities: None,
        }
    }

    /// Attaches an optional multimodal projector file to the config.
    pub fn with_mmproj_file(mut self, mmproj_file: impl Into<String>) -> Self {
        self.mmproj_file = Some(mmproj_file.into());
        self
    }

    /// Sets the number of layers to offload to GPU.
    pub fn with_gpu_layers(mut self, n: u32) -> Self {
        self.n_gpu_layers = Some(n);
        self
    }

    /// Overrides the default llama.cpp context size.
    pub fn with_context_size(mut self, size: u32) -> Self {
        self.context_size = size;
        self
    }

    /// Stores shared model catalog metadata for this config entry.
    pub fn with_model_info(mut self, model_info: ModelInfo) -> Self {
        self.model_info = Some(model_info);
        self
    }

    /// Stores shared capability metadata for this config entry.
    pub fn with_capabilities(mut self, capabilities: ProviderCapabilities) -> Self {
        self.capabilities = Some(capabilities);
        self
    }
}
