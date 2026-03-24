use std::sync::Arc;

use futures::StreamExt;
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::CancellationToken;
use ulid::Ulid;

use brain_types::*;

use crate::output::truncate_tool_result;

const TERMINUS_JSON_PROMPT_TEMPLATE: &str =
    include_str!("terminus2/templates/terminus-json-plain.txt");

const COMPLETION_CONFIRMATION: &str = r#"Current terminal state:
{terminal_state}

Are you sure you want to mark the task as complete? This will trigger your solution to be graded and you won't be able to make any further corrections. If so, include "task_complete": true in your JSON response again."#;

const TERMINUS_TIMEOUT_TEMPLATE: &str = include_str!("terminus2/templates/timeout.txt");

const DEFAULT_TERMINUS_MAX_ITERATIONS: u32 = 1_000_000;
const DEFAULT_PROACTIVE_SUMMARIZATION_THRESHOLD: u64 = 8_000;

pub struct Terminus2Loop;

impl AgentLoop for Terminus2Loop {
    fn run(
        &self,
        provider: Arc<dyn Provider>,
        tools: Vec<Arc<dyn Tool>>,
        messages: Vec<Message>,
        config: AgentConfig,
        cancel: CancellationToken,
        session_id: Option<Ulid>,
    ) -> EventStream {
        let (tx, rx) = mpsc::channel(256);

        tokio::spawn(async move {
            if let Err(error) = run_inner(
                provider,
                tools,
                messages,
                config,
                cancel,
                session_id,
                tx.clone(),
            )
            .await
            {
                let _ = tx
                    .send(Event::Error {
                        code: error.code(),
                        message: error.to_string(),
                        recoverable: error.recoverable(),
                    })
                    .await;
            }
        });

        Box::pin(ReceiverStream::new(rx))
    }
}

#[derive(Debug, Clone)]
struct ParsedCommand {
    keystrokes: String,
    duration_seconds: f64,
}

#[derive(Debug, Clone)]
struct ParseResult {
    commands: Vec<ParsedCommand>,
    is_task_complete: bool,
    error: Option<String>,
    warning: Option<String>,
    analysis: String,
    plan: String,
}

#[derive(Debug, Clone, Default)]
struct UsageTotals {
    prompt_tokens: u32,
    completion_tokens: u32,
    cache_read_tokens: u32,
    cache_write_tokens: u32,
    reasoning_tokens: u32,
    total_tokens: u32,
}

impl UsageTotals {
    fn add(&mut self, usage: &TokenUsage) {
        self.prompt_tokens += usage.prompt;
        self.completion_tokens += usage.completion;
        self.cache_read_tokens += usage.cache_read.unwrap_or(0);
        self.cache_write_tokens += usage.cache_write.unwrap_or(0);
        self.reasoning_tokens += usage.reasoning.unwrap_or(0);
        self.total_tokens += usage.total;
    }

    fn merge(&mut self, other: &Self) {
        self.prompt_tokens += other.prompt_tokens;
        self.completion_tokens += other.completion_tokens;
        self.cache_read_tokens += other.cache_read_tokens;
        self.cache_write_tokens += other.cache_write_tokens;
        self.reasoning_tokens += other.reasoning_tokens;
        self.total_tokens += other.total_tokens;
    }
}

