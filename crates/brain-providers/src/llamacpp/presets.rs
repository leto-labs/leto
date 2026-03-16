use super::config::LlamaCppConfig;

#[derive(Debug, Clone, Copy)]
pub struct LlamaCppModelPreset {
    pub name: &'static str,
    pub model_id: &'static str,
    pub gguf_file: &'static str,
}

impl LlamaCppModelPreset {
    pub const QWEN35_0_8B: Self = Self {
        name: "qwen3.5-0.8b",
        model_id: "unsloth/Qwen3.5-0.8B-GGUF",
        gguf_file: "Qwen3.5-0.8B-Q4_K_M.gguf",
    };
    pub const QWEN35_2B: Self = Self {
        name: "qwen3.5-2b",
        model_id: "unsloth/Qwen3.5-2B-GGUF",
        gguf_file: "Qwen3.5-2B-Q4_K_M.gguf",
    };

    pub const ALL: &[Self] = &[Self::QWEN35_0_8B, Self::QWEN35_2B];

    pub fn by_name(name: &str) -> Option<Self> {
        let lower = name.to_ascii_lowercase();
        Self::ALL.iter().find(|p| p.name == lower).copied()
    }

    pub fn into_config(self) -> LlamaCppConfig {
        LlamaCppConfig::new(self.model_id, self.gguf_file)
    }
}
