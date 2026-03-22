use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AtifConfig {
    pub emit_events: bool,
    pub schema_version: atif::SchemaVersion,
}

impl Default for AtifConfig {
    fn default() -> Self {
        Self {
            emit_events: false,
            schema_version: atif::SchemaVersion::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DoomLoopStrategy {
    #[default]
    Steer,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentConfig {
    pub max_iterations: u32,
    pub system_prompt: Option<String>,
    pub loop_name: Option<String>,
    pub atif: AtifConfig,
    pub tool_output_max_bytes: usize,
    pub doom_loop_threshold: u32,
    pub doom_loop_strategy: DoomLoopStrategy,
    pub compaction_threshold: Option<f32>,
    pub compaction_model: Option<String>,
    pub max_retries: u32,
    pub inference: InferenceConfig,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_iterations: 20,
            system_prompt: None,
            loop_name: None,
            atif: AtifConfig::default(),
            tool_output_max_bytes: 16_000,
            doom_loop_threshold: 3,
            doom_loop_strategy: DoomLoopStrategy::Steer,
            compaction_threshold: Some(0.8),
            compaction_model: None,
            max_retries: 3,
            inference: InferenceConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct InferenceConfig {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub reasoning: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            provider: None,
            model: None,
            reasoning: None,
            max_tokens: None,
            temperature: None,
        }
    }
}

impl InferenceConfig {
    pub fn is_empty(&self) -> bool {
        self.provider.is_none()
            && self.model.is_none()
            && self.reasoning.is_none()
            && self.max_tokens.is_none()
            && self.temperature.is_none()
    }

    pub fn merged_with(&self, overrides: &Self) -> Self {
        Self {
            provider: overrides.provider.clone().or_else(|| self.provider.clone()),
            model: overrides.model.clone().or_else(|| self.model.clone()),
            reasoning: overrides
                .reasoning
                .clone()
                .or_else(|| self.reasoning.clone()),
            max_tokens: overrides.max_tokens.or(self.max_tokens),
            temperature: overrides.temperature.or(self.temperature),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: Ulid,
    pub message_count: u32,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt: u32,
    pub completion: u32,
    pub total: u32,
    #[serde(default)]
    pub cache_read: Option<u32>,
    #[serde(default)]
    pub cache_write: Option<u32>,
    #[serde(default)]
    pub reasoning: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_config_defaults() {
        let cfg = AgentConfig::default();
        assert_eq!(cfg.max_iterations, 20);
        assert!(cfg.system_prompt.is_none());
        assert!(cfg.loop_name.is_none());
        assert!(!cfg.atif.emit_events);
        assert_eq!(cfg.atif.schema_version, atif::SchemaVersion::V1_6);
        assert_eq!(cfg.tool_output_max_bytes, 16_000);
        assert_eq!(cfg.doom_loop_threshold, 3);
        assert_eq!(cfg.doom_loop_strategy, DoomLoopStrategy::Steer);
        assert_eq!(cfg.compaction_threshold, Some(0.8));
        assert!(cfg.compaction_model.is_none());
        assert_eq!(cfg.max_retries, 3);
        assert!(cfg.inference.model.is_none());
    }

    #[test]
    fn inference_config_defaults() {
        let cfg = InferenceConfig::default();
        assert!(cfg.provider.is_none());
        assert!(cfg.model.is_none());
        assert!(cfg.reasoning.is_none());
        assert!(cfg.max_tokens.is_none());
        assert!(cfg.temperature.is_none());
    }

    #[test]
    fn inference_config_is_empty_when_all_fields_missing() {
        assert!(InferenceConfig::default().is_empty());
    }

    #[test]
    fn inference_config_merged_with_overrides_only_replaces_present_fields() {
        let defaults = InferenceConfig {
            provider: Some("openai".into()),
            model: Some("gpt-4o-mini".into()),
            reasoning: Some("medium".into()),
            max_tokens: Some(4096),
            temperature: Some(0.7),
        };
        let overrides = InferenceConfig {
            provider: None,
            model: Some("gpt-5".into()),
            reasoning: None,
            max_tokens: None,
            temperature: Some(0.2),
        };

        let merged = defaults.merged_with(&overrides);
        assert_eq!(merged.provider.as_deref(), Some("openai"));
        assert_eq!(merged.model.as_deref(), Some("gpt-5"));
        assert_eq!(merged.reasoning.as_deref(), Some("medium"));
        assert_eq!(merged.max_tokens, Some(4096));
        assert_eq!(merged.temperature, Some(0.2));
    }

    #[test]
    fn agent_config_serde_roundtrip() {
        let cfg = AgentConfig {
            max_iterations: 10,
            system_prompt: Some("You are helpful.".into()),
            loop_name: Some("simple".into()),
            atif: AtifConfig {
                emit_events: true,
                schema_version: atif::SchemaVersion::V1_6,
            },
            tool_output_max_bytes: 24_000,
            doom_loop_threshold: 4,
            doom_loop_strategy: DoomLoopStrategy::Error,
            compaction_threshold: Some(0.9),
            compaction_model: Some("gpt-5-mini".into()),
            max_retries: 5,
            inference: InferenceConfig {
                provider: Some("openai".into()),
                model: Some("gpt-4".into()),
                reasoning: Some("low".into()),
                max_tokens: Some(4096),
                temperature: Some(0.7),
            },
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let deserialized: AgentConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.max_iterations, 10);
        assert_eq!(
            deserialized.system_prompt.as_deref(),
            Some("You are helpful.")
        );
        assert_eq!(deserialized.loop_name.as_deref(), Some("simple"));
        assert!(deserialized.atif.emit_events);
        assert_eq!(deserialized.atif.schema_version, atif::SchemaVersion::V1_6);
        assert_eq!(deserialized.tool_output_max_bytes, 24_000);
        assert_eq!(deserialized.doom_loop_threshold, 4);
        assert_eq!(deserialized.doom_loop_strategy, DoomLoopStrategy::Error);
        assert_eq!(deserialized.compaction_threshold, Some(0.9));
        assert_eq!(deserialized.compaction_model.as_deref(), Some("gpt-5-mini"));
        assert_eq!(deserialized.max_retries, 5);
        assert_eq!(deserialized.inference.model.as_deref(), Some("gpt-4"));
        assert_eq!(deserialized.inference.reasoning.as_deref(), Some("low"));
    }

    #[test]
    fn agent_config_uses_defaults_for_missing_hardening_fields() {
        let cfg: AgentConfig = serde_json::from_str(
            r#"{
                "max_iterations": 7,
                "inference": {
                    "model": "gpt-5"
                }
            }"#,
        )
        .unwrap();

        assert_eq!(cfg.max_iterations, 7);
        assert!(!cfg.atif.emit_events);
        assert_eq!(cfg.tool_output_max_bytes, 16_000);
        assert_eq!(cfg.doom_loop_threshold, 3);
        assert_eq!(cfg.doom_loop_strategy, DoomLoopStrategy::Steer);
        assert_eq!(cfg.compaction_threshold, Some(0.8));
        assert!(cfg.compaction_model.is_none());
        assert_eq!(cfg.max_retries, 3);
        assert_eq!(cfg.inference.model.as_deref(), Some("gpt-5"));
    }

    #[test]
    fn token_usage_serde() {
        let usage = TokenUsage {
            prompt: 100,
            completion: 50,
            total: 150,
            cache_read: Some(25),
            cache_write: None,
            reasoning: Some(10),
        };
        let json = serde_json::to_string(&usage).unwrap();
        let deserialized: TokenUsage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.total, 150);
        assert_eq!(deserialized.cache_read, Some(25));
        assert_eq!(deserialized.reasoning, Some(10));
    }

    #[test]
    fn session_summary_serde() {
        let summary = SessionSummary {
            id: Ulid::new(),
            message_count: 42,
            updated_at: Utc::now(),
        };
        let json = serde_json::to_string(&summary).unwrap();
        let deserialized: SessionSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.message_count, 42);
    }
}
