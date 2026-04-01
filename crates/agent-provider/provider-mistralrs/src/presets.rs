use std::borrow::Cow;

use provider::{ModelInfo, ProviderCapabilities, StreamGranularity};

use super::config::MistralRsConfig;

const fn mistralrs_capabilities() -> ProviderCapabilities {
    ProviderCapabilities {
        system_messages: true,
        developer_messages: true,
        input_text: true,
        input_image_urls: false,
        tool_calls: false,
        tool_results: false,
        reasoning_blocks: true,
        refusal_blocks: false,
        tool_call_argument_deltas: false,
        parallel_tool_calls: false,
        stream_granularity: StreamGranularity::Block,
    }
}

/// Backend-specific artifacts for a mistral.rs preset.
#[derive(Debug, Clone, Copy)]
pub struct MistralRsArtifacts {
    /// Hugging Face repo id or local directory used to resolve preset files.
    pub repo_id: &'static str,
    /// Optional Hugging Face revision or snapshot pin for reproducible downloads.
    pub revision: Option<&'static str>,
    /// GGUF filenames that mistral.rs should load for the preset.
    pub gguf_filenames: &'static [&'static str],
    /// GGUF architecture reported by the upstream Hub metadata.
    pub architecture: &'static str,
    /// Maximum context length reported by the upstream GGUF metadata.
    pub context_length: u32,
}

/// Curated mistral.rs preset combining shared model metadata and local artifacts.
#[derive(Debug, Clone)]
pub struct MistralRsModelPreset {
    /// Shared provider-facing model metadata for the preset.
    pub model_info: ModelInfo,
    /// mistral.rs-specific artifact details for the preset.
    pub artifacts: MistralRsArtifacts,
}

impl MistralRsModelPreset {
    /// Built-in Qwen3 0.6B GGUF preset.
    pub const QWEN3_0_6B: Self = Self {
        model_info: ModelInfo {
            id: Cow::Borrowed("qwen3-0.6b"),
            name: Cow::Borrowed("Qwen3 0.6B"),
            family: Some(Cow::Borrowed("qwen3")),
            reasoning_efforts: Cow::Borrowed(&[]),
            tool_call: false,
            attachment: false,
            structured_output: None,
            temperature: Some(true),
            knowledge: None,
            release_date: Some(Cow::Borrowed("2025-04-28")),
            last_updated: Some(Cow::Borrowed("2025-06-23")),
            open_weights: Some(true),
            input_modalities: Cow::Borrowed(&["text"]),
            output_modalities: Cow::Borrowed(&["text"]),
            cost: None,
            limit: None,
            status: None,
            capabilities: Some(mistralrs_capabilities()),
        },
        artifacts: MistralRsArtifacts {
            repo_id: "unsloth/Qwen3-0.6B-GGUF",
            revision: Some("50968a4468ef4233ed78cd7c3de230dd1d61a56b"),
            gguf_filenames: &["Qwen3-0.6B-Q4_K_M.gguf"],
            architecture: "qwen3",
            context_length: 40_960,
        },
    };
    /// Built-in Qwen3 1.7B GGUF preset.
    pub const QWEN3_1_7B: Self = Self {
        model_info: ModelInfo {
            id: Cow::Borrowed("qwen3-1.7b"),
            name: Cow::Borrowed("Qwen3 1.7B"),
            family: Some(Cow::Borrowed("qwen3")),
            reasoning_efforts: Cow::Borrowed(&[]),
            tool_call: false,
            attachment: false,
            structured_output: None,
            temperature: Some(true),
            knowledge: None,
            release_date: Some(Cow::Borrowed("2025-04-28")),
            last_updated: Some(Cow::Borrowed("2025-06-08")),
            open_weights: Some(true),
            input_modalities: Cow::Borrowed(&["text"]),
            output_modalities: Cow::Borrowed(&["text"]),
            cost: None,
            limit: None,
            status: None,
            capabilities: Some(mistralrs_capabilities()),
        },
        artifacts: MistralRsArtifacts {
            repo_id: "unsloth/Qwen3-1.7B-GGUF",
            revision: Some("d7f544eead698dbd1f15126ef60b45a1e1933222"),
            gguf_filenames: &["Qwen3-1.7B-Q4_K_M.gguf"],
            architecture: "qwen3",
            context_length: 40_960,
        },
    };
    /// Built-in Qwen3 4B GGUF preset.
    pub const QWEN3_4B: Self = Self {
        model_info: ModelInfo {
            id: Cow::Borrowed("qwen3-4b"),
            name: Cow::Borrowed("Qwen3 4B"),
            family: Some(Cow::Borrowed("qwen3")),
            reasoning_efforts: Cow::Borrowed(&[]),
            tool_call: false,
            attachment: false,
            structured_output: None,
            temperature: Some(true),
            knowledge: None,
            release_date: Some(Cow::Borrowed("2025-05-05")),
            last_updated: Some(Cow::Borrowed("2025-05-21")),
            open_weights: Some(true),
            input_modalities: Cow::Borrowed(&["text"]),
            output_modalities: Cow::Borrowed(&["text"]),
            cost: None,
            limit: None,
            status: None,
            capabilities: Some(mistralrs_capabilities()),
        },
        artifacts: MistralRsArtifacts {
            repo_id: "Qwen/Qwen3-4B-GGUF",
            revision: Some("bc640142c66e1fdd12af0bd68f40445458f3869b"),
            gguf_filenames: &["Qwen3-4B-Q4_K_M.gguf"],
            architecture: "qwen3",
            context_length: 40_960,
        },
    };

