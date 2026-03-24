use provider::{ModelInfo, ProviderCapabilities};

/// Device-selection policy for mistral.rs model loading.
#[derive(Debug, Clone, Default)]
pub enum DevicePreference {
    /// Let mistral.rs choose the most appropriate device automatically.
    #[default]
    Auto,
    /// Force model execution onto the CPU.
    Cpu,
}

/// Runtime configuration for a single mistral.rs-backed model entry.
#[derive(Debug, Clone)]
pub struct MistralRsConfig {
    /// Hugging Face repo id or local directory used to resolve GGUF artifacts.
    pub model_id: String,
    /// GGUF filenames to load for this model entry.
    pub gguf_files: Vec<String>,
    /// Preferred execution device for the loaded model.
    pub device: DevicePreference,
    /// Optional shared provider-facing metadata for this configured model.
    pub model_info: Option<ModelInfo>,
    /// Optional shared capability overrides for this configured model.
    pub capabilities: Option<ProviderCapabilities>,
}

impl MistralRsConfig {
    /// Creates a config from a model source identifier and GGUF file list.
    pub fn new(model_id: impl Into<String>, gguf_files: Vec<String>) -> Self {
        Self {
            model_id: model_id.into(),
            gguf_files,
            device: DevicePreference::default(),
            model_info: None,
            capabilities: None,
        }
    }

    /// Overrides the default automatic device selection policy.
    pub fn with_device(mut self, device: DevicePreference) -> Self {
        self.device = device;
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
