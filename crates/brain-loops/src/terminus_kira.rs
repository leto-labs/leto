use std::sync::Arc;

use futures::StreamExt;
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio::time::{Duration, Instant, sleep, timeout};
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::CancellationToken;
use ulid::Ulid;

use brain_types::*;

use crate::output::truncate_tool_result;

const KIRA_PROMPT_TEMPLATE: &str =
    include_str!("terminus_kira/templates/terminus-kira.txt");
const TERMINUS_TIMEOUT_TEMPLATE: &str = include_str!("terminus2/templates/timeout.txt");
const DEFAULT_KIRA_MAX_ITERATIONS: u32 = 1_000_000;
const BLOCK_TIMEOUT: Duration = Duration::from_secs(30);
const POLL_INTERVAL: Duration = Duration::from_millis(250);
const MAX_COMMAND_DURATION_SECONDS: f64 = 60.0;

const COMPLETION_CONFIRMATION_TEMPLATE: &str = r#"Original task:
{instruction}

Current terminal state:
{terminal_state}

Are you sure you want to mark the task as complete?

[!] Checklist
- Does your solution meet the requirements in the original task above? [TODO/DONE]
- Does your solution account for potential changes in numeric values, array sizes, file contents, or configuration parameters? [TODO/DONE]
- Have you verified your solution from the all perspectives of a test engineer, a QA engineer, and the user who requested this task?
  - test engineer [TODO/DONE]
  - QA engineer [TODO/DONE]
  - user who requested this task [TODO/DONE]

After this point, solution grading will begin and no further edits will be possible. If everything looks good, call task_complete tool again."#;

pub struct TerminusKiraLoop;

impl AgentLoop for TerminusKiraLoop {
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
struct CommandRequest {
    keystrokes: String,
    duration_seconds: f64,
}

#[derive(Debug, Clone)]
struct ImageReadRequest {
    file_path: String,
    instruction: String,
}

#[derive(Debug, Clone, Default)]
struct ParsedToolTurn {
    execute_call_id: Option<String>,
    completion_call_id: Option<String>,
    image_call_id: Option<String>,
    commands: Vec<CommandRequest>,
    is_task_complete: bool,
    feedback: Option<String>,
    analysis: String,
    plan: String,
    image_read: Option<ImageReadRequest>,
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
}

#[derive(Debug, Clone)]
struct ProviderTurn {
    content: String,
    tool_calls: Vec<ToolCall>,
    usage: Option<TokenUsage>,
}

#[derive(Debug, Clone, Deserialize)]
struct TerminalObservation {
    #[serde(default)]
    timed_out: bool,
    terminal_state: String,
    observation: String,
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
    let shell_tool = tools
        .iter()
        .find(|tool| tool.definition().name == "shell")
        .cloned();

    let session_key = session_id
        .map(|id| id.to_string())
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
        KIRA_PROMPT_TEMPLATE,
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
        message.set_text_content(initial_prompt);
        message.reasoning_content = None;
    } else {
        messages.push(Message::user(initial_prompt));
    }

    let mut chat = messages;
    let mut iterations = 0u32;
    let mut pending_completion = false;
    let mut usage = UsageTotals::default();

