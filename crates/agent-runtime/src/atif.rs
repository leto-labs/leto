use chrono::{DateTime, Utc};
use provider::{ContentBlock, Message, MessageRole, Provider, ToolDefinition, Usage};
use serde_json::{Map, Value, json};
use ulid::Ulid;

use crate::RuntimeConfig;

const SCHEMA_VERSION: atif::SchemaVersion = atif::SchemaVersion::V1_6;

#[derive(Debug, Clone, Default)]
pub(crate) struct TurnSummary {
    pub(crate) iterations: u32,
    pub(crate) prompt_tokens: Option<u32>,
    pub(crate) completion_tokens: Option<u32>,
    pub(crate) cache_read_tokens: Option<u32>,
    pub(crate) cache_write_tokens: Option<u32>,
    pub(crate) reasoning_tokens: Option<u32>,
    pub(crate) total_tokens: u32,
}

#[derive(Debug, Clone)]
pub(crate) struct CompletedTrajectory {
    pub(crate) trajectory: atif::Trajectory,
    pub(crate) new_steps: Vec<atif::Step>,
    pub(crate) final_metrics: atif::FinalMetrics,
}

impl TurnSummary {
    pub(crate) fn record_usage(&mut self, usage: &Usage) {
        accumulate_optional(&mut self.prompt_tokens, usage.input_tokens);
        accumulate_optional(&mut self.completion_tokens, usage.output_tokens);
        accumulate_optional(&mut self.cache_read_tokens, usage.cache_read_tokens);
        accumulate_optional(&mut self.cache_write_tokens, usage.cache_write_tokens);
        accumulate_optional(&mut self.reasoning_tokens, usage.reasoning_tokens);

        self.total_tokens = self.total_tokens.saturating_add(
            usage.total_tokens.unwrap_or_else(|| {
                usage
                    .input_tokens
                    .unwrap_or(0)
                    .saturating_add(usage.output_tokens.unwrap_or(0))
            }),
        );
    }

