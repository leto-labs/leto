use chrono::DateTime;
use serde::{Deserialize, Serialize};

use crate::{
    JsonObject, MessageContent, Metrics, Observation, SchemaVersion, ToolCall, ValidationError,
    require_supported,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StepSource {
    System,
    User,
    Agent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ReasoningEffort {
    Label(String),
    Numeric(f64),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub step_id: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    pub source: StepSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<ReasoningEffort>,
    pub message: MessageContent,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observation: Option<Observation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metrics: Option<Metrics>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_copied_context: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra: Option<JsonObject>,
}

impl Step {
    pub(crate) fn validate(&self, schema_version: SchemaVersion) -> Result<(), ValidationError> {
        if self.step_id == 0 {
            return Err(ValidationError::InvalidStepId(self.step_id));
        }

        if let Some(timestamp) = &self.timestamp {
            DateTime::parse_from_rfc3339(timestamp).map_err(|_| {
                ValidationError::InvalidTimestamp {
                    timestamp: timestamp.clone(),
                }
            })?;
        }

        if self.source != StepSource::Agent
            && (self.model_name.is_some()
                || self.reasoning_effort.is_some()
                || self.reasoning_content.is_some()
                || self.tool_calls.is_some()
                || self.metrics.is_some())
        {
            return Err(ValidationError::AgentOnlyFieldOnNonAgentStep {
                step_id: self.step_id,
            });
        }

        if self.observation.is_some() && self.source == StepSource::System {
            require_supported(
                schema_version.supports_system_step_observation(),
                "steps[].observation",
                SchemaVersion::V1_2,
                schema_version,
            )?;
        }
        if self.is_copied_context.is_some() {
            require_supported(
                schema_version.supports_copied_context(),
                "steps[].is_copied_context",
                SchemaVersion::V1_5,
                schema_version,
            )?;
        }

        self.message.validate(schema_version, "steps[].message")?;
        if let Some(observation) = &self.observation {
            observation.validate(schema_version)?;
        }
        if let Some(metrics) = &self.metrics {
            metrics.validate(schema_version)?;
        }

        Ok(())
    }

    pub fn has_multimodal_content(&self) -> bool {
        self.message.has_multimodal_content()
            || self
                .observation
                .as_ref()
                .is_some_and(Observation::has_multimodal_content)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Observation, ObservationResult, SchemaVersion, Step, StepSource, ValidationError};

    #[test]
    fn validation_rejects_agent_only_fields_on_user_steps() {
        let step = Step {
            step_id: 1,
            timestamp: None,
            source: StepSource::User,
            model_name: Some("gpt-5.4".into()),
            reasoning_effort: None,
            message: "hello".into(),
            reasoning_content: None,
            tool_calls: None,
            observation: None,
            metrics: None,
            is_copied_context: None,
            extra: None,
        };

        assert!(matches!(
            step.validate(SchemaVersion::V1_6),
            Err(ValidationError::AgentOnlyFieldOnNonAgentStep { step_id: 1 })
        ));
    }

    #[test]
    fn system_observation_requires_v1_2() {
        let step = Step {
            step_id: 1,
            timestamp: None,
            source: StepSource::System,
            model_name: None,
            reasoning_effort: None,
            message: "system".into(),
            reasoning_content: None,
            tool_calls: None,
            observation: Some(Observation {
                results: vec![ObservationResult {
                    source_call_id: None,
                    content: Some("checkpoint".into()),
                    subagent_trajectory_ref: None,
                }],
            }),
            metrics: None,
            is_copied_context: None,
            extra: None,
        };

        assert!(matches!(
            step.validate(SchemaVersion::V1_1),
            Err(ValidationError::UnsupportedFieldForSchemaVersion { .. })
        ));
    }
}