    loop {
        if cancel.is_cancelled() {
            return Err(BrainError::Cancelled);
        }
        if iterations >= effective_max_iterations(&config) {
            return Err(BrainError::MaxIterations(iterations));
        }
        iterations += 1;

        let turn = query_provider(
            provider.as_ref(),
            &chat,
            &kira_tool_defs(),
            &config.inference,
            config.max_retries,
            session_id,
            &tx,
            &cancel,
        )
        .await?;
        if let Some(ref usage_info) = turn.usage {
            usage.add(usage_info);
        }

        let parsed = parse_tool_calls(&turn.tool_calls);
        let assistant_text = if !turn.content.trim().is_empty() {
            turn.content.clone()
        } else if !parsed.analysis.is_empty() || !parsed.plan.is_empty() {
            format!("Analysis: {}\nPlan: {}", parsed.analysis, parsed.plan)
        } else if let Some(feedback) = parsed.feedback.as_deref() {
            feedback.to_owned()
        } else {
            "Used native tool calling".to_owned()
        };

        let mut assistant = Message::assistant(assistant_text);
        assistant.tool_calls = turn.tool_calls.clone();
        let _ = tx
            .send(Event::MessageDone {
                message: assistant.clone(),
            })
            .await;
        chat.push(assistant);

        if let Some(feedback) = parsed.feedback.as_deref()
            && parsed.commands.is_empty()
            && parsed.image_read.is_none()
            && !parsed.is_task_complete
        {
            pending_completion = false;
            chat.push(Message::user(feedback));
            continue;
        }

        let mut observation = None::<String>;
        let mut terminal_state_for_completion = initial_screen.terminal_state.clone();

        if let Some(image_read) = parsed.image_read.as_ref() {
            let call_id = parsed
                .image_call_id
                .clone()
                .unwrap_or_else(|| format!("call_{iterations}_image_read"));
            emit_tool_lifecycle(
                &call_id,
                "image_read",
                serde_json::json!({
                    "file_path": image_read.file_path,
                    "image_read_instruction": image_read.instruction,
                }),
                &tx,
            )
            .await;
            let result = execute_image_read(
                provider.as_ref(),
                shell_tool.as_deref(),
                image_read,
                &config.inference,
                session_id,
                &cancel,
            )
            .await?;
            let truncated = truncate_tool_result(&result, &config);
            let _ = tx
                .send(Event::ToolCallDone {
                    id: call_id,
                    result: truncated.clone(),
                    is_error: false,
                })
                .await;
            observation = Some(result);
        } else if !parsed.commands.is_empty() {
            let call_id = parsed
                .execute_call_id
                .clone()
                .unwrap_or_else(|| format!("call_{iterations}_execute"));
            emit_tool_lifecycle(
                &call_id,
                "execute_commands",
                serde_json::json!({
                    "analysis": parsed.analysis,
                    "plan": parsed.plan,
                    "commands": parsed
                        .commands
                        .iter()
                        .map(|command| serde_json::json!({
                            "keystrokes": command.keystrokes,
                            "duration": command.duration_seconds,
                        }))
                        .collect::<Vec<_>>(),
                }),
                &tx,
            )
            .await;
            let executed = execute_command_batch(
                &*terminal_tool,
                &session_key,
                &parsed.commands,
                config.tool_output_max_bytes,
            )
            .await?;
            terminal_state_for_completion = executed.terminal_state.clone();
            let truncated = truncate_tool_result(&executed.observation, &config);
            let _ = tx
                .send(Event::ToolCallDone {
                    id: call_id,
                    result: truncated,
                    is_error: false,
                })
                .await;
            observation = Some(executed.observation);
        } else if !parsed.is_task_complete {
            let capture =
                terminal_capture(&*terminal_tool, &session_key, config.tool_output_max_bytes).await?;
            terminal_state_for_completion = capture.terminal_state.clone();
            observation = Some(capture.observation);
        }

        if parsed.is_task_complete {
            let call_id = parsed
                .completion_call_id
                .clone()
                .unwrap_or_else(|| format!("call_{iterations}_task_complete"));
            emit_tool_lifecycle(&call_id, "task_complete", serde_json::json!({}), &tx).await;

            let completion_message = if pending_completion {
                terminal_state_for_completion.clone()
            } else {
                render_template(
                    COMPLETION_CONFIRMATION_TEMPLATE,
                    &[
                        ("instruction", original_instruction.as_str()),
                        ("terminal_state", terminal_state_for_completion.as_str()),
                    ],
                )
            };
            let truncated = truncate_tool_result(&completion_message, &config);
            let _ = tx
                .send(Event::ToolCallDone {
                    id: call_id,
                    result: truncated.clone(),
                    is_error: false,
                })
                .await;

            if pending_completion {
                break;
            }

            pending_completion = true;
            chat.push(Message::user(completion_message));
            continue;
        }

        pending_completion = false;
        if let Some(observation) = observation {
            chat.push(Message::user(observation));
        }
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

fn kira_tool_defs() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "execute_commands".into(),
            description: "Run one or more shell command batches with analysis and plan.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "analysis": { "type": "string" },
                    "plan": { "type": "string" },
                    "commands": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "keystrokes": { "type": "string" },
                                "duration": { "type": "number" }
                            },
                            "required": ["keystrokes", "duration"]
                        }
                    }
                },
                "required": ["analysis", "plan", "commands"]
            }),
        },
        ToolDef {
            name: "task_complete".into(),
            description: "Signal that the task is complete and ready for verification.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        },
        ToolDef {
            name: "image_read".into(),
            description: "Analyze an image file by path using multimodal model input.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": { "type": "string" },
                    "image_read_instruction": { "type": "string" }
                },
                "required": ["file_path", "image_read_instruction"]
            }),
        },
    ]
}

