use super::config::MistralRsConfig;

#[derive(Debug, Clone, Copy)]
pub struct MistralRsModelPreset {
    pub name: &'static str,
    pub model_id: &'static str,
    pub gguf_files: &'static [&'static str],
}

impl MistralRsModelPreset {
    pub const QWEN3_0_6B: Self = Self {
        name: "qwen3-0.6b",
        model_id: "Qwen/Qwen3-0.6B-GGUF",
        gguf_files: &["qwen3-0.6b-q4_k_m.gguf"],
    };
    pub const QWEN3_1_7B: Self = Self {
        name: "qwen3-1.7b",
        model_id: "Qwen/Qwen3-1.7B-GGUF",
        gguf_files: &["qwen3-1.7b-q4_k_m.gguf"],
    };
    pub const QWEN3_4B: Self = Self {
        name: "qwen3-4b",
        model_id: "Qwen/Qwen3-4B-GGUF",
        gguf_files: &["qwen3-4b-q4_k_m.gguf"],
    };

    pub const ALL: &[Self] = &[Self::QWEN3_0_6B, Self::QWEN3_1_7B, Self::QWEN3_4B];

    pub fn by_name(name: &str) -> Option<Self> {
        let lower = name.to_ascii_lowercase();
        Self::ALL.iter().find(|p| p.name == lower).copied()
    }

    pub fn into_config(self) -> MistralRsConfig {
        MistralRsConfig::new(
            self.model_id,
            self.gguf_files.iter().map(|s| s.to_string()).collect(),
        )
    }
}
