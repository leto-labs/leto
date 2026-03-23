//! Native ATIF transcript construction for `brain`.
//!
//! This module owns append-only trajectory building at turn boundaries. It
//! preserves historical ATIF steps already persisted for the session and only
//! appends the new steps produced by the current turn.

use std::sync::Arc;

use atif::{
    Agent as AtifAgent, FinalMetrics as AtifFinalMetrics, Observation as AtifObservation,
    ObservationResult as AtifObservationResult, Step as AtifStep, StepSource as AtifStepSource,
    ToolCall as AtifToolCall, Trajectory as AtifTrajectory,
};
use brain_types::{AgentConfig, AtifEvent, BrainError, Event, Message, Provider, Role, Tool};
use chrono::{DateTime, Utc};
use serde_json::{Map, Value, json};
use ulid::Ulid;

#[derive(Debug, Clone, Copy)]
pub(crate) struct TurnSummary {
    pub iterations: u32,
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub cache_read_tokens: Option<u32>,
    pub cache_write_tokens: Option<u32>,
    pub reasoning_tokens: Option<u32>,
    pub total_tokens: u32,
}

#[derive(Debug, Clone)]
pub(crate) struct CompletedTrajectory {
    pub trajectory: AtifTrajectory,
    pub new_steps: Vec<AtifStep>,
    pub final_metrics: AtifFinalMetrics,
}

impl TurnSummary {
    pub(crate) fn final_metrics(&self) -> AtifFinalMetrics {
        AtifFinalMetrics {
            total_prompt_tokens: self.prompt_tokens,
            total_completion_tokens: self.completion_tokens,
            total_cached_tokens: self.cache_read_tokens,
            total_cost_usd: None,
            total_steps: None,
            extra: Some(
                json_object(json!({
                    "iterations": self.iterations,
                    "total_tokens": self.total_tokens,
                    "cache_write_tokens": self.cache_write_tokens,
                    "reasoning_tokens": self.reasoning_tokens,
                }))
                .expect("final metrics extra must be an object"),
            ),
        }
    }
}

pub(crate) fn started_event(
    session_id: Ulid,
    config: &AgentConfig,
    provider: &dyn Provider,
    tools: &[Arc<dyn Tool>],
    started_at: DateTime<Utc>,
) -> Result<Event, BrainError> {
    Ok(Event::Atif {
        event: AtifEvent::TrajectoryStarted {
            schema_version: config.atif.schema_version,
            session_id: session_id.to_string(),
            agent: build_agent(config, provider, tools, started_at, None)?,
        },
    })
}

pub(crate) fn complete_turn(
    session_id: Ulid,
    existing_trajectory: Option<AtifTrajectory>,
    config: &AgentConfig,
    provider: &dyn Provider,
    tools: &[Arc<dyn Tool>],
    new_messages: &[Message],
    started_at: DateTime<Utc>,
    finished_at: DateTime<Utc>,
    turn_summary: TurnSummary,
) -> Result<CompletedTrajectory, BrainError> {
    let mut trajectory = existing_trajectory.unwrap_or_else(|| AtifTrajectory {
        schema_version: config.atif.schema_version,
        session_id: session_id.to_string(),
        agent: AtifAgent {
            name: "brain".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            model_name: None,
            tool_definitions: None,
            extra: None,
        },
        steps: Vec::new(),
        notes: None,
        final_metrics: None,
        continued_trajectory_ref: None,
        extra: None,
    });

    trajectory.schema_version = config.atif.schema_version;
    trajectory.agent = build_agent(config, provider, tools, started_at, Some(finished_at))?;

    let start_step_id = trajectory.steps.len() as u32 + 1;
    let include_system_prompt = trajectory
        .steps
        .is_empty()
        .then_some(config.system_prompt.as_deref())
        .flatten();
    let new_steps = build_steps(
        new_messages,
        effective_model_name(config, provider),
        start_step_id,
        include_system_prompt,
    )?;
    trajectory.steps.extend(new_steps.clone());

    let final_metrics = {
        let mut metrics = turn_summary.final_metrics();
        metrics.total_steps = Some(trajectory.steps.len() as u32);
        metrics
    };
    trajectory.final_metrics = Some(final_metrics.clone());

    trajectory
        .validate()
        .map_err(|error| BrainError::Internal(format!("invalid ATIF trajectory: {error}")))?;

    Ok(CompletedTrajectory {
        trajectory,
        new_steps,
        final_metrics,
    })
}

