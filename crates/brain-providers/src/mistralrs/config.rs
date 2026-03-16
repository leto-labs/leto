#[derive(Debug, Clone)]
pub enum DevicePreference {
    Auto,
    Cpu,
}

impl Default for DevicePreference {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Clone)]
pub struct MistralRsConfig {
    pub model_id: String,
    pub gguf_files: Vec<String>,
    pub device: DevicePreference,
}

impl MistralRsConfig {
    pub fn new(model_id: impl Into<String>, gguf_files: Vec<String>) -> Self {
        Self {
            model_id: model_id.into(),
            gguf_files,
            device: DevicePreference::default(),
        }
    }

    pub fn with_device(mut self, device: DevicePreference) -> Self {
        self.device = device;
        self
    }
}
