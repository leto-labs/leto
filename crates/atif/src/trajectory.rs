use serde::{Deserialize, Serialize};

use crate::{
    Agent, FinalMetrics, JsonObject, Observation, SchemaVersion, Step, ValidationError,
    require_supported,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Trajectory {
    #[serde(default)]
    pub schema_version: SchemaVersion,
    pub session_id: String,
    pub agent: Agent,
    pub steps: Vec<Step>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub final_metrics: Option<FinalMetrics>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continued_trajectory_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra: Option<JsonObject>,
}

impl Trajectory {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.steps.is_empty() {
            return Err(ValidationError::EmptySteps);
        }

        if self.extra.is_some() {
            require_supported(
                self.schema_version.supports_trajectory_extra(),
                "trajectory.extra",
                SchemaVersion::V1_1,
                self.schema_version,
            )?;
        }

        self.agent.validate(self.schema_version)?;

        for (index, step) in self.steps.iter().enumerate() {
            let expected = index + 1;
            if step.step_id != expected as u32 {
                return Err(ValidationError::NonSequentialStepId {
                    expected: expected as u32,
                    actual: step.step_id,
                });
            }
            step.validate(self.schema_version)?;
        }

        for step in &self.steps {
            if let Some(observation) = &step.observation {
                validate_tool_call_references(
                    step.step_id,
                    observation,
                    step.tool_calls.as_deref(),
                )?;
            }
        }

        Ok(())
    }

    pub fn has_multimodal_content(&self) -> bool {
        self.steps.iter().any(Step::has_multimodal_content)
    }
}

fn validate_tool_call_references(
    step_id: u32,
    observation: &Observation,
    tool_calls: Option<&[crate::ToolCall]>,
) -> Result<(), ValidationError> {
    let valid_tool_call_ids = tool_calls
        .map(|calls| {
            calls
                .iter()
                .map(|tool_call| tool_call.tool_call_id.as_str())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    for result in &observation.results {
        if let Some(source_call_id) = result.source_call_id.as_deref()
            && !valid_tool_call_ids.contains(&source_call_id)
        {
            return Err(ValidationError::UnknownSourceCallId {
                step_id,
                source_call_id: source_call_id.to_owned(),
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::{
        Agent, ContentPart, ContentPartKind, FinalMetrics, ImageMediaType, ImageSource,
        MessageContent, Observation, ObservationResult, SchemaVersion, Step, StepSource,
        Trajectory, ValidationError,
    };

    fn sample_agent() -> Agent {
        Agent {
            name: "brain".into(),
            version: "0.1.0".into(),
            model_name: Some("gpt-5.4".into()),
            tool_definitions: None,
            extra: None,
        }
    }

    #[test]
    fn trajectory_serializes_with_default_schema_version() {
        let trajectory = Trajectory {
            schema_version: SchemaVersion::default(),
            session_id: "sess_123".into(),
            agent: sample_agent(),
            steps: vec![Step {
                step_id: 1,
                timestamp: None,
                source: StepSource::User,
                model_name: None,
                reasoning_effort: None,
                message: "hello".into(),
                reasoning_content: None,
                tool_calls: None,
                observation: None,
                metrics: None,
                is_copied_context: None,
                extra: None,
            }],
            notes: None,
            final_metrics: None,
            continued_trajectory_ref: None,
            extra: None,
        };

        let value = serde_json::to_value(&trajectory).unwrap();
        assert_eq!(value["schema_version"], "ATIF-v1.6");
    }

    #[test]
    fn validation_rejects_non_sequential_steps() {
        let trajectory = Trajectory {
            schema_version: SchemaVersion::default(),
            session_id: "sess_123".into(),
            agent: sample_agent(),
            steps: vec![Step {
                step_id: 2,
                timestamp: None,
                source: StepSource::User,
                model_name: None,
                reasoning_effort: None,
                message: "hello".into(),
                reasoning_content: None,
                tool_calls: None,
                observation: None,
                metrics: None,
                is_copied_context: None,
                extra: None,
            }],
            notes: None,
            final_metrics: None,
            continued_trajectory_ref: None,
            extra: None,
        };

        assert!(matches!(
            trajectory.validate(),
            Err(ValidationError::NonSequentialStepId {
                expected: 1,
                actual: 2
            })
        ));
    }

    #[test]
    fn validation_rejects_unknown_tool_call_references() {
        let trajectory = Trajectory {
            schema_version: SchemaVersion::default(),
            session_id: "sess_123".into(),
            agent: sample_agent(),
            steps: vec![Step {
                step_id: 1,
                timestamp: None,
                source: StepSource::Agent,
                model_name: Some("gpt-5.4".into()),
                reasoning_effort: None,
                message: "tool".into(),
                reasoning_content: None,
                tool_calls: None,
                observation: Some(Observation {
                    results: vec![ObservationResult {
                        source_call_id: Some("call_123".into()),
                        content: Some("result".into()),
                        subagent_trajectory_ref: None,
                    }],
                }),
                metrics: None,
                is_copied_context: None,
                extra: None,
            }],
            notes: None,
            final_metrics: None,
            continued_trajectory_ref: None,
            extra: None,
        };

        assert!(matches!(
            trajectory.validate(),
            Err(ValidationError::UnknownSourceCallId { .. })
        ));
    }

    #[test]
    fn content_parts_support_multimodal_detection() {
        let trajectory = Trajectory {
            schema_version: SchemaVersion::default(),
            session_id: "sess_123".into(),
            agent: sample_agent(),
            steps: vec![Step {
                step_id: 1,
                timestamp: None,
                source: StepSource::User,
                model_name: None,
                reasoning_effort: None,
                message: MessageContent::Parts(vec![
                    ContentPart {
                        kind: ContentPartKind::Text,
                        text: Some("look".into()),
                        source: None,
                    },
                    ContentPart {
                        kind: ContentPartKind::Image,
                        text: None,
                        source: Some(ImageSource {
                            media_type: ImageMediaType::Png,
                            path: "images/example.png".into(),
                        }),
                    },
                ]),
                reasoning_content: None,
                tool_calls: None,
                observation: None,
                metrics: None,
                is_copied_context: None,
                extra: None,
            }],
            notes: None,
            final_metrics: Some(FinalMetrics {
                total_prompt_tokens: Some(1),
                total_completion_tokens: Some(2),
                total_cached_tokens: None,
                total_cost_usd: Some(0.1),
                total_steps: Some(1),
                extra: Some(serde_json::from_value(json!({"status": "completed"})).unwrap()),
            }),
            continued_trajectory_ref: None,
            extra: None,
        };

        assert!(trajectory.has_multimodal_content());
        trajectory.validate().unwrap();
    }

    #[test]
    fn trajectory_extra_requires_v1_1() {
        let trajectory = Trajectory {
            schema_version: SchemaVersion::V1_0,
            session_id: "sess_123".into(),
            agent: sample_agent(),
            steps: vec![Step {
                step_id: 1,
                timestamp: None,
                source: StepSource::User,
                model_name: None,
                reasoning_effort: None,
                message: "hello".into(),
                reasoning_content: None,
                tool_calls: None,
                observation: None,
                metrics: None,
                is_copied_context: None,
                extra: None,
            }],
            notes: None,
            final_metrics: None,
            continued_trajectory_ref: None,
            extra: Some(serde_json::from_value(json!({"debug": true})).unwrap()),
        };

        assert!(matches!(
            trajectory.validate(),
            Err(ValidationError::UnsupportedFieldForSchemaVersion { .. })
        ));
    }
}