fn build_agent(
    config: &AgentConfig,
    provider: &dyn Provider,
    tools: &[Arc<dyn Tool>],
    started_at: DateTime<Utc>,
    finished_at: Option<DateTime<Utc>>,
) -> Result<AtifAgent, BrainError> {
    let mut extra = Map::new();
    extra.insert("provider".into(), Value::String(provider.info().name));
    if let Some(loop_name) = config.loop_name.as_ref() {
        extra.insert("loop_name".into(), Value::String(loop_name.clone()));
    }
    extra.insert("started_at".into(), Value::String(started_at.to_rfc3339()));
    if let Some(finished_at) = finished_at {
        extra.insert(
            "finished_at".into(),
            Value::String(finished_at.to_rfc3339()),
        );
    }

    Ok(AtifAgent {
        name: "brain".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        model_name: effective_model_name(config, provider),
        tool_definitions: {
            let definitions = tools
                .iter()
                .map(|tool| tool_definition_json(&tool.definition()))
                .collect::<Result<Vec<_>, _>>()?;
            (!definitions.is_empty()).then_some(definitions)
        },
        extra: (!extra.is_empty()).then_some(extra),
    })
}

fn effective_model_name(config: &AgentConfig, provider: &dyn Provider) -> Option<String> {
    config
        .inference
        .model
        .clone()
        .or_else(|| provider.info().default_model)
}

fn build_steps(
    messages: &[Message],
    model_name: Option<String>,
    start_step_id: u32,
    leading_system_prompt: Option<&str>,
) -> Result<Vec<AtifStep>, BrainError> {
    let mut steps = Vec::new();
    let mut step_id = start_step_id;

    if let Some(prompt) = leading_system_prompt {
        steps.push(AtifStep {
            step_id,
            timestamp: None,
            source: AtifStepSource::System,
            model_name: None,
            reasoning_effort: None,
            message: prompt.into(),
            reasoning_content: None,
            tool_calls: None,
            observation: None,
            metrics: None,
            is_copied_context: None,
            extra: None,
        });
        step_id += 1;
    }

    let mut index = 0usize;
    while index < messages.len() {
        let message = &messages[index];
        match message.role {
            Role::System => {
                steps.push(AtifStep {
                    step_id,
                    timestamp: Some(message.created_at.to_rfc3339()),
                    source: AtifStepSource::System,
                    model_name: None,
                    reasoning_effort: None,
                    message: to_atif_message_content(&message.content),
                    reasoning_content: None,
                    tool_calls: None,
                    observation: None,
                    metrics: None,
                    is_copied_context: None,
                    extra: None,
                });
                step_id += 1;
                index += 1;
            }
            Role::User => {
                steps.push(AtifStep {
                    step_id,
                    timestamp: Some(message.created_at.to_rfc3339()),
                    source: AtifStepSource::User,
                    model_name: None,
                    reasoning_effort: None,
                    message: to_atif_message_content(&message.content),
                    reasoning_content: None,
                    tool_calls: None,
                    observation: None,
                    metrics: None,
                    is_copied_context: None,
                    extra: None,
                });
                step_id += 1;
                index += 1;
            }
            Role::Assistant => {
                let tool_calls = if message.tool_calls.is_empty() {
                    None
                } else {
                    Some(
                        message
                            .tool_calls
                            .iter()
                            .map(|tool_call| {
                                Ok(AtifToolCall {
                                    tool_call_id: tool_call.id.clone(),
                                    function_name: tool_call.name.clone(),
                                    arguments: json_object(tool_call.arguments.clone())?,
                                })
                            })
                            .collect::<Result<Vec<_>, BrainError>>()?,
                    )
                };

                let known_call_ids = message
                    .tool_calls
                    .iter()
                    .map(|tool_call| tool_call.id.as_str())
                    .collect::<Vec<_>>();
                let mut observation_results = Vec::new();
                let mut next_index = index + 1;
                while next_index < messages.len() && matches!(messages[next_index].role, Role::Tool)
                {
                    let tool_message = &messages[next_index];
                    let source_call_id = tool_message
                        .tool_call_id
                        .as_deref()
                        .filter(|call_id| known_call_ids.contains(call_id))
                        .map(ToOwned::to_owned);
                    observation_results.push(AtifObservationResult {
                        source_call_id,
                        content: Some(to_atif_message_content(&tool_message.content)),
                        subagent_trajectory_ref: None,
                    });
                    next_index += 1;
                }

                steps.push(AtifStep {
                    step_id,
                    timestamp: Some(message.created_at.to_rfc3339()),
                    source: AtifStepSource::Agent,
                    model_name: model_name.clone(),
                    reasoning_effort: None,
                    message: assistant_step_message(message).into(),
                    reasoning_content: message.reasoning_content.clone().map(Into::into),
                    tool_calls,
                    observation: (!observation_results.is_empty()).then_some(AtifObservation {
                        results: observation_results,
                    }),
                    metrics: None,
                    is_copied_context: None,
                    extra: None,
                });
                step_id += 1;
                index = next_index;
            }
            Role::Tool => {
                steps.push(AtifStep {
                    step_id,
                    timestamp: Some(message.created_at.to_rfc3339()),
                    source: AtifStepSource::Agent,
                    model_name: model_name.clone(),
                    reasoning_effort: None,
                    message: "Tool result".into(),
                    reasoning_content: None,
                    tool_calls: None,
                    observation: Some(AtifObservation {
                        results: vec![AtifObservationResult {
                            source_call_id: None,
                            content: Some(to_atif_message_content(&message.content)),
                            subagent_trajectory_ref: None,
                        }],
                    }),
                    metrics: None,
                    is_copied_context: None,
                    extra: None,
                });
                step_id += 1;
                index += 1;
            }
        }
    }

    Ok(steps)
}

