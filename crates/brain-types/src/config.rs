use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub max_iterations: u32,
    pub system_prompt: Option<String>,
    pub inference: InferenceConfig,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_iterations: 20,
            system_prompt: None,
            inference: InferenceConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceConfig {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            provider: None,
            model: None,
            max_tokens: None,
            temperature: None,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_config_defaults() {
        let cfg = AgentConfig::default();
        assert_eq!(cfg.max_iterations, 20);
        assert!(cfg.system_prompt.is_none());
        assert!(cfg.inference.model.is_none());
    }

    #[test]
    fn inference_config_defaults() {
        let cfg = InferenceConfig::default();
        assert!(cfg.provider.is_none());
        assert!(cfg.model.is_none());
        assert!(cfg.max_tokens.is_none());
        assert!(cfg.temperature.is_none());
    }

    #[test]
    fn agent_config_serde_roundtrip() {
        let cfg = AgentConfig {
            max_iterations: 10,
            system_prompt: Some("You are helpful.".into()),
            inference: InferenceConfig {
                provider: Some("openai".into()),
                model: Some("gpt-4".into()),
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
        assert_eq!(deserialized.inference.model.as_deref(), Some("gpt-4"));
    }

    #[test]
    fn token_usage_serde() {
        let usage = TokenUsage {
            prompt: 100,
            completion: 50,
            total: 150,
        };
        let json = serde_json::to_string(&usage).unwrap();
        let deserialized: TokenUsage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.total, 150);
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
