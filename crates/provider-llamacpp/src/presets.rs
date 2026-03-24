use std::borrow::Cow;

use provider::{ModelInfo, ProviderCapabilities, StreamGranularity};

use super::config::LlamaCppConfig;

const fn llama_capabilities(input_image_urls: bool) -> ProviderCapabilities {
    ProviderCapabilities {
        system_messages: true,
        developer_messages: true,
        input_text: true,
        input_image_urls,
        tool_calls: false,
        tool_results: false,
        reasoning_blocks: true,
        refusal_blocks: false,
        tool_call_argument_deltas: false,
        parallel_tool_calls: false,
        stream_granularity: StreamGranularity::Block,
    }
}

/// Backend-specific artifacts and runtime defaults for a llama.cpp preset.
#[derive(Debug, Clone, Copy)]
pub struct LlamaCppArtifacts {
    /// Hugging Face repo id or local directory used to resolve preset files.
    pub repo_id: &'static str,
    /// Optional Hugging Face revision or snapshot pin for reproducible downloads.
    pub revision: Option<&'static str>,
    /// GGUF filename for the preset weights.
    pub gguf_filename: &'static str,
    /// Optional multimodal projector filename required to enable vision input.
    pub mmproj_filename: Option<&'static str>,
    /// GGUF architecture reported by the upstream Hub metadata.
    pub architecture: &'static str,
    /// Maximum context length reported by the upstream GGUF metadata.
    pub context_length: u32,
    /// Optional number of layers to offload to GPU for this preset.
    pub n_gpu_layers: Option<u32>,
    /// Default runtime context size used when building a config from the preset.
    pub default_context_size: u32,
}

/// Curated llama.cpp preset combining shared model metadata and local artifacts.
#[derive(Debug, Clone)]
pub struct LlamaCppModelPreset {
    /// Shared provider-facing model metadata for the preset.
    pub model_info: ModelInfo,
    /// llama.cpp-specific artifact and runtime details for the preset.
    pub artifacts: LlamaCppArtifacts,
}

impl LlamaCppModelPreset {
    /// Built-in Qwen3.5 0.8B GGUF preset.
    pub const QWEN35_0_8B: Self = Self {
        model_info: ModelInfo {
            id: Cow::Borrowed("qwen3.5-0.8b"),
            name: Cow::Borrowed("Qwen3.5 0.8B"),
            family: Some(Cow::Borrowed("qwen3.5")),
            reasoning_efforts: Cow::Borrowed(&[]),
            tool_call: false,
            attachment: true,
            structured_output: None,
            temperature: Some(true),
            knowledge: None,
            release_date: Some(Cow::Borrowed("2026-03-01")),
            last_updated: Some(Cow::Borrowed("2026-03-02")),
            open_weights: Some(true),
            input_modalities: Cow::Borrowed(&["text", "image"]),
            output_modalities: Cow::Borrowed(&["text"]),
            cost: None,
            limit: None,
            status: None,
            capabilities: Some(llama_capabilities(true)),
        },
        artifacts: LlamaCppArtifacts {
            repo_id: "unsloth/Qwen3.5-0.8B-GGUF",
            revision: Some("6ab461498e2023f6e3c1baea90a8f0fe38ab64d0"),
            gguf_filename: "Qwen3.5-0.8B-Q4_K_M.gguf",
            mmproj_filename: Some("mmproj-F16.gguf"),
            architecture: "qwen35",
            context_length: 262_144,
            n_gpu_layers: None,
            default_context_size: 4096,
        },
    };
    /// Built-in Qwen3.5 2B GGUF preset.
    pub const QWEN35_2B: Self = Self {
        model_info: ModelInfo {
            id: Cow::Borrowed("qwen3.5-2b"),
            name: Cow::Borrowed("Qwen3.5 2B"),
            family: Some(Cow::Borrowed("qwen3.5")),
            reasoning_efforts: Cow::Borrowed(&[]),
            tool_call: false,
            attachment: true,
            structured_output: None,
            temperature: Some(true),
            knowledge: None,
            release_date: Some(Cow::Borrowed("2026-02-28")),
            last_updated: Some(Cow::Borrowed("2026-03-02")),
            open_weights: Some(true),
            input_modalities: Cow::Borrowed(&["text", "image"]),
            output_modalities: Cow::Borrowed(&["text"]),
            cost: None,
            limit: None,
            status: None,
            capabilities: Some(llama_capabilities(true)),
        },
        artifacts: LlamaCppArtifacts {
            repo_id: "unsloth/Qwen3.5-2B-GGUF",
            revision: Some("f6d5376be1edb4d416d56da11e5397a961aca8ae"),
            gguf_filename: "Qwen3.5-2B-Q4_K_M.gguf",
            mmproj_filename: Some("mmproj-F16.gguf"),
            architecture: "qwen35",
            context_length: 262_144,
            n_gpu_layers: None,
            default_context_size: 4096,
        },
    };