    pub(crate) fn final_metrics(&self) -> atif::FinalMetrics {
        atif::FinalMetrics {
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

pub(crate) fn schema_version() -> atif::SchemaVersion {
    SCHEMA_VERSION
}

pub(crate) fn build_agent(
    config: &RuntimeConfig,
    provider: &dyn Provider,
    tools: &[ToolDefinition],
    started_at: DateTime<Utc>,
    finished_at: Option<DateTime<Utc>>,
) -> Result<atif::Agent, String> {
    let mut extra = Map::new();
    extra.insert("provider".into(), Value::String(provider.info().name));
    if let Some(loop_name) = config
        .request
        .metadata
        .get("loop_name")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
    {
        extra.insert("loop_name".into(), Value::String(loop_name));
    }
    extra.insert("started_at".into(), Value::String(started_at.to_rfc3339()));
    if let Some(finished_at) = finished_at {
        extra.insert(
            "finished_at".into(),
            Value::String(finished_at.to_rfc3339()),
        );
    }

    Ok(atif::Agent {
        name: "agent".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        model_name: effective_model_name(config, provider),
        tool_definitions: {
            let definitions = tools
                .iter()
                .map(tool_definition_json)
                .collect::<Result<Vec<_>, _>>()?;
            (!definitions.is_empty()).then_some(definitions)
        },
        extra: (!extra.is_empty()).then_some(extra),
    })
}

pub(crate) fn complete_turn(
    session_id: Ulid,
    existing_trajectory: Option<atif::Trajectory>,
    config: &RuntimeConfig,
    provider: &dyn Provider,
    tools: &[ToolDefinition],
    new_messages: &[Message],
    started_at: DateTime<Utc>,
    finished_at: DateTime<Utc>,
    turn_summary: TurnSummary,
) -> Result<CompletedTrajectory, String> {
    let mut trajectory = existing_trajectory.unwrap_or_else(|| atif::Trajectory {
        schema_version: schema_version(),
        session_id: session_id.to_string(),
        agent: atif::Agent {
            name: "agent".into(),
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

    trajectory.schema_version = schema_version();
    trajectory.agent = build_agent(config, provider, tools, started_at, Some(finished_at))?;

    let start_step_id = trajectory.steps.len() as u32 + 1;
    let new_steps = build_steps(
        new_messages,
        effective_model_name(config, provider),
        config
            .request
            .reasoning
            .as_ref()
            .and_then(|reasoning| reasoning.effort.clone())
            .map(atif::ReasoningEffort::Label),
        start_step_id,
    )?;
    trajectory.steps.extend(new_steps.clone());

    let mut final_metrics = turn_summary.final_metrics();
    final_metrics.total_steps = Some(trajectory.steps.len() as u32);
    trajectory.final_metrics = Some(final_metrics.clone());

    trajectory
        .validate()
        .map_err(|error| format!("invalid ATIF trajectory: {error}"))?;

    Ok(CompletedTrajectory {
        trajectory,
        new_steps,
        final_metrics,
    })
}

fn effective_model_name(config: &RuntimeConfig, provider: &dyn Provider) -> Option<String> {
    config
        .model
        .clone()
        .or_else(|| provider.info().default_model().map(|m| m.id.to_string()))
}

fn build_steps(
    messages: &[Message],
    model_name: Option<String>,
    reasoning_effort: Option<atif::ReasoningEffort>,
    start_step_id: u32,
) -> Result<Vec<atif::Step>, String> {
    let mut steps = Vec::new();
    let mut step_id = start_step_id;
    let mut index = 0usize;

    while index < messages.len() {
        let message = &messages[index];
        match message.role {
            MessageRole::System | MessageRole::Developer => {
                steps.push(atif::Step {
                    step_id,
                    timestamp: None,
                    source: atif::StepSource::System,
                    model_name: None,
                    reasoning_effort: None,
                    message: message_content_from_blocks(&message.content, false, false),
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
            MessageRole::User => {
                if is_tool_result_message(message) {
                    steps.push(atif::Step {
                        step_id,
                        timestamp: None,
                        source: atif::StepSource::Agent,
                        model_name: model_name.clone(),
                        reasoning_effort: reasoning_effort.clone(),
                        message: "Tool result".into(),
                        reasoning_content: None,
                        tool_calls: None,
                        observation: Some(atif::Observation {
                            results: tool_result_observations(message, &[]),
                        }),
                        metrics: None,
                        is_copied_context: None,
                        extra: None,
                    });
                } else {
                    steps.push(atif::Step {
                        step_id,
                        timestamp: None,
                        source: atif::StepSource::User,
                        model_name: None,
                        reasoning_effort: None,
                        message: message_content_from_blocks(&message.content, false, false),
                        reasoning_content: None,
                        tool_calls: None,
                        observation: None,
                        metrics: None,
                        is_copied_context: None,
                        extra: None,
                    });
                }
                step_id += 1;
                index += 1;
            }
            MessageRole::Assistant => {
                let tool_calls = tool_calls_from_blocks(&message.content)?;
                let known_call_ids = tool_calls
                    .as_deref()
                    .unwrap_or(&[])
                    .iter()
                    .map(|tool_call| tool_call.tool_call_id.as_str())
                    .collect::<Vec<_>>();
                let mut observation_results = Vec::new();
                let mut next_index = index + 1;
                while next_index < messages.len() && is_tool_result_message(&messages[next_index]) {
                    observation_results.extend(tool_result_observations(
                        &messages[next_index],
                        &known_call_ids,
                    ));
                    next_index += 1;
                }

                steps.push(atif::Step {
                    step_id,
                    timestamp: None,
                    source: atif::StepSource::Agent,
                    model_name: model_name.clone(),
                    reasoning_effort: reasoning_effort.clone(),
                    message: assistant_step_message(message),
                    reasoning_content: reasoning_text_from_blocks(&message.content),
                    tool_calls,
                    observation: (!observation_results.is_empty()).then_some(atif::Observation {
                        results: observation_results,
                    }),
                    metrics: None,
                    is_copied_context: None,
                    extra: None,
                });
                step_id += 1;
                index = next_index;
            }
        }
    }

    Ok(steps)
}

fn assistant_step_message(message: &Message) -> atif::MessageContent {
    let text = lossy_text_from_blocks(&message.content, false, false);
    if !text.is_empty() {
        return text.into();
    }

    let tool_names = message
        .content
        .iter()
        .filter_map(|block| match block {
            ContentBlock::ToolCall { name, .. } => Some(name.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    if !tool_names.is_empty() {
        return format!("Called {}", tool_names.join(", ")).into();
    }

    "Assistant turn".into()
}

fn message_content_from_blocks(
    blocks: &[ContentBlock],
    include_reasoning: bool,
    include_tool_results: bool,
) -> atif::MessageContent {
    lossy_text_from_blocks(blocks, include_reasoning, include_tool_results).into()
}

fn reasoning_text_from_blocks(blocks: &[ContentBlock]) -> Option<String> {
    let text = blocks
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Reasoning { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    (!text.is_empty()).then_some(text)
}

fn lossy_text_from_blocks(
    blocks: &[ContentBlock],
    include_reasoning: bool,
    include_tool_results: bool,
) -> String {
    blocks
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text { text } | ContentBlock::Refusal { text } => Some(text.clone()),
            ContentBlock::ImageUrl { .. } => Some("[image omitted]".to_owned()),
            ContentBlock::Reasoning { text } if include_reasoning => Some(text.clone()),
            ContentBlock::ToolResult { output, .. } if include_tool_results => {
                Some(json_value_to_string(output))
            }
            ContentBlock::ToolCall { .. }
            | ContentBlock::ToolResult { .. }
            | ContentBlock::Reasoning { .. } => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn tool_calls_from_blocks(blocks: &[ContentBlock]) -> Result<Option<Vec<atif::ToolCall>>, String> {
    let mut tool_calls = Vec::new();
    for block in blocks {
        let ContentBlock::ToolCall { id, name, input } = block else {
            continue;
        };
        let arguments = match input {
            Value::Object(arguments) => arguments.clone(),
            _ => {
                return Err(format!(
                    "tool call '{name}' used non-object arguments, which ATIF does not support"
                ));
            }
        };
        tool_calls.push(atif::ToolCall {
            tool_call_id: id.clone(),
            function_name: name.clone(),
            arguments,
        });
    }

    Ok((!tool_calls.is_empty()).then_some(tool_calls))
}

fn is_tool_result_message(message: &Message) -> bool {
    !message.content.is_empty()
        && message
            .content
            .iter()
            .all(|block| matches!(block, ContentBlock::ToolResult { .. }))
}

fn tool_result_observations(
    message: &Message,
    known_call_ids: &[&str],
) -> Vec<atif::ObservationResult> {
    message
        .content
        .iter()
        .filter_map(|block| match block {
            ContentBlock::ToolResult {
                call_id,
                output,
                ..
            } => Some(atif::ObservationResult {
                source_call_id: known_call_ids.contains(&call_id.as_str()).then(|| call_id.clone()),
                content: Some(json_value_to_string(output).into()),
                subagent_trajectory_ref: None,
            }),
            _ => None,
        })
        .collect()
}

fn tool_definition_json(tool_def: &ToolDefinition) -> Result<Map<String, Value>, String> {
    json_object(json!({
        "type": "function",
        "function": {
            "name": tool_def.name,
            "description": tool_def.description,
            "parameters": tool_def.input_schema,
        }
    }))
}

fn json_object(value: Value) -> Result<Map<String, Value>, String> {
    match value {
        Value::Object(map) => Ok(map),
        _ => Err("expected JSON object".into()),
    }
}

fn json_value_to_string(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        _ => serde_json::to_string(value).unwrap_or_else(|_| String::new()),
    }
}

fn accumulate_optional(target: &mut Option<u32>, value: Option<u32>) {
    if let Some(value) = value {
        *target = Some(target.unwrap_or(0).saturating_add(value));
    }
}