#[derive(Debug, Clone)]
struct ProviderTurn {
    content: String,
    usage: Option<TokenUsage>,
    output_length_exceeded: bool,
    reasoning_content: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct TerminalObservation {
    #[serde(default)]
    timed_out: bool,
    terminal_state: String,
    observation: String,
}

#[derive(Debug, Clone)]
struct SummaryOutcome {
    handoff_prompt: String,
    summary_tokens: usize,
    usage: UsageTotals,
}

async fn run_inner(
    provider: Arc<dyn Provider>,
    tools: Vec<Arc<dyn Tool>>,
    mut messages: Vec<Message>,
    config: AgentConfig,
    cancel: CancellationToken,
    session_id: Option<Ulid>,
    tx: mpsc::Sender<Event>,
) -> Result<(), BrainError> {
    if let Some(ref prompt) = config.system_prompt {
        messages.insert(0, Message::system(prompt));
    }

    let terminal_tool = tools
        .iter()
        .find(|tool| tool.definition().name == "terminal_session")
        .cloned()
        .ok_or_else(|| BrainError::ToolNotFound("terminal_session".into()))?;

    let session_key = session_id
        .map(|session_id| session_id.to_string())
        .unwrap_or_else(|| Ulid::new().to_string());
    let original_instruction = messages
        .iter()
        .rev()
        .find(|message| matches!(message.role, Role::User))
        .map(|message| message.content_text_lossy())
        .unwrap_or_default();

    let initial_screen =
        terminal_capture(&*terminal_tool, &session_key, config.tool_output_max_bytes).await?;
    let initial_prompt = render_template(
        TERMINUS_JSON_PROMPT_TEMPLATE,
        &[
            ("instruction", original_instruction.as_str()),
            ("terminal_state", initial_screen.terminal_state.as_str()),
        ],
    );

    if let Some(message) = messages
        .iter_mut()
        .rev()
        .find(|message| matches!(message.role, Role::User))
    {
        message.set_text_content(initial_prompt.clone());
        message.reasoning_content = None;
    } else {
        messages.push(Message::user(initial_prompt.clone()));
    }

    let mut iterations = 0u32;
    let mut pending_completion = false;
    let mut usage = UsageTotals::default();
    let mut chat = messages;
    let effective_max_iterations = effective_max_iterations(&config);

    loop {
        if cancel.is_cancelled() {
            return Err(BrainError::Cancelled);
        }
        if iterations >= effective_max_iterations {
            return Err(BrainError::MaxIterations(iterations));
        }
        iterations += 1;

        if should_proactively_summarize(provider.as_ref(), &chat, &config.inference) {
            let original_messages = chat.len();
            if let Some(summary) = summarize_history(
                provider.as_ref(),
                &mut chat,
                &config.inference,
                session_id,
                &session_key,
                &terminal_tool,
                &tx,
                &cancel,
            )
            .await?
            {
                usage.merge(&summary.usage);
                let _ = tx
                    .send(Event::Compaction {
                        original_messages,
                        summary_tokens: summary.summary_tokens,
                    })
                    .await;
                chat.push(Message::user(summary.handoff_prompt));
            }
        }

        let turn = match query_provider(
            provider.as_ref(),
            &chat,
            &config.inference,
            config.max_retries,
            session_id,
            &tx,
            &cancel,
            true,
        )
        .await
        {
            Ok(turn) => turn,
            Err(error) => match error.inference_kind() {
                Some(InferenceErrorKind::ContextLengthExceeded) => {
                    unwind_messages_to_free_tokens(
                        provider.as_ref(),
                        &config.inference,
                        &mut chat,
                        4_000,
                    );
                    let original_messages = chat.len();
                    let Some(summary) = summarize_with_fallback(
                        provider.as_ref(),
                        &mut chat,
                        &config.inference,
                        session_id,
                        &session_key,
                        &terminal_tool,
                        &tx,
                        &cancel,
                    )
                    .await?
                    else {
                        return Err(error);
                    };

                    usage.merge(&summary.usage);
                    let _ = tx
                        .send(Event::Compaction {
                            original_messages,
                            summary_tokens: summary.summary_tokens,
                        })
                        .await;
                    chat.push(Message::user(summary.handoff_prompt));
                    continue;
                }
                Some(InferenceErrorKind::OutputLengthExceeded) => {
                    chat.push(Message::user(output_length_retry_prompt(
                        provider.as_ref(),
                        &config.inference,
                    )));
                    continue;
                }
                _ => return Err(error),
            },
        };
        if let Some(ref usage_info) = turn.usage {
            usage.add(usage_info);
        }

        let parse = parse_json_response(&turn.content);
        if turn.output_length_exceeded && parse.error.is_some() {
            let retry_prompt = output_length_retry_prompt(provider.as_ref(), &config.inference);
            let mut truncated_assistant = Message::assistant(&turn.content);
            truncated_assistant.reasoning_content = turn.reasoning_content.clone();
            chat.push(truncated_assistant);
            chat.push(Message::user(retry_prompt));
            continue;
        }
        if let Some(error) = parse.error.clone() {
            let mut assistant = Message::assistant(&turn.content);
            assistant.reasoning_content = turn.reasoning_content.clone();
            let _ = tx
                .send(Event::MessageDone {
                    message: assistant.clone(),
                })
                .await;
            chat.push(assistant);

            let repair_prompt = repair_prompt(&error, parse.warning.as_deref());
            chat.push(Message::user(repair_prompt));
            continue;
        }

        let mut tool_calls = parse
            .commands
            .iter()
            .enumerate()
            .map(|(index, command)| ToolCall {
                id: format!("call_{iterations}_{}", index + 1),
                name: "bash_command".into(),
                arguments: serde_json::json!({
                    "keystrokes": command.keystrokes,
                    "duration": command.duration_seconds,
                }),
            })
            .collect::<Vec<_>>();
        if parse.is_task_complete {
            tool_calls.push(ToolCall {
                id: format!("call_{iterations}_task_complete"),
                name: "mark_task_complete".into(),
                arguments: serde_json::json!({}),
            });
        }

        let assistant_content = format!("Analysis: {}\nPlan: {}", parse.analysis, parse.plan);
        let mut assistant = Message::assistant(assistant_content.clone());
        assistant.reasoning_content = turn.reasoning_content.clone();
        assistant.tool_calls = tool_calls.clone();
        let _ = tx
            .send(Event::MessageDone {
                message: assistant.clone(),
            })
            .await;
        let mut provider_assistant = Message::assistant(assistant_content);
        provider_assistant.reasoning_content = turn.reasoning_content.clone();
        chat.push(provider_assistant);

        let mut latest_terminal = initial_screen.clone();

        for (index, command) in parse.commands.iter().enumerate() {
            let call_id = format!("call_{iterations}_{}", index + 1);
            let arguments = serde_json::json!({
                "keystrokes": command.keystrokes,
                "duration": command.duration_seconds,
            });
            emit_tool_lifecycle(&call_id, "bash_command", arguments.clone(), &tx).await;
            let terminal_result = terminal_send_keys(
                &*terminal_tool,
                &session_key,
                command,
                config.tool_output_max_bytes,
            )
            .await?;
            latest_terminal = terminal_result.clone();
            let _ = tx
                .send(Event::ToolCallDone {
                    id: call_id.clone(),
                    result: truncate_tool_result(&terminal_result.observation, &config),
                    is_error: false,
                })
                .await;
        }

        let current_terminal = if parse.commands.is_empty() {
            terminal_capture(&*terminal_tool, &session_key, config.tool_output_max_bytes).await?
        } else {
            latest_terminal
        };
        let terminal_state = current_terminal.terminal_state.clone();
        let mut observation = current_terminal.observation.clone();

        if let Some(warning) = parse.warning.as_deref() {
            observation =
                format!("Previous response had warnings:\nWARNINGS: {warning}\n\n{observation}");
        }

        if parse.is_task_complete {
            let completion_call_id = format!("call_{iterations}_task_complete");
            emit_tool_lifecycle(
                &completion_call_id,
                "mark_task_complete",
                serde_json::json!({}),
                &tx,
            )
            .await;

            let mark_result = if pending_completion {
                truncate_tool_result(&terminal_state, &config)
            } else {
                truncate_tool_result(
                    &COMPLETION_CONFIRMATION.replace("{terminal_state}", &terminal_state),
                    &config,
                )
            };
            let _ = tx
                .send(Event::ToolCallDone {
                    id: completion_call_id,
                    result: mark_result.clone(),
                    is_error: false,
                })
                .await;

            if pending_completion {
                break;
            }

            pending_completion = true;
            chat.push(Message::user(mark_result));
            continue;
        }

        pending_completion = false;
        chat.push(Message::user(observation));
    }

    let _ = terminal_close(&*terminal_tool, &session_key).await;

    let _ = tx
        .send(Event::TurnDone {
            iterations,
            prompt_tokens: (usage.prompt_tokens > 0).then_some(usage.prompt_tokens),
            completion_tokens: (usage.completion_tokens > 0).then_some(usage.completion_tokens),
            cache_read_tokens: (usage.cache_read_tokens > 0).then_some(usage.cache_read_tokens),
            cache_write_tokens: (usage.cache_write_tokens > 0).then_some(usage.cache_write_tokens),
            reasoning_tokens: (usage.reasoning_tokens > 0).then_some(usage.reasoning_tokens),
            total_tokens: usage.total_tokens,
        })
        .await;
    Ok(())
}

async fn query_provider(
    provider: &dyn Provider,
    messages: &[Message],
    inference: &InferenceConfig,
    max_retries: u32,
    session_id: Option<Ulid>,
    tx: &mpsc::Sender<Event>,
    cancel: &CancellationToken,
    stream_tokens: bool,
) -> Result<ProviderTurn, BrainError> {
    let max_attempts = max_retries.saturating_add(1);

    'attempt: for attempt in 1..=max_attempts {
        if cancel.is_cancelled() {
            return Err(BrainError::Cancelled);
        }

        let mut stream = match provider.chat(messages, &[], inference, session_id).await {
            Ok(stream) => stream,
            Err(error) => match error.inference_kind() {
                Some(InferenceErrorKind::Generic) if attempt < max_attempts => {
                    let _ = tx
                        .send(Event::Retry {
                            attempt,
                            max: max_attempts,
                            error: error.to_string(),
                        })
                        .await;
                    sleep(Duration::from_millis(u64::from(attempt) * 200)).await;
                    continue;
                }
                _ => return Err(error),
            },
        };

        let mut content = String::new();
        let mut usage = None;
        let mut saw_output = false;
        let mut output_length_exceeded = false;

        while let Some(chunk) = stream.next().await {
            if cancel.is_cancelled() {
                return Err(BrainError::Cancelled);
            }
            match chunk {
                Ok(ChatChunk::Delta { content: delta }) => {
                    saw_output = true;
                    if stream_tokens {
                        let _ = tx
                            .send(Event::Token {
                                delta: delta.clone(),
                            })
                            .await;
                    }
                    content.push_str(&delta);
                }
                Ok(ChatChunk::Done { usage: chunk_usage }) => {
                    saw_output = true;
                    usage = chunk_usage;
                }
                Ok(ChatChunk::ToolCallDelta { .. } | ChatChunk::ToolCall { .. }) => {
                    saw_output = true;
                }
                Ok(_) => {
                    saw_output = true;
                }
                Err(error)
                    if !saw_output
                        && attempt < max_attempts
                        && matches!(error.inference_kind(), Some(InferenceErrorKind::Generic)) =>
                {
                    let _ = tx
                        .send(Event::Retry {
                            attempt,
                            max: max_attempts,
                            error: error.to_string(),
                        })
                        .await;
                    sleep(Duration::from_millis(u64::from(attempt) * 200)).await;
                    continue 'attempt;
                }
                Err(error)
                    if matches!(
                        error.inference_kind(),
                        Some(InferenceErrorKind::OutputLengthExceeded)
                    ) && !content.is_empty() =>
                {
                    output_length_exceeded = true;
                    break;
                }
                Err(error) => return Err(error),
            }
        }

        return Ok(ProviderTurn {
            content,
            usage,
            output_length_exceeded,
            reasoning_content: None,
        });
    }