fn parse_tool_calls(tool_calls: &[ToolCall]) -> ParsedToolTurn {
    let mut parsed = ParsedToolTurn::default();

    if tool_calls.is_empty() {
        parsed.feedback = Some(
            "WARNINGS: Your response contained no tool calls. Please use execute_commands to run commands."
                .into(),
        );
        return parsed;
    }

    for tool_call in tool_calls {
        match tool_call.name.as_str() {
            "execute_commands" => {
                parsed.execute_call_id = Some(tool_call.id.clone());
                parsed.analysis = tool_call
                    .arguments
                    .get("analysis")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
                    .to_owned();
                parsed.plan = tool_call
                    .arguments
                    .get("plan")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
                    .to_owned();
                let commands = tool_call
                    .arguments
                    .get("commands")
                    .and_then(|value| value.as_array())
                    .cloned()
                    .unwrap_or_default();
                parsed.commands.extend(commands.into_iter().filter_map(|command| {
                    let keystrokes = command.get("keystrokes")?.as_str()?.to_owned();
                    let duration = command
                        .get("duration")
                        .and_then(|value| value.as_f64())
                        .unwrap_or(1.0)
                        .min(MAX_COMMAND_DURATION_SECONDS);
                    Some(CommandRequest {
                        keystrokes,
                        duration_seconds: duration,
                    })
                }));
            }
            "task_complete" => {
                parsed.completion_call_id = Some(tool_call.id.clone());
                parsed.is_task_complete = true;
            }
            "image_read" => {
                parsed.image_call_id = Some(tool_call.id.clone());
                let file_path = tool_call
                    .arguments
                    .get("file_path")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default();
                let instruction = tool_call
                    .arguments
                    .get("image_read_instruction")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default();
                if file_path.is_empty() || instruction.is_empty() {
                    parsed.feedback = Some(
                        "WARNINGS: image_read requires both file_path and image_read_instruction arguments."
                            .into(),
                    );
                } else {
                    parsed.image_read = Some(ImageReadRequest {
                        file_path: file_path.to_owned(),
                        instruction: instruction.to_owned(),
                    });
                }
            }
            other => {
                parsed.feedback = Some(format!(
                    "WARNINGS: Unknown function '{other}'. Please use execute_commands, task_complete, or image_read."
                ));
            }
        }
    }

    parsed
}