    /// All built-in mistral.rs presets exposed by this crate.
    pub const ALL: &[Self] = &[Self::QWEN3_0_6B, Self::QWEN3_1_7B, Self::QWEN3_4B];

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
    pub fn into_config(self) -> MistralRsConfig {
        let Self {
            model_info,
            artifacts,
        } = self;
        let mut config = MistralRsConfig::new(
            artifacts.repo_id,
            artifacts
                .gguf_filenames
                .iter()
                .map(|value| value.to_string())
                .collect(),
        );
        if let Some(capabilities) = model_info.capabilities.clone() {
            config = config.with_capabilities(capabilities);
        }
        config.with_model_info(model_info)
    }
}

#[cfg(test)]
mod tests {
    use super::MistralRsModelPreset;

    #[test]
    fn preset_lookup_is_case_insensitive() {
        let preset = MistralRsModelPreset::by_name("QwEn3-0.6B")
            .expect("mixed-case preset names should resolve");

        assert_eq!(preset.id(), "qwen3-0.6b");
    }

    #[test]
    fn preset_lookup_works() {
        assert!(MistralRsModelPreset::by_name("qwen3-0.6b").is_some());
        assert!(MistralRsModelPreset::by_name("qwen3-1.7b").is_some());
        assert!(MistralRsModelPreset::by_name("qwen3-4b").is_some());
        assert!(MistralRsModelPreset::by_name("missing").is_none());
    }

    #[test]
    fn into_config_preserves_model_metadata() {
        let config = MistralRsModelPreset::QWEN3_0_6B.into_config();
        let model_info = config
            .model_info
            .expect("preset should populate model info");
        let capabilities = config
            .capabilities
            .expect("preset should populate capabilities");

        assert_eq!(model_info.id.as_ref(), "qwen3-0.6b");
        assert_eq!(model_info.name.as_ref(), "Qwen3 0.6B");
        assert_eq!(config.gguf_files, vec!["Qwen3-0.6B-Q4_K_M.gguf"]);
        assert_eq!(
            MistralRsModelPreset::QWEN3_0_6B.artifacts.repo_id,
            "unsloth/Qwen3-0.6B-GGUF"
        );
        assert_eq!(
            MistralRsModelPreset::QWEN3_0_6B.artifacts.revision,
            Some("50968a4468ef4233ed78cd7c3de230dd1d61a56b")
        );
        assert_eq!(
            MistralRsModelPreset::QWEN3_0_6B.artifacts.gguf_filenames,
            &["Qwen3-0.6B-Q4_K_M.gguf"]
        );
        assert_eq!(
            MistralRsModelPreset::QWEN3_0_6B.artifacts.architecture,
            "qwen3"
        );
        assert_eq!(
            MistralRsModelPreset::QWEN3_0_6B.artifacts.context_length,
            40_960
        );
        assert!(!capabilities.tool_calls);
        assert!(capabilities.reasoning_blocks);
        assert_eq!(model_info.input_modalities.as_ref(), ["text"]);
    }
}