    Err(BrainError::Inference(
        "provider retry loop exhausted unexpectedly".into(),
    ))
}

async fn emit_tool_lifecycle(
    id: &str,
    name: &str,
    arguments: serde_json::Value,
    tx: &mpsc::Sender<Event>,
) {
    let _ = tx
        .send(Event::ToolCallPending {
            id: id.to_owned(),
            name: name.to_owned(),
            arguments: arguments.clone(),
        })
        .await;
    let _ = tx.send(Event::ToolCallApproved { id: id.to_owned() }).await;
    let _ = tx
        .send(Event::ToolCallStart {
            id: id.to_owned(),
            name: name.to_owned(),
            arguments,
        })
        .await;
}

async fn terminal_capture(
    terminal_tool: &dyn Tool,
    session_key: &str,
    max_bytes: usize,
) -> Result<TerminalObservation, BrainError> {
    invoke_terminal_tool(
        terminal_tool,
        serde_json::json!({
            "session_id": session_key,
            "action": "capture",
            "max_bytes": max_bytes,
        }),
    )
    .await
}

async fn terminal_send_keys(
    terminal_tool: &dyn Tool,
    session_key: &str,
    command: &ParsedCommand,
    max_bytes: usize,
) -> Result<TerminalObservation, BrainError> {
    let logical_keys = parse_logical_keys(&command.keystrokes);
    let observation = invoke_terminal_tool(
        terminal_tool,
        match logical_keys {
            Some(keys) => serde_json::json!({
                "session_id": session_key,
                "action": "send_keys",
                "keys": keys,
                "min_wait_seconds": command.duration_seconds,
                "max_bytes": max_bytes,
            }),
            None => serde_json::json!({
                "session_id": session_key,
                "action": "send_keys",
                "keystrokes": command.keystrokes,
                "min_wait_seconds": command.duration_seconds,
                "max_bytes": max_bytes,
            }),
        },
    )
    .await?;

    if observation.timed_out {
        return Ok(TerminalObservation {
            observation: render_template(
                TERMINUS_TIMEOUT_TEMPLATE,
                &[
                    ("command", command.keystrokes.as_str()),
                    ("timeout_sec", &format!("{:.1}", command.duration_seconds)),
                    ("terminal_state", observation.terminal_state.as_str()),
                ],
            ),
            ..observation
        });
    }

    Ok(observation)
}

fn parse_logical_keys(keystrokes: &str) -> Option<Vec<String>> {
    let trimmed = keystrokes.trim();
    if trimmed.is_empty() {
        return None;
    }

    let tokens = trimmed
        .split_whitespace()
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if tokens.iter().all(|token| {
        matches!(
            token.as_str(),
            "C-c" | "C-d" | "Enter" | "C-m" | "KPEnter" | "C-j" | "^M" | "^J"
        )
    }) {
        Some(tokens)
    } else {
        None
    }
}

async fn terminal_close(terminal_tool: &dyn Tool, session_key: &str) -> Result<(), BrainError> {
    let _ = invoke_terminal_tool(
        terminal_tool,
        serde_json::json!({
            "session_id": session_key,
            "action": "close",
        }),
    )
    .await?;
    Ok(())
}

async fn invoke_terminal_tool(
    terminal_tool: &dyn Tool,
    args: serde_json::Value,
) -> Result<TerminalObservation, BrainError> {
    let raw = terminal_tool.execute(args).await?;
    serde_json::from_str(&raw).map_err(BrainError::from)
}