async fn query_provider(
    provider: &dyn Provider,
    messages: &[Message],
    tools: &[ToolDef],
    inference: &InferenceConfig,
    max_retries: u32,
    session_id: Option<Ulid>,
    tx: &mpsc::Sender<Event>,
    cancel: &CancellationToken,
) -> Result<ProviderTurn, BrainError> {
    let max_attempts = max_retries.saturating_add(1);

    'attempt: for attempt in 1..=max_attempts {
        if cancel.is_cancelled() {
            return Err(BrainError::Cancelled);
        }

        let mut stream = match provider.chat(messages, tools, inference, session_id).await {
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
        let mut tool_calls = Vec::new();
        let mut saw_output = false;

        while let Some(chunk) = stream.next().await {
            if cancel.is_cancelled() {
                return Err(BrainError::Cancelled);
            }
            match chunk {
                Ok(ChatChunk::Delta { content: delta }) => {
                    saw_output = true;
                    content.push_str(&delta);
                    let _ = tx.send(Event::Token { delta }).await;
                }
                Ok(ChatChunk::ToolCallDelta {
                    id,
                    name,
                    arguments_delta,
                }) => {
                    saw_output = true;
                    let _ = tx
                        .send(Event::ToolCallDelta {
                            id,
                            name,
                            arguments_delta,
                        })
                        .await;
                }
                Ok(ChatChunk::ToolCall {
                    id,
                    name,
                    arguments,
                }) => {
                    saw_output = true;
                    tool_calls.push(ToolCall {
                        id,
                        name,
                        arguments,
                    });
                }
                Ok(ChatChunk::Done { usage: chunk_usage }) => {
                    saw_output = true;
                    usage = chunk_usage;
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
                Err(error) => return Err(error),
            }
        }

        return Ok(ProviderTurn {
            content,
            tool_calls,
            usage,
        });
    }

    Err(BrainError::Inference(
        "provider retry loop exhausted unexpectedly".into(),
    ))
}

async fn execute_command_batch(
    terminal_tool: &dyn Tool,
    session_key: &str,
    commands: &[CommandRequest],
    max_bytes: usize,
) -> Result<TerminalObservation, BrainError> {
    let mut latest = terminal_capture(terminal_tool, session_key, max_bytes).await?;
    for (index, command) in commands.iter().enumerate() {
        latest = if parse_logical_keys(&command.keystrokes).is_some() {
            terminal_send_keys(terminal_tool, session_key, command, max_bytes).await?
        } else {
            terminal_send_keys_with_marker(terminal_tool, session_key, command, index, max_bytes)
                .await?
        };
    }
    Ok(latest)
}

async fn execute_image_read(
    provider: &dyn Provider,
    shell_tool: Option<&dyn Tool>,
    image_read: &ImageReadRequest,
    inference: &InferenceConfig,
    session_id: Option<Ulid>,
    cancel: &CancellationToken,
) -> Result<String, BrainError> {
    let shell_tool = shell_tool.ok_or_else(|| BrainError::ToolNotFound("shell".into()))?;
    let mime = infer_image_mime(&image_read.file_path).ok_or_else(|| BrainError::ToolFailed {
        tool: "image_read".into(),
        reason: "unsupported image format; convert to png, jpg, gif, or webp first".into(),
    })?;

    let escaped_path = shell_single_quote(&image_read.file_path);
    let raw = with_block_timeout(
        shell_tool.execute(serde_json::json!({
            "command": format!("base64 '{escaped_path}' | tr -d '\\n'"),
        })),
        "image_read base64 shell command",
    )
    .await?;

    let mut multimodal_inference = inference.clone();
    multimodal_inference.reasoning = None;

    let data_url = format!("data:{mime};base64,{}", raw.trim());
    let messages = vec![Message::user_parts(vec![
        ContentPart::Text {
            text: image_read.instruction.clone(),
        },
        ContentPart::ImageUrl { url: data_url },
    ])];
    let turn = query_provider(
        provider,
        &messages,
        &[],
        &multimodal_inference,
        0,
        session_id,
        &mpsc::channel(1).0,
        cancel,
    )
    .await?;

    Ok(format!(
        "File Read Result for '{}':\n{}",
        image_read.file_path, turn.content
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
    command: &CommandRequest,
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
                "keystrokes": normalize_command_keystrokes(&command.keystrokes),
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

async fn terminal_send_keys_with_marker(
    terminal_tool: &dyn Tool,
    session_key: &str,
    command: &CommandRequest,
    marker_index: usize,
    max_bytes: usize,
) -> Result<TerminalObservation, BrainError> {
    let marker = format!("__CMDEND__{}__", marker_index);
    let mut send_text = normalize_command_keystrokes(&command.keystrokes);
    send_text.push_str(&format!("printf '\\n{marker}\\n'\n"));
    let _ = invoke_terminal_tool(
        terminal_tool,
        serde_json::json!({
            "session_id": session_key,
            "action": "send_keys",
            "keystrokes": send_text,
            "min_wait_seconds": 0.0,
            "max_bytes": max_bytes,
        }),
    )
    .await?;

    let deadline = Instant::now() + Duration::from_secs_f64(command.duration_seconds);
    let mut latest = terminal_capture(terminal_tool, session_key, max_bytes).await?;
    loop {
        if latest.terminal_state.contains(&marker) || latest.observation.contains(&marker) {
            latest.terminal_state = strip_marker(&latest.terminal_state, &marker);
            latest.observation = strip_marker(&latest.observation, &marker);
            return Ok(latest);
        }
        if Instant::now() >= deadline {
            let recovered_terminal_state = match terminal_interrupt(terminal_tool, session_key, max_bytes).await {
                Ok(recovered) => strip_marker(&recovered.terminal_state, &marker),
                Err(error) => {
                    tracing::warn!(%error, session_key, "failed to interrupt timed out terminal command");
                    strip_marker(&latest.terminal_state, &marker)
                }
            };
            return Ok(TerminalObservation {
                timed_out: true,
                terminal_state: recovered_terminal_state.clone(),
                observation: render_template(
                    TERMINUS_TIMEOUT_TEMPLATE,
                    &[
                        ("command", command.keystrokes.as_str()),
                        ("timeout_sec", &format!("{:.1}", command.duration_seconds)),
                        ("terminal_state", recovered_terminal_state.as_str()),
                    ],
                ),
            });
        }
        sleep(POLL_INTERVAL).await;
        latest = terminal_capture(terminal_tool, session_key, max_bytes).await?;
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

async fn terminal_interrupt(
    terminal_tool: &dyn Tool,
    session_key: &str,
    max_bytes: usize,
) -> Result<TerminalObservation, BrainError> {
    invoke_terminal_tool(
        terminal_tool,
        serde_json::json!({
            "session_id": session_key,
            "action": "send_keys",
            "keys": ["C-c"],
            "min_wait_seconds": 0.1,
            "max_bytes": max_bytes,
        }),
    )
    .await
}

async fn invoke_terminal_tool(
    terminal_tool: &dyn Tool,
    args: serde_json::Value,
) -> Result<TerminalObservation, BrainError> {
    let raw = with_block_timeout(terminal_tool.execute(args), "terminal_session").await?;
    serde_json::from_str(&raw).map_err(BrainError::from)
}

async fn with_block_timeout<T>(
    fut: impl std::future::Future<Output = Result<T, BrainError>>,
    label: &str,
) -> Result<T, BrainError> {
    timeout(BLOCK_TIMEOUT, fut)
        .await
        .map_err(|_| BrainError::Inference(format!("{label} timed out")))?
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

fn strip_marker(value: &str, marker: &str) -> String {
    value
        .lines()
        .filter(|line| !line.contains(marker))
        .collect::<Vec<_>>()
        .join("\n")
}

fn normalize_command_keystrokes(keystrokes: &str) -> String {
    let decoded = keystrokes
        .replace("\\r\\n", "\n")
        .replace("\\n", "\n")
        .replace("\\r", "\r")
        .replace("\\t", "\t");

    if parse_logical_keys(&decoded).is_some() {
        return decoded.trim().to_owned();
    }

    let normalized = decoded.trim_end_matches('\n');
    if normalized.is_empty() {
        "\n".to_owned()
    } else {
        format!("{normalized}\n")
    }
}

fn shell_single_quote(value: &str) -> String {
    value.replace('\'', "'\"'\"'")
}

fn infer_image_mime(path: &str) -> Option<&'static str> {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".png") {
        Some("image/png")
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        Some("image/jpeg")
    } else if lower.ends_with(".gif") {
        Some("image/gif")
    } else if lower.ends_with(".webp") {
        Some("image/webp")
    } else {
        None
    }
}

fn render_template(template: &str, values: &[(&str, &str)]) -> String {
    values.iter().fold(template.to_owned(), |acc, (key, value)| {
        acc.replace(&format!("{{{key}}}"), value)
    })
}

fn effective_max_iterations(config: &AgentConfig) -> u32 {
    if config.max_iterations == AgentConfig::default().max_iterations {
        DEFAULT_KIRA_MAX_ITERATIONS
    } else {
        config.max_iterations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_execute_commands_tool_call() {
        let parsed = parse_tool_calls(&[ToolCall {
            id: "call-1".into(),
            name: "execute_commands".into(),
            arguments: serde_json::json!({
                "analysis": "inspect",
                "plan": "edit",
                "commands": [
                    { "keystrokes": "ls\n", "duration": 1.5 }
                ]
            }),
        }]);

        assert_eq!(parsed.execute_call_id.as_deref(), Some("call-1"));
        assert_eq!(parsed.analysis, "inspect");
        assert_eq!(parsed.plan, "edit");
        assert_eq!(parsed.commands.len(), 1);
        assert_eq!(parsed.commands[0].keystrokes, "ls\n");
    }

    #[test]
    fn parse_image_read_tool_call() {
        let parsed = parse_tool_calls(&[ToolCall {
            id: "call-2".into(),
            name: "image_read".into(),
            arguments: serde_json::json!({
                "file_path": "/tmp/test.png",
                "image_read_instruction": "describe it"
            }),
        }]);

        assert_eq!(parsed.image_call_id.as_deref(), Some("call-2"));
        let image_read = parsed.image_read.expect("image read");
        assert_eq!(image_read.file_path, "/tmp/test.png");
        assert_eq!(image_read.instruction, "describe it");
    }

    #[test]
    fn completion_message_renders_checklist() {
        let rendered = render_template(
            COMPLETION_CONFIRMATION_TEMPLATE,
            &[("instruction", "solve it"), ("terminal_state", "$ pwd")],
        );
        assert!(rendered.contains("Original task:\nsolve it"));
        assert!(rendered.contains("Current terminal state:\n$ pwd"));
        assert!(rendered.contains("Checklist"));
    }

    #[test]
    fn normalize_command_keystrokes_appends_newline_for_shell_commands() {
        assert_eq!(normalize_command_keystrokes("ls -la /tmp"), "ls -la /tmp\n");
        assert_eq!(normalize_command_keystrokes("pwd\n"), "pwd\n");
    }

    #[test]
    fn normalize_command_keystrokes_decodes_literal_newline_escape_sequences() {
        assert_eq!(
            normalize_command_keystrokes("echo \"Hello\" > hello.txt\\n"),
            "echo \"Hello\" > hello.txt\n"
        );
    }

    #[test]
    fn normalize_command_keystrokes_preserves_logical_key_sequences() {
        assert_eq!(normalize_command_keystrokes("C-c Enter"), "C-c Enter");
    }
}