    /// All built-in llama.cpp presets exposed by this crate.
    pub const ALL: &[Self] = &[Self::QWEN35_0_8B, Self::QWEN35_2B];

    /// Returns the shared stable model identifier for this preset.
    pub fn id(&self) -> &str {
        self.model_info.id.as_ref()
    }

    /// Resolves a built-in preset by its stable model identifier.
    pub fn by_name(name: &str) -> Option<Self> {
        let lower = name.to_ascii_lowercase();
        Self::ALL
            .iter()
            .find(|preset| preset.id() == lower)
            .cloned()
    }

    /// Converts the preset into a runtime config carrying its metadata forward.
    pub fn into_config(self) -> LlamaCppConfig {
        self.into_config_with_vision(true)
    }

    /// Converts the preset into a text-only runtime config by disabling vision artifacts.
    pub fn into_text_only_config(self) -> LlamaCppConfig {
        self.into_config_with_vision(false)
    }

    /// Converts the preset into a runtime config and optionally enables vision.
    pub fn into_config_with_vision(self, enable_vision: bool) -> LlamaCppConfig {
        let Self {
            mut model_info,
            artifacts,
        } = self;
        model_info.attachment = enable_vision && artifacts.mmproj_filename.is_some();
        model_info.input_modalities = if model_info.attachment {
            Cow::Borrowed(&["text", "image"])
        } else {
            Cow::Borrowed(&["text"])
        };
        model_info.capabilities = Some(llama_capabilities(model_info.attachment));

        let mut config = LlamaCppConfig::new(artifacts.repo_id, artifacts.gguf_filename)
            .with_context_size(artifacts.default_context_size);
        if enable_vision {
            if let Some(mmproj_filename) = artifacts.mmproj_filename {
                config = config.with_mmproj_file(mmproj_filename);
            }
        }
        if let Some(n_gpu_layers) = artifacts.n_gpu_layers {
            config = config.with_gpu_layers(n_gpu_layers);
        }
        if let Some(capabilities) = model_info.capabilities.clone() {
            config = config.with_capabilities(capabilities);
        }
        config.with_model_info(model_info)
    }
}

#[cfg(test)]
mod tests {
    use super::LlamaCppModelPreset;

    #[test]
    fn preset_lookup_works() {
        assert!(LlamaCppModelPreset::by_name("qwen3.5-0.8b").is_some());
        assert!(LlamaCppModelPreset::by_name("qwen3.5-2b").is_some());
        assert!(LlamaCppModelPreset::by_name("missing").is_none());
    }

    #[test]
    fn into_config_preserves_model_metadata() {
        let config = LlamaCppModelPreset::QWEN35_0_8B.into_config();
        let model_info = config
            .model_info
            .expect("preset should populate model info");
        let capabilities = config
            .capabilities
            .expect("preset should populate capabilities");

        assert_eq!(model_info.id.as_ref(), "qwen3.5-0.8b");
        assert_eq!(model_info.name.as_ref(), "Qwen3.5 0.8B");
        assert_eq!(config.context_size, 4096);
        assert_eq!(config.mmproj_file.as_deref(), Some("mmproj-F16.gguf"));
        assert_eq!(
            LlamaCppModelPreset::QWEN35_0_8B.artifacts.repo_id,
            "unsloth/Qwen3.5-0.8B-GGUF"
        );
        assert_eq!(
            LlamaCppModelPreset::QWEN35_0_8B.artifacts.revision,
            Some("6ab461498e2023f6e3c1baea90a8f0fe38ab64d0")
        );
        assert_eq!(
            LlamaCppModelPreset::QWEN35_0_8B.artifacts.gguf_filename,
            "Qwen3.5-0.8B-Q4_K_M.gguf"
        );
        assert_eq!(
            LlamaCppModelPreset::QWEN35_0_8B.artifacts.mmproj_filename,
            Some("mmproj-F16.gguf")
        );
        assert_eq!(
            LlamaCppModelPreset::QWEN35_0_8B.artifacts.architecture,
            "qwen35"
        );
        assert_eq!(
            LlamaCppModelPreset::QWEN35_0_8B.artifacts.context_length,
            262_144
        );
        assert!(!capabilities.tool_calls);
        assert!(capabilities.reasoning_blocks);
        assert!(model_info.attachment);
        assert_eq!(model_info.input_modalities.as_ref(), ["text", "image"]);
        assert!(capabilities.input_image_urls);
    }

    #[test]
    fn into_text_only_config_disables_projector_and_image_metadata() {
        let config = LlamaCppModelPreset::QWEN35_0_8B.into_text_only_config();
        let model_info = config
            .model_info
            .expect("text-only preset should populate model info");
        let capabilities = config
            .capabilities
            .expect("text-only preset should populate capabilities");

        assert_eq!(config.mmproj_file, None);
        assert!(!model_info.attachment);
        assert_eq!(model_info.input_modalities.as_ref(), ["text"]);
        assert!(!capabilities.input_image_urls);
    }
}