fn parse_json_response(response: &str) -> ParseResult {
    let initial = try_parse_json_response(response);
    if initial.error.is_none() {
        return initial;
    }

    for (warning, candidate) in
        auto_fix_json_response(response, initial.error.as_deref().unwrap_or_default())
    {
        let mut corrected = try_parse_json_response(&candidate);
        if corrected.error.is_none() {
            let combined = combine_warning_parts(
                Some(format!(
                    "AUTO-CORRECTED: {warning} - please fix this in future responses"
                )),
                corrected.warning,
            );
            corrected.warning = combined;
            return corrected;
        }
    }

    initial
}

fn try_parse_json_response(response: &str) -> ParseResult {
    let mut warnings = Vec::new();
    let Some(json_content) = extract_json_content(response, &mut warnings) else {
        return ParseResult {
            commands: Vec::new(),
            is_task_complete: false,
            error: Some("No valid JSON found in response".into()),
            warning: combine_warnings(warnings),
            analysis: String::new(),
            plan: String::new(),
        };
    };

    let parsed = match serde_json::from_str::<serde_json::Value>(&json_content) {
        Ok(parsed) => parsed,
        Err(error) => {
            return ParseResult {
                commands: Vec::new(),
                is_task_complete: false,
                error: Some(format_json_error(&json_content, &error)),
                warning: combine_warnings(warnings),
                analysis: String::new(),
                plan: String::new(),
            };
        }
    };

    let Some(obj) = parsed.as_object() else {
        return ParseResult {
            commands: Vec::new(),
            is_task_complete: false,
            error: Some("Response must be a JSON object".into()),
            warning: combine_warnings(warnings),
            analysis: String::new(),
            plan: String::new(),
        };
    };

    let missing_fields = ["analysis", "plan", "commands"]
        .into_iter()
        .filter(|field| !obj.contains_key(*field))
        .collect::<Vec<_>>();
    if !missing_fields.is_empty() {
        return ParseResult {
            commands: Vec::new(),
            is_task_complete: false,
            error: Some(format!(
                "Missing required fields: {}",
                missing_fields.join(", ")
            )),
            warning: combine_warnings(warnings),
            analysis: String::new(),
            plan: String::new(),
        };
    }

    if !obj.get("analysis").is_none_or(serde_json::Value::is_string) {
        warnings.push("Field 'analysis' should be a string".into());
    }
    if !obj.get("plan").is_none_or(serde_json::Value::is_string) {
        warnings.push("Field 'plan' should be a string".into());
    }

    let commands_value = &obj["commands"];
    let Some(commands_array) = commands_value.as_array() else {
        return ParseResult {
            commands: Vec::new(),
            is_task_complete: false,
            error: Some("Field 'commands' must be an array".into()),
            warning: combine_warnings(warnings),
            analysis: String::new(),
            plan: String::new(),
        };
    };

    check_field_order(&json_content, &mut warnings);

    let is_task_complete = obj
        .get("task_complete")
        .map(|value| match value {
            serde_json::Value::Bool(value) => *value,
            serde_json::Value::String(value) => {
                matches!(value.to_ascii_lowercase().as_str(), "true" | "1" | "yes")
            }
            other => {
                warnings.push(format!(
                    "Field 'task_complete' should be a boolean or string, got {other}"
                ));
                false
            }
        })
        .unwrap_or(false);

    let analysis = obj
        .get("analysis")
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_owned();
    let plan = obj
        .get("plan")
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_owned();

    let mut commands = Vec::with_capacity(commands_array.len());
    for (index, value) in commands_array.iter().enumerate() {
        let Some(command) = value.as_object() else {
            if is_task_complete {
                warnings.push(format!("Command {} must be an object", index + 1));
                return ParseResult {
                    commands: Vec::new(),
                    is_task_complete: true,
                    error: None,
                    warning: combine_warnings(warnings),
                    analysis,
                    plan,
                };
            }
            return ParseResult {
                commands: Vec::new(),
                is_task_complete: false,
                error: Some(format!("Command {} must be an object", index + 1)),
                warning: combine_warnings(warnings),
                analysis,
                plan,
            };
        };
        if !command
            .get("keystrokes")
            .is_some_and(serde_json::Value::is_string)
        {
            if is_task_complete {
                warnings.push(format!(
                    "Command {} missing required 'keystrokes' field",
                    index + 1
                ));
                return ParseResult {
                    commands: Vec::new(),
                    is_task_complete: true,
                    error: None,
                    warning: combine_warnings(warnings),
                    analysis,
                    plan,
                };
            }
        }
        let Some(keystrokes) = command.get("keystrokes").and_then(|value| value.as_str()) else {
            return ParseResult {
                commands: Vec::new(),
                is_task_complete: false,
                error: Some(format!(
                    "Command {} missing required 'keystrokes' field",
                    index + 1
                )),
                warning: combine_warnings(warnings),
                analysis,
                plan,
            };
        };
        let duration_seconds = match command.get("duration") {
            Some(duration) => match duration.as_f64() {
                Some(duration) => duration,
                None => {
                    warnings.push(format!(
                        "Command {}: Invalid duration value, using default 1.0",
                        index + 1
                    ));
                    1.0
                }
            },
            None => {
                warnings.push(format!(
                    "Command {}: Missing duration field, using default 1.0",
                    index + 1
                ));
                1.0
            }
        }
        .clamp(0.0, 60.0);
        let unknown_fields = command
            .keys()
            .filter(|field| *field != "keystrokes" && *field != "duration")
            .cloned()
            .collect::<Vec<_>>();
        if !unknown_fields.is_empty() {
            warnings.push(format!(
                "Command {}: Unknown fields: {}",
                index + 1,
                unknown_fields.join(", ")
            ));
        }
        if index < commands_array.len() - 1 && !keystrokes.ends_with('\n') {
            warnings.push(format!(
                "Command {} should end with newline when followed by another command. Otherwise the two commands will be concatenated together on the same line.",
                index + 1
            ));
        }
        commands.push(ParsedCommand {
            keystrokes: keystrokes.to_owned(),
            duration_seconds,
        });
    }

    ParseResult {
        commands,
        is_task_complete,
        error: None,
        warning: combine_warnings(warnings),
        analysis,
        plan,
    }
}

