/// Configuration for a single llama.cpp model.
///
/// Models are identified by a HuggingFace repo ID and GGUF filename,
/// just like `MistralRsConfig`. On first use the GGUF file is downloaded
/// from HuggingFace and cached locally at `~/.cache/huggingface/hub/`.
#[derive(Debug, Clone)]
pub struct LlamaCppConfig {
    /// HuggingFace repo ID (e.g. `"unsloth/Qwen3.5-2B-GGUF"`) or local directory path.
    pub model_id: String,
    /// GGUF filename inside the repo (e.g. `"Qwen3.5-2B-Q4_K_M.gguf"`).
    pub gguf_file: String,
    /// Number of layers to offload to GPU. `None` keeps all on CPU.
    pub n_gpu_layers: Option<u32>,
    /// Context window size in tokens.
    pub context_size: u32,
}

impl LlamaCppConfig {
    pub fn new(model_id: impl Into<String>, gguf_file: impl Into<String>) -> Self {
        Self {
            model_id: model_id.into(),
            gguf_file: gguf_file.into(),
            n_gpu_layers: None,
            context_size: 4096,
        }
    }

    pub fn with_gpu_layers(mut self, n: u32) -> Self {
        self.n_gpu_layers = Some(n);
        self
    }

    pub fn with_context_size(mut self, size: u32) -> Self {
        self.context_size = size;
        self
    }
}