pub(crate) fn completion_events(completed: &CompletedTrajectory) -> Vec<Event> {
    let mut events = Vec::with_capacity(completed.new_steps.len() + 2);
    for step in &completed.new_steps {
        events.push(Event::Atif {
            event: AtifEvent::StepCompleted { step: step.clone() },
        });
    }
    events.push(Event::Atif {
        event: AtifEvent::FinalMetrics {
            final_metrics: completed.final_metrics.clone(),
        },
    });
    events.push(Event::Atif {
        event: AtifEvent::TrajectoryCompleted {
            trajectory: completed.trajectory.clone(),
        },
    });
    events
}

fn assistant_step_message(message: &Message) -> String {
    if !message.content.is_empty() {
        return message.content.to_string();
    }

    if !message.tool_calls.is_empty() {
        let names = message
            .tool_calls
            .iter()
            .map(|tool_call| tool_call.name.as_str())
            .collect::<Vec<_>>();
        return format!("Called {}", names.join(", "));
    }

    "Assistant turn".to_owned()
}

fn to_atif_message_content(content: &brain_types::MessageContent) -> atif::MessageContent {
    match content {
        brain_types::MessageContent::Text(text) => text.clone().into(),
        brain_types::MessageContent::Parts(parts) => parts
            .iter()
            .map(|part| match part {
                brain_types::ContentPart::Text { text } => text.clone(),
                brain_types::ContentPart::ImageUrl { .. } => "[image omitted]".to_owned(),
            })
            .collect::<Vec<_>>()
            .join("\n")
            .into(),
    }
}

fn tool_definition_json(tool_def: &brain_types::ToolDef) -> Result<Map<String, Value>, BrainError> {
    json_object(json!({
        "type": "function",
        "function": {
            "name": tool_def.name,
            "description": tool_def.description,
            "parameters": tool_def.parameters,
        }
    }))
}

fn json_object(value: Value) -> Result<Map<String, Value>, BrainError> {
    match value {
        Value::Object(map) => Ok(map),
        _ => Err(BrainError::Internal("expected JSON object".into())),
    }
}