fn extract_json_content(response: &str, warnings: &mut Vec<String>) -> Option<String> {
    let mut json_start = None;
    let mut brace_depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (index, ch) in response.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }

        match ch {
            '{' => {
                if brace_depth == 0 {
                    json_start = Some(index);
                }
                brace_depth += 1;
            }
            '}' => {
                if brace_depth == 0 {
                    continue;
                }
                brace_depth -= 1;
                if brace_depth == 0 {
                    let start = json_start?;
                    let json = response[start..=index].to_owned();
                    if !response[..start].trim().is_empty() {
                        warnings.push("Extra text detected before JSON object".into());
                    }
                    if !response[index + 1..].trim().is_empty() {
                        warnings.push("Extra text detected after JSON object".into());
                    }
                    return Some(json);
                }
            }
            _ => {}
        }
    }

    None
}

fn combine_warnings(warnings: Vec<String>) -> Option<String> {
    if warnings.is_empty() {
        None
    } else {
        Some(
            warnings
                .into_iter()
                .map(|warning| format!("- {warning}"))
                .collect::<Vec<_>>()
                .join("\n"),
        )
    }
}

fn combine_warning_parts(left: Option<String>, right: Option<String>) -> Option<String> {
    match (left, right) {
        (Some(left), Some(right)) => Some(format!("- {left}\n{right}")),
        (Some(left), None) => Some(format!("- {left}")),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

fn render_template(template: &str, replacements: &[(&str, &str)]) -> String {
    let mut rendered = template.to_owned();
    for (key, value) in replacements {
        rendered = rendered.replace(&format!("{{{key}}}"), value);
    }
    rendered.replace("{{", "{").replace("}}", "}")
}

fn format_json_error(content: &str, error: &serde_json::Error) -> String {
    if content.len() < 200 {
        format!("Invalid JSON: {error} | Content: {:?}", content)
    } else {
        format!(
            "Invalid JSON: {error} | Content preview: {:?}...",
            &content[..100]
        )
    }
}

fn auto_fix_json_response(response: &str, error: &str) -> Vec<(String, String)> {
    let mut fixes = Vec::new();

    if error.contains("Invalid JSON")
        || error.contains("Expecting")
        || error.contains("Unterminated")
        || error.contains("No valid JSON found")
    {
        let brace_count =
            response.matches('{').count() as isize - response.matches('}').count() as isize;
        if brace_count > 0 {
            fixes.push((
                "Fixed incomplete JSON by adding missing closing brace".into(),
                format!("{response}{}", "}".repeat(brace_count as usize)),
            ));
        }
    }

    if let Some(json_content) = extract_json_candidate_from_mixed_content(response) {
        fixes.push(("Extracted JSON from mixed content".into(), json_content));
    }

    fixes
}

fn extract_json_candidate_from_mixed_content(response: &str) -> Option<String> {
    let mut candidates = Vec::new();
    let mut start = 0;
    while let Some(open) = response[start..].find('{') {
        let absolute_open = start + open;
        let mut depth = 0usize;
        let mut in_string = false;
        let mut escaped = false;
        for (offset, ch) in response[absolute_open..].char_indices() {
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == '"' {
                in_string = !in_string;
                continue;
            }
            if in_string {
                continue;
            }
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        let candidate = response[absolute_open..=absolute_open + offset].to_owned();
                        if serde_json::from_str::<serde_json::Value>(&candidate).is_ok() {
                            candidates.push(candidate);
                        }
                        break;
                    }
                }
                _ => {}
            }
        }
        start = absolute_open + 1;
    }

    candidates.into_iter().next()
}

fn check_field_order(response: &str, warnings: &mut Vec<String>) {
    let expected_order = ["analysis", "plan", "commands"];
    let mut positions = Vec::new();

    for field in expected_order {
        let pattern = format!("\"{field}\"");
        if let Some(position) = response.find(&pattern) {
            positions.push((field, position));
        }
    }

    if positions.len() < 2 {
        return;
    }

    let mut actual = positions;
    actual.sort_by_key(|(_, position)| *position);
    let actual_order = actual
        .into_iter()
        .map(|(field, _)| field)
        .collect::<Vec<_>>();
    let expected_present = expected_order
        .into_iter()
        .filter(|field| actual_order.contains(field))
        .collect::<Vec<_>>();

    if actual_order != expected_present {
        warnings.push(format!(
            "Fields appear in wrong order. Found: {}, expected: {}",
            actual_order.join(" -> "),
            expected_present.join(" -> ")
        ));
    }
}

fn repair_prompt(error: &str, warning: Option<&str>) -> String {
    match warning {
        Some(warning) => format!(
            "Previous response had parsing errors:\nERROR: {error}\nWARNINGS: {warning}\n\nPlease fix these issues and provide a proper JSON response."
        ),
        None => format!(
            "Previous response had parsing errors:\nERROR: {error}\n\nPlease fix these issues and provide a proper JSON response."
        ),
    }
}

async fn summarize_history(
    provider: &dyn Provider,
    chat: &mut Vec<Message>,
    inference: &InferenceConfig,
    session_id: Option<Ulid>,
    session_key: &str,
    terminal_tool: &Arc<dyn Tool>,
    tx: &mpsc::Sender<Event>,
    cancel: &CancellationToken,
) -> Result<Option<SummaryOutcome>, BrainError> {
    if chat.is_empty() {
        return Ok(None);
    }

    let original_instruction = chat
        .iter()
        .find(|message| matches!(message.role, Role::User))
        .map(|message| message.content.clone())
        .unwrap_or_default();
    let current_screen = terminal_capture(terminal_tool.as_ref(), session_key, 10_000).await?;

    let _ = tx
        .send(Event::Progress {
            phase: "summarization".into(),
            message: "Generating Terminus-2 handoff summary".into(),
            percent: Some(0.33),
        })
        .await;
    let summary_prompt = format!(
        "You are about to hand off your work to another AI agent.\nPlease provide a comprehensive summary of what you have accomplished so far on this task:\n\nOriginal Task: {original_instruction}\n\nBased on the conversation history, please provide a detailed summary covering:\n1. Major Actions Completed\n2. Important Information Learned\n3. Challenging Problems Addressed\n4. Current Status"
    );
    let mut summary_history = chat.clone();
    summary_history.push(Message::user(summary_prompt.clone()));
    let summary = query_provider(
        provider,
        &summary_history,
        inference,
        0,
        session_id,
        tx,
        cancel,
        false,
    )
    .await?;

    let _ = tx
        .send(Event::Progress {
            phase: "summarization".into(),
            message: "Generating successor questions".into(),
            percent: Some(0.66),
        })
        .await;
    let question_prompt = format!(
        "You are picking up work from a previous AI agent on this task:\n\nOriginal Task: {original_instruction}\n\nSummary from Previous Agent:\n{}\n\nCurrent Terminal Screen:\n{}\n\nPlease begin by asking several questions (at least five, more if necessary) about the current state of the solution that are not answered in the summary from the prior agent.",
        summary.content, current_screen.terminal_state
    );
    let questions = query_provider(
        provider,
        &[Message::user(question_prompt.clone())],
        inference,
        0,
        session_id,
        tx,
        cancel,
        false,
    )
    .await?;

    let _ = tx
        .send(Event::Progress {
            phase: "summarization".into(),
            message: "Answering successor questions".into(),
            percent: Some(1.0),
        })
        .await;
    let mut answers_history = chat.clone();
    answers_history.push(Message::user(summary_prompt));
    answers_history.push(Message::assistant(summary.content.clone()));
    let answer_request = format!(
        "The next agent has a few questions for you, please answer each of them one by one in detail:\n\n{}",
        questions.content
    );
    answers_history.push(Message::user(answer_request));
    let answers = query_provider(
        provider,
        &answers_history,
        inference,
        0,
        session_id,
        tx,
        cancel,
        false,
    )
    .await?;

    let mut usage = UsageTotals::default();
    if let Some(ref usage_info) = summary.usage {
        usage.add(usage_info);
    }
    if let Some(ref usage_info) = questions.usage {
        usage.add(usage_info);
    }
    if let Some(ref usage_info) = answers.usage {
        usage.add(usage_info);
    }

    let system_messages = chat
        .iter()
        .filter(|message| matches!(message.role, Role::System))
        .cloned()
        .collect::<Vec<_>>();
    let handoff_prompt = format!(
        "Here are the answers the other agent provided.\n\n{}\n\nContinue working on this task from where the previous agent left off. You can no longer ask questions. Please follow the spec to interact with the terminal.",
        answers.content
    );
    *chat = system_messages;
    chat.push(Message::user(question_prompt));
    chat.push(Message::assistant(questions.content));

    Ok(Some(SummaryOutcome {
        summary_tokens: estimate_tokens(&handoff_prompt) as usize,
        handoff_prompt,
        usage,
    }))
}

fn should_proactively_summarize(
    provider: &dyn Provider,
    chat: &[Message],
    inference: &InferenceConfig,
) -> bool {
    let context_limit = active_model_limit(provider, inference)
        .map(|limit| limit.context)
        .unwrap_or(0);
    if context_limit == 0 {
        return false;
    }

    let used_tokens = estimate_tokens_for_messages(chat);
    context_limit.saturating_sub(used_tokens) < DEFAULT_PROACTIVE_SUMMARIZATION_THRESHOLD
}

fn unwind_messages_to_free_tokens(
    provider: &dyn Provider,
    inference: &InferenceConfig,
    chat: &mut Vec<Message>,
    target_free_tokens: u64,
) {
    let Some(context_limit) = active_model_limit(provider, inference).map(|limit| limit.context)
    else {
        return;
    };

    while chat.len() > 1 {
        let used_tokens = estimate_tokens_for_messages(chat);
        let free_tokens = context_limit.saturating_sub(used_tokens);
        if free_tokens >= target_free_tokens {
            break;
        }

        if chat.len() >= 3 {
            chat.truncate(chat.len() - 2);
        } else {
            break;
        }
    }
}

async fn summarize_with_fallback(
    provider: &dyn Provider,
    chat: &mut Vec<Message>,
    inference: &InferenceConfig,
    session_id: Option<Ulid>,
    session_key: &str,
    terminal_tool: &Arc<dyn Tool>,
    tx: &mpsc::Sender<Event>,
    cancel: &CancellationToken,
) -> Result<Option<SummaryOutcome>, BrainError> {
    if let Some(summary) = summarize_history(
        provider,
        chat,
        inference,
        session_id,
        session_key,
        terminal_tool,
        tx,
        cancel,
    )
    .await?
    {
        return Ok(Some(summary));
    }

    let original_instruction = chat
        .iter()
        .find(|message| matches!(message.role, Role::User))
        .map(|message| message.content.clone())
        .unwrap_or_default();
    let current_screen = terminal_capture(terminal_tool.as_ref(), session_key, 10_000).await?;
    let limited_screen =
        truncate_tool_result(&current_screen.terminal_state, &AgentConfig::default());
    let short_prompt = format!(
        "Briefly continue this task: {original_instruction}\n\nCurrent state: {limited_screen}\n\nNext steps (2-3 sentences):"
    );
    let short_summary = query_provider(
        provider,
        &[Message::user(short_prompt)],
        inference,
        0,
        session_id,
        tx,
        cancel,
        false,
    )
    .await
    .ok()
    .map(|turn| turn.content);

    let handoff_prompt = match short_summary {
        Some(summary) => format!("{original_instruction}\n\nSummary: {summary}"),
        None => format!("{original_instruction}\n\nCurrent state: {limited_screen}"),
    };

    Ok(Some(SummaryOutcome {
        summary_tokens: estimate_tokens(&handoff_prompt) as usize,
        handoff_prompt,
        usage: UsageTotals::default(),
    }))
}

fn estimate_tokens_for_messages(messages: &[Message]) -> u64 {
    messages
        .iter()
        .map(|message| {
            estimate_tokens(&message.content_text_lossy())
                + message
                    .reasoning_content
                    .as_deref()
                    .map(estimate_tokens)
                    .unwrap_or(0)
        })
        .sum()
}

fn estimate_tokens(text: &str) -> u64 {
    ((text.len() as u64) / 4).max(1)
}

fn active_model_limit(provider: &dyn Provider, inference: &InferenceConfig) -> Option<ModelLimit> {
    let info = provider.info();
    let model_id = inference
        .model
        .as_deref()
        .or(info.default_model.as_deref())?;

    info.models
        .iter()
        .find(|model| model.id == model_id)
        .and_then(|model| model.limit)
}

fn output_length_retry_prompt(provider: &dyn Provider, inference: &InferenceConfig) -> String {
    let limit = active_model_limit(provider, inference)
        .map(|limit| format!("{} tokens", limit.output))
        .unwrap_or_else(|| "the model output limit".to_owned());

    format!(
        "ERROR!! NONE of the actions you just requested were performed because you exceeded {limit}. Your outputs must be less than {limit}. Re-issue this request, breaking it into chunks that fit within {limit}."
    )
}

fn effective_max_iterations(config: &AgentConfig) -> u32 {
    if config.max_iterations == AgentConfig::default().max_iterations {
        DEFAULT_TERMINUS_MAX_ITERATIONS
    } else {
        config.max_iterations
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    use super::*;
    use async_stream::stream;
    use futures::StreamExt;
    use futures::future::BoxFuture;

    const TEST_MODEL: ModelInfo = ModelInfo {
        id: "test-model",
        name: "Test Model",
        family: None,
        reasoning: None,
        tool_call: false,
        attachment: false,
        structured_output: None,
        temperature: None,
        knowledge: None,
        release_date: None,
        last_updated: None,
        open_weights: None,
        input_modalities: &["text"],
        output_modalities: &["text"],
        cost: None,
        limit: Some(ModelLimit {
            context: 16_000,
            input: None,
            output: 4_096,
        }),
        status: None,
    };

    #[derive(Clone)]
    enum ScriptedProviderStep {
        Response(&'static str),
        Error(&'static str),
    }

    struct ScriptedProvider {
        steps: Mutex<VecDeque<ScriptedProviderStep>>,
        seen_messages: Mutex<Vec<Vec<Message>>>,
    }

    impl ScriptedProvider {
        fn new(steps: impl IntoIterator<Item = ScriptedProviderStep>) -> Self {
            Self {
                steps: Mutex::new(steps.into_iter().collect()),
                seen_messages: Mutex::new(Vec::new()),
            }
        }
    }

    impl Provider for ScriptedProvider {
        fn chat<'a>(
            &'a self,
            messages: &'a [Message],
            _tools: &'a [ToolDef],
            _config: &'a InferenceConfig,
            _session_id: Option<Ulid>,
        ) -> BoxFuture<'a, Result<ChatStream, BrainError>> {
            self.seen_messages.lock().unwrap().push(messages.to_vec());
            let step = self
                .steps
                .lock()
                .unwrap()
                .pop_front()
                .expect("provider called more times than scripted");

            Box::pin(async move {
                match step {
                    ScriptedProviderStep::Error(message) => {
                        Err(BrainError::Inference(message.into()))
                    }
                    ScriptedProviderStep::Response(content) => {
                        let content = content.to_owned();
                        let response_stream = stream! {
                            yield Ok(ChatChunk::Delta { content });
                            yield Ok(ChatChunk::Done {
                                usage: Some(TokenUsage {
                                    prompt: 10,
                                    completion: 5,
                                    total: 15,
                                    cache_read: None,
                                    cache_write: None,
                                    reasoning: None,
                                }),
                            });
                        };
                        Ok(Box::pin(response_stream) as ChatStream)
                    }
                }
            })
        }

        fn info(&self) -> ProviderInfo {
            ProviderInfo {
                name: "scripted".into(),
                default_model: Some(TEST_MODEL.id.into()),
                models: vec![TEST_MODEL],
            }
        }
    }

    struct FakeTerminalTool {
        state: Arc<Mutex<String>>,
    }

    impl FakeTerminalTool {
        fn new(initial_state: impl Into<String>) -> Self {
            Self {
                state: Arc::new(Mutex::new(initial_state.into())),
            }
        }
    }

    impl Tool for FakeTerminalTool {
        fn definition(&self) -> ToolDef {
            ToolDef {
                name: "terminal_session".into(),
                description: "fake terminal".into(),
                parameters: serde_json::json!({}),
            }
        }

        fn execute(&self, args: serde_json::Value) -> BoxFuture<'_, Result<String, BrainError>> {
            let state = self.state.clone();

            Box::pin(async move {
                let action = args
                    .get("action")
                    .and_then(|value| value.as_str())
                    .unwrap_or("capture");
                let mut terminal = state.lock().unwrap();

                match action {
                    "capture" | "close" => Ok(serde_json::json!({
                        "timed_out": false,
                        "terminal_state": terminal.clone(),
                        "observation": terminal.clone(),
                    })
                    .to_string()),
                    "send_keys" => {
                        let keystrokes = args
                            .get("keystrokes")
                            .and_then(|value| value.as_str())
                            .unwrap_or_default();
                        let trimmed = keystrokes.trim();
                        *terminal = if trimmed.contains("echo hi") {
                            "hi".into()
                        } else if trimmed.is_empty() {
                            terminal.clone()
                        } else {
                            format!("ran: {trimmed}")
                        };

                        Ok(serde_json::json!({
                            "timed_out": false,
                            "terminal_state": terminal.clone(),
                            "observation": terminal.clone(),
                        })
                        .to_string())
                    }
                    other => Err(BrainError::ToolFailed {
                        tool: "terminal_session".into(),
                        reason: format!("unsupported fake action {other}"),
                    }),
                }
            })
        }
    }

    #[test]
    fn parses_valid_json_response() {
        let parsed = parse_json_response(
            r#"{
                "analysis": "Inspect repo state",
                "plan": "Run ls",
                "commands": [{"keystrokes": "ls\n", "duration": 0.1}],
                "task_complete": false
            }"#,
        );

        assert!(parsed.error.is_none());
        assert_eq!(parsed.analysis, "Inspect repo state");
        assert_eq!(parsed.plan, "Run ls");
        assert_eq!(parsed.commands.len(), 1);
        assert!(!parsed.is_task_complete);
    }

    #[test]
    fn rejects_missing_required_fields() {
        let parsed = parse_json_response(r#"{"commands": []}"#);
        assert!(parsed.error.is_some());
    }

    #[test]
    fn extracts_json_with_extra_text() {
        let parsed = parse_json_response(
            "Some explanation\n{\"analysis\":\"a\",\"plan\":\"b\",\"commands\":[]}\nTrailing text",
        );
        assert!(parsed.error.is_none());
        assert!(parsed.warning.is_some());
    }

    #[test]
    fn renders_harbor_prompt_template() {
        let rendered = render_template(
            TERMINUS_JSON_PROMPT_TEMPLATE,
            &[("instruction", "solve it"), ("terminal_state", "$ ")],
        );

        assert!(rendered.contains("\"analysis\": \"Analyze the current state"));
        assert!(rendered.contains("Task Description:\nsolve it"));
        assert!(rendered.contains("Current terminal state:\n$ "));
        assert!(!rendered.contains("install missing tools"));
    }

    #[test]
    fn uses_large_default_iteration_budget_for_terminus2() {
        assert_eq!(
            effective_max_iterations(&AgentConfig::default()),
            DEFAULT_TERMINUS_MAX_ITERATIONS
        );
    }

    #[tokio::test]
    async fn query_provider_retries_generic_errors() {
        let provider = ScriptedProvider::new([
            ScriptedProviderStep::Error("temporary upstream error"),
            ScriptedProviderStep::Response(r#"{"analysis":"ok","plan":"continue","commands":[]}"#),
        ]);
        let (tx, mut rx) = mpsc::channel(8);

        let turn = query_provider(
            &provider,
            &[Message::user("task")],
            &InferenceConfig::default(),
            1,
            None,
            &tx,
            &CancellationToken::new(),
            false,
        )
        .await
        .unwrap();

        assert!(turn.content.contains("\"analysis\":\"ok\""));
        match rx.recv().await.expect("retry event") {
            Event::Retry { attempt, max, .. } => {
                assert_eq!(attempt, 1);
                assert_eq!(max, 2);
            }
            other => panic!("expected retry event, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn summarize_history_rewrites_chat_into_handoff_form() {
        let provider = ScriptedProvider::new([
            ScriptedProviderStep::Response("Summary of completed work"),
            ScriptedProviderStep::Response("1. What changed?\n2. What remains?"),
            ScriptedProviderStep::Response("Answers from the predecessor agent"),
        ]);
        let terminal_tool: Arc<dyn Tool> = Arc::new(FakeTerminalTool::new("$ "));
        let (tx, _rx) = mpsc::channel(8);
        let mut chat = vec![
            Message::system("system"),
            Message::user("original task"),
            Message::assistant("Analysis: explored repo\nPlan: continue"),
        ];

        let summary = summarize_history(
            &provider,
            &mut chat,
            &InferenceConfig::default(),
            None,
            "session-1",
            &terminal_tool,
            &tx,
            &CancellationToken::new(),
        )
        .await
        .unwrap()
        .expect("summary outcome");

        assert!(
            summary
                .handoff_prompt
                .contains("Answers from the predecessor agent")
        );
        assert!(summary.summary_tokens > 0);
        assert_eq!(chat.len(), 3);
        assert!(matches!(chat[0].role, Role::System));
        assert!(
            chat[1]
                .content
                .to_string()
                .contains("Summary from Previous Agent")
        );
        assert_eq!(
            chat[2].content,
            MessageContent::text("1. What changed?\n2. What remains?")
        );
    }

    #[tokio::test]
    async fn completion_requires_two_task_complete_responses() {
        let provider: Arc<dyn Provider> = Arc::new(ScriptedProvider::new([
            ScriptedProviderStep::Response(
                r#"{
                    "analysis": "Command ran successfully",
                    "plan": "Ask to complete",
                    "commands": [{"keystrokes": "echo hi\n", "duration": 0.0}],
                    "task_complete": true
                }"#,
            ),
            ScriptedProviderStep::Response(
                r#"{
                    "analysis": "Confirmed final state",
                    "plan": "Finish",
                    "commands": [],
                    "task_complete": true
                }"#,
            ),
        ]));
        let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(FakeTerminalTool::new("$ "))];
        let stream = Terminus2Loop.run(
            provider,
            tools,
            vec![Message::user("solve the task")],
            AgentConfig::default(),
            CancellationToken::new(),
            Some(Ulid::new()),
        );

        let events = stream.collect::<Vec<_>>().await;
        let completion_starts = events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    Event::ToolCallStart { name, .. } if name == "mark_task_complete"
                )
            })
            .count();
        let confirmation_prompt_seen = events.iter().any(|event| {
            matches!(
                event,
                Event::ToolCallDone { result, .. }
                    if result.contains("Are you sure you want to mark the task as complete?")
            )
        });

        assert_eq!(completion_starts, 2);
        assert!(confirmation_prompt_seen);
        assert!(
            events
                .iter()
                .any(|event| matches!(event, Event::TurnDone { .. }))
        );
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, Event::Error { .. }))
        );
    }

    #[tokio::test]
    async fn provider_history_stays_text_only_between_turns() {
        let provider = Arc::new(ScriptedProvider::new([
            ScriptedProviderStep::Response(
                r#"{
                    "analysis": "Inspect terminal",
                    "plan": "Run a quick command",
                    "commands": [{"keystrokes": "echo hi\n", "duration": 0.0}],
                    "task_complete": false
                }"#,
            ),
            ScriptedProviderStep::Response(
                r#"{
                    "analysis": "Ready to finish",
                    "plan": "Request completion",
                    "commands": [],
                    "task_complete": true
                }"#,
            ),
            ScriptedProviderStep::Response(
                r#"{
                    "analysis": "Final confirmation",
                    "plan": "Finish",
                    "commands": [],
                    "task_complete": true
                }"#,
            ),
        ]));
        let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(FakeTerminalTool::new("$ "))];
        let stream = Terminus2Loop.run(
            provider.clone(),
            tools,
            vec![Message::user("solve the task")],
            AgentConfig::default(),
            CancellationToken::new(),
            Some(Ulid::new()),
        );

        let events = stream.collect::<Vec<_>>().await;
        assert!(
            events
                .iter()
                .any(|event| matches!(event, Event::TurnDone { .. }))
        );

        let seen_messages = provider.seen_messages.lock().unwrap();
        assert!(seen_messages.len() >= 2);

        let second_turn = &seen_messages[1];
        let assistant = second_turn
            .iter()
            .find(|message| matches!(message.role, Role::Assistant))
            .expect("assistant message should be present on second turn");
        assert!(assistant.tool_calls.is_empty());
        assert!(
            assistant
                .content
                .to_string()
                .contains("Analysis: Inspect terminal")
        );
    }
}
