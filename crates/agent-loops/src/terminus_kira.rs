use std::time::{Duration, SystemTime, UNIX_EPOCH};

use agent_runtime::{
    ContentBlock, LoopContext, LoopDecision, LoopStrategy, Message, MessageRole, OpenPtyRequest,
    PtyCaptureMode, PtyCaptureRequest, PtyCaptureResult, PtyExecRequest, RuntimeError,
    RuntimeOperationResult, SubcallRequest, ToolDefinition,
};
use futures::future::BoxFuture;
use serde_json::json;

use crate::terminal::{
    TerminusPhase, TerminusState, TerminusVariant, build_transcript, completion_confirmation,
    find_owned_pty, initial_prompt, normalize_command_keystrokes, render_capture_observation,
    split_transcript_state, strip_marker, timeout_observation,
};

const POLL_INTERVAL: Duration = Duration::from_millis(250);
const IMAGE_START_MARKER: &str = "__IMGSTART__";
const IMAGE_END_MARKER: &str = "__IMGEND__";

/// Native semantic-tool terminus loop backed by runtime PTY orchestration.
pub struct TerminusKiraLoop;

impl LoopStrategy for TerminusKiraLoop {
    fn name(&self) -> &'static str {
        "terminus_kira"
    }

    fn decide<'a>(&'a self, ctx: LoopContext) -> BoxFuture<'a, Result<LoopDecision, RuntimeError>> {
        Box::pin(async move { decide_terminus_kira(ctx).await })
    }
}

async fn decide_terminus_kira(ctx: LoopContext) -> Result<LoopDecision, RuntimeError> {
    let runtime_state = ctx.state();
    if !runtime_state.active_turn {
        return Ok(LoopDecision::WaitForInput);
    }

    let (leading, loop_state, conversation) =
        split_transcript_state(&runtime_state.transcript, TerminusVariant::TerminusKira);

    let Some(loop_state) = loop_state else {
        let Some(pty) = find_owned_pty(
            runtime_state.ptys.values().cloned(),
            TerminusVariant::TerminusKira,
        ) else {
            let mut request = OpenPtyRequest::default();
            request.label = Some(format!(
                "{}:{}",
                TerminusVariant::TerminusKira.label_prefix(),
                runtime_state.session_id
            ));
            return Ok(LoopDecision::OpenPty { request });
        };

        let Some(capture) = latest_capture(runtime_state, pty.pty_id) else {
            return Ok(LoopDecision::CapturePty {
                request: PtyCaptureRequest {
                    pty_id: pty.pty_id,
                    mode: PtyCaptureMode::VisibleScreen,
                    since_cursor: None,
                },
            });
        };

        let original_instruction = conversation
            .iter()
            .rev()
            .find(|message| message.role == MessageRole::User)
            .map(|message| message.plain_text_lossy())
            .unwrap_or_default();
        let prompt = initial_prompt(
            TerminusVariant::TerminusKira,
            &original_instruction,
            &capture.snapshot.visible_screen,
        );
        let loop_state = TerminusState {
            variant: TerminusVariant::TerminusKira,
            original_instruction,
            pty_id: pty.pty_id,
            pending_completion: false,
            phase: TerminusPhase::NeedProvider { sequence: 1 },
        };
        return Ok(LoopDecision::RewriteTranscript {
            rewrite: agent_runtime::TranscriptRewrite {
                purpose: "terminus_kira_initialize".into(),
                messages: build_transcript(&leading, &loop_state, vec![Message::user_text(prompt)]),
            },
        });
    };

    match &loop_state.phase {
        TerminusPhase::NeedProvider { sequence } => {
            let purpose = main_purpose(*sequence);
            if runtime_state
                .last_subcall
                .as_ref()
                .is_none_or(|result| result.purpose != purpose)
            {
                return Ok(LoopDecision::RunSubcall {
                    request: SubcallRequest {
                        purpose,
                        messages: provider_messages(&leading, &conversation),
                        tools: kira_tool_defs(),
                        model_override: None,
                    },
                });
            }

            let result = runtime_state.last_subcall.as_ref().expect("checked above");
            if let Some(error) = result.error.as_deref() {
                if runtime_state
                    .context_pressure
                    .as_ref()
                    .is_some_and(|pressure| pressure.should_compact)
                {
                    return Ok(LoopDecision::CompactContext);
                }
                return Err(RuntimeError::Internal(format!(
                    "terminus_kira provider subcall failed: {error}"
                )));
            }

            let assistant = result
                .message
                .clone()
                .unwrap_or_else(|| Message::assistant_text("Used native tool calling"));
            let parsed = parse_tool_turn(&assistant);
            let mut next_conversation = conversation.clone();
            next_conversation.push(assistant);

            let next_state = if let Some(image_read) = parsed.image_read {
                TerminusState {
                    phase: TerminusPhase::PendingImageRead {
                        sequence: sequence + 1,
                        file_path: image_read.file_path,
                        instruction: image_read.instruction,
                    },
                    ..loop_state.clone()
                }
            } else if !parsed.commands.is_empty() {
                let commands = parsed
                    .commands
                    .iter()
                    .map(|command| normalize_command_keystrokes(command))
                    .collect::<Vec<_>>();
                let marker = format!("__CMDEND__{}__", sequence + 1);
                let command_text = commands.join("\n");
                let deadline_ms = now_millis().saturating_add(parsed.total_duration_ms.max(1));
                let started_cursor = runtime_state
                    .ptys
                    .get(&loop_state.pty_id)
                    .map(|pty| pty.output_cursor)
                    .unwrap_or(0);
                TerminusState {
                    phase: TerminusPhase::PollingCommand {
                        sequence: sequence + 1,
                        commands,
                        marker,
                        deadline_ms,
                        command_text,
                        started_cursor,
                        task_complete: parsed.is_task_complete,
                        warning: parsed.feedback,
                        interrupt_sent: false,
                    },
                    ..loop_state.clone()
                }
            } else if let Some(feedback) = parsed.feedback {
                next_conversation.push(Message::user_text(feedback));
                TerminusState {
                    phase: TerminusPhase::NeedProvider {
                        sequence: sequence + 1,
                    },
                    ..loop_state.clone()
                }
            } else {
                TerminusState {
                    phase: TerminusPhase::PendingObservation {
                        sequence: sequence + 1,
                        task_complete: parsed.is_task_complete,
                        warning: None,
                    },
                    ..loop_state.clone()
                }
            };

            Ok(LoopDecision::RewriteTranscript {
                rewrite: agent_runtime::TranscriptRewrite {
                    purpose: format!("terminus_kira_apply_subcall_{sequence}"),
                    messages: build_transcript(&leading, &next_state, next_conversation),
                },
            })
        }
        TerminusPhase::PollingCommand {
            sequence,
            commands,
            marker,
            deadline_ms,
            command_text,
            started_cursor,
            task_complete,
            warning,
            interrupt_sent,
        } => {
            if latest_exec(runtime_state, loop_state.pty_id)
                .is_none_or(|result| !result.steps.iter().any(|step| step.contains(marker)))
            {
                let mut steps = commands.clone();
                steps.push(format!("printf '\\n{marker}\\n'"));
                return Ok(LoopDecision::ExecutePtyBatch {
                    request: PtyExecRequest {
                        pty_id: loop_state.pty_id,
                        steps,
                        wait_ms: Some(0),
                        background: false,
                    },
                });
            }

            if let Some(observation) = poll_observation(
                latest_capture(runtime_state, loop_state.pty_id),
                marker,
                warning.as_deref(),
                false,
                command_text,
            ) {
                return transition_from_observation(
                    &leading,
                    &conversation,
                    &loop_state,
                    *sequence,
                    *task_complete,
                    observation,
                );
            }

            if now_millis() >= *deadline_ms {
                if !interrupt_sent {
                    let next_state = TerminusState {
                        phase: TerminusPhase::PollingCommand {
                            sequence: *sequence,
                            commands: commands.clone(),
                            marker: marker.clone(),
                            deadline_ms: *deadline_ms,
                            command_text: command_text.clone(),
                            started_cursor: *started_cursor,
                            task_complete: *task_complete,
                            warning: warning.clone(),
                            interrupt_sent: true,
                        },
                        ..loop_state.clone()
                    };
                    return Ok(LoopDecision::RewriteTranscript {
                        rewrite: agent_runtime::TranscriptRewrite {
                            purpose: format!("terminus_kira_mark_interrupt_{sequence}"),
                            messages: build_transcript(&leading, &next_state, conversation.clone()),
                        },
                    });
                }

                if !matches!(
                    runtime_state.recent_operations.back(),
                    Some(RuntimeOperationResult::PtyCapture { result }) if result.pty_id == loop_state.pty_id
                ) {
                    return Ok(LoopDecision::CapturePty {
                        request: PtyCaptureRequest {
                            pty_id: loop_state.pty_id,
                            mode: PtyCaptureMode::Incremental,
                            since_cursor: Some(*started_cursor),
                        },
                    });
                }

                let capture = latest_capture(runtime_state, loop_state.pty_id)
                    .ok_or_else(|| RuntimeError::Internal("missing PTY capture".into()))?;
                let terminal_state = strip_marker(&capture.snapshot.visible_screen, marker);
                let mut observation = timeout_observation(
                    command_text,
                    (*deadline_ms).saturating_sub(now_millis()) as f64 / 1000.0,
                    &terminal_state,
                );
                if let Some(warning) = warning {
                    observation = format!(
                        "Previous response had warnings:\nWARNINGS: {warning}\n\n{observation}"
                    );
                }
                return transition_from_observation(
                    &leading,
                    &conversation,
                    &loop_state,
                    *sequence,
                    *task_complete,
                    observation,
                );
            }

            if *interrupt_sent {
                return Ok(LoopDecision::InterruptPty {
                    pty_id: loop_state.pty_id,
                });
            }

            tokio::time::sleep(POLL_INTERVAL).await;
            Ok(LoopDecision::CapturePty {
                request: PtyCaptureRequest {
                    pty_id: loop_state.pty_id,
                    mode: PtyCaptureMode::Incremental,
                    since_cursor: Some(*started_cursor),
                },
            })
        }
        TerminusPhase::PendingImageRead {
            sequence,
            file_path,
            instruction,
        } => {
            let purpose = image_purpose(*sequence);
            if runtime_state
                .last_subcall
                .as_ref()
                .is_some_and(|result| result.purpose == purpose)
            {
                let result = runtime_state.last_subcall.as_ref().expect("checked above");
                if let Some(error) = result.error.as_deref() {
                    return Err(RuntimeError::Internal(format!(
                        "terminus_kira image_read subcall failed: {error}"
                    )));
                }
                let content = result
                    .message
                    .as_ref()
                    .map(|message| message.plain_text_lossy())
                    .unwrap_or_default();
                let mut next_conversation = conversation.clone();
                next_conversation.push(Message::user_text(format!(
                    "File Read Result for '{}':\n{}",
                    file_path, content
                )));
                let next_state = TerminusState {
                    phase: TerminusPhase::NeedProvider {
                        sequence: sequence + 1,
                    },
                    ..loop_state.clone()
                };
                return Ok(LoopDecision::RewriteTranscript {
                    rewrite: agent_runtime::TranscriptRewrite {
                        purpose: format!("terminus_kira_apply_image_read_{sequence}"),
                        messages: build_transcript(&leading, &next_state, next_conversation),
                    },
                });
            }

            if let Some(data_url) =
                extract_image_data_url(latest_exec(runtime_state, loop_state.pty_id), file_path)
            {
                return Ok(LoopDecision::RunSubcall {
                    request: SubcallRequest {
                        purpose,
                        messages: vec![Message::new(
                            MessageRole::User,
                            vec![
                                ContentBlock::text(instruction.clone()),
                                ContentBlock::image_url(data_url),
                            ],
                        )],
                        tools: Vec::new(),
                        model_override: None,
                    },
                });
            }

            return Ok(LoopDecision::ExecutePtyBatch {
                request: PtyExecRequest {
                    pty_id: loop_state.pty_id,
                    steps: vec![format!(
                        "printf '{IMAGE_START_MARKER}'; base64 '{}' | tr -d '\\n'; printf '{IMAGE_END_MARKER}\\n'",
                        shell_single_quote(file_path)
                    )],
                    wait_ms: Some(250),
                    background: false,
                },
            });
        }
        TerminusPhase::PendingObservation {
            sequence,
            task_complete,
            warning,
        } => {
            if !matches!(
                runtime_state.recent_operations.back(),
                Some(RuntimeOperationResult::PtyCapture { result }) if result.pty_id == loop_state.pty_id
            ) {
                return Ok(LoopDecision::CapturePty {
                    request: PtyCaptureRequest {
                        pty_id: loop_state.pty_id,
                        mode: PtyCaptureMode::VisibleScreen,
                        since_cursor: None,
                    },
                });
            }

            let capture = latest_capture(runtime_state, loop_state.pty_id)
                .ok_or_else(|| RuntimeError::Internal("missing PTY capture".into()))?;
            let mut observation = render_capture_observation(&capture);
            if let Some(warning) = warning {
                observation = format!(
                    "Previous response had warnings:\nWARNINGS: {warning}\n\n{observation}"
                );
            }
            transition_from_observation(
                &leading,
                &conversation,
                &loop_state,
                *sequence,
                *task_complete,
                observation,
            )
        }
        TerminusPhase::ClosingPty => {
            if runtime_state
                .ptys
                .get(&loop_state.pty_id)
                .is_none_or(|pty| pty.status == agent_runtime::PtyStatus::Closed)
            {
                return Ok(LoopDecision::FinishTurn);
            }
            Ok(LoopDecision::ClosePty {
                pty_id: loop_state.pty_id,
            })
        }
        TerminusPhase::Completed => Ok(LoopDecision::FinishTurn),
        other => Err(RuntimeError::Internal(format!(
            "terminus_kira entered unsupported phase: {other:?}"
        ))),
    }
}

fn transition_from_observation(
    leading: &[Message],
    conversation: &[Message],
    loop_state: &TerminusState,
    next_sequence: u64,
    task_complete: bool,
    observation: String,
) -> Result<LoopDecision, RuntimeError> {
    let mut next_conversation = conversation.to_vec();
    let next_state = if task_complete {
        if loop_state.pending_completion {
            TerminusState {
                phase: TerminusPhase::ClosingPty,
                ..loop_state.clone()
            }
        } else {
            let confirmation =
                completion_confirmation(&loop_state.original_instruction, &observation);
            next_conversation.push(Message::user_text(confirmation));
            TerminusState {
                pending_completion: true,
                phase: TerminusPhase::NeedProvider {
                    sequence: next_sequence,
                },
                ..loop_state.clone()
            }
        }
    } else {
        next_conversation.push(Message::user_text(observation));
        TerminusState {
            pending_completion: false,
            phase: TerminusPhase::NeedProvider {
                sequence: next_sequence,
            },
            ..loop_state.clone()
        }
    };

    Ok(LoopDecision::RewriteTranscript {
        rewrite: agent_runtime::TranscriptRewrite {
            purpose: format!("terminus_kira_apply_observation_{next_sequence}"),
            messages: build_transcript(leading, &next_state, next_conversation),
        },
    })
}

fn poll_observation(
    capture: Option<PtyCaptureResult>,
    marker: &str,
    warning: Option<&str>,
    timed_out: bool,
    command_text: &str,
) -> Option<String> {
    let capture = capture?;
    let snapshot_text = capture
        .snapshot
        .incremental_output
        .clone()
        .unwrap_or_else(|| capture.snapshot.visible_screen.clone());
    if !snapshot_text.contains(marker) && !timed_out {
        return None;
    }

    let terminal_state = strip_marker(&capture.snapshot.visible_screen, marker);
    let mut observation = if timed_out {
        timeout_observation(command_text, 0.0, &terminal_state)
    } else {
        let cleaned_incremental = capture
            .snapshot
            .incremental_output
            .as_deref()
            .map(|value| strip_marker(value, marker))
            .filter(|value| !value.trim().is_empty());
        if let Some(incremental) = cleaned_incremental {
            format!(
                "Current terminal state:\n{}\n\nNew terminal output:\n{}",
                terminal_state.trim_end(),
                incremental.trim_end()
            )
        } else {
            format!("Current terminal state:\n{}", terminal_state.trim_end())
        }
    };
    if let Some(warning) = warning {
        observation =
            format!("Previous response had warnings:\nWARNINGS: {warning}\n\n{observation}");
    }
    Some(observation)
}

fn latest_capture(
    state: &agent_runtime::SessionState,
    pty_id: agent_runtime::PtyId,
) -> Option<PtyCaptureResult> {
    match state.recent_operations.back() {
        Some(RuntimeOperationResult::PtyCapture { result }) if result.pty_id == pty_id => {
            Some(result.clone())
        }
        Some(RuntimeOperationResult::PtyExecution { result }) if result.pty_id == pty_id => {
            Some(PtyCaptureResult {
                pty_id,
                snapshot: result.snapshot.clone(),
            })
        }
        _ => state
            .last_pty_capture
            .as_ref()
            .filter(|result| result.pty_id == pty_id)
            .cloned(),
    }
}

fn latest_exec(
    state: &agent_runtime::SessionState,
    pty_id: agent_runtime::PtyId,
) -> Option<agent_runtime::PtyExecResult> {
    match state.recent_operations.back() {
        Some(RuntimeOperationResult::PtyExecution { result }) if result.pty_id == pty_id => {
            Some(result.clone())
        }
        _ => None,
    }
}

fn provider_messages(leading: &[Message], conversation: &[Message]) -> Vec<Message> {
    let mut messages = leading.to_vec();
    messages.extend(conversation.iter().cloned());
    messages
}

fn main_purpose(sequence: u64) -> String {
    format!("terminus_kira_main_{sequence}")
}

fn image_purpose(sequence: u64) -> String {
    format!("terminus_kira_image_{sequence}")
}

fn kira_tool_defs() -> Vec<ToolDefinition> {
    // These are loop-private semantic tools. They describe the Kira protocol
    // for one provider subcall, not reusable application tools. The current
    // runtime models that by attaching tool definitions directly to the
    // subcall. A later refactor should align this with loop-aware tool support
    // so each loop can declare its supported tools through a first-class
    // runtime surface.
    vec![
        ToolDefinition::new(
            "execute_commands",
            "Run one or more shell command batches with analysis and plan.",
            json!({
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
        ),
        ToolDefinition::new(
            "task_complete",
            "Signal that the task is complete and ready for verification.",
            json!({ "type": "object", "properties": {} }),
        ),
        ToolDefinition::new(
            "image_read",
            "Analyze an image file by path using multimodal model input.",
            json!({
                "type": "object",
                "properties": {
                    "file_path": { "type": "string" },
                    "image_read_instruction": { "type": "string" }
                },
                "required": ["file_path", "image_read_instruction"]
            }),
        ),
    ]
}

#[derive(Debug, Clone)]
struct ParsedImageRead {
    file_path: String,
    instruction: String,
}

#[derive(Debug, Clone, Default)]
struct ParsedToolTurn {
    commands: Vec<String>,
    total_duration_ms: u64,
    is_task_complete: bool,
    feedback: Option<String>,
    image_read: Option<ParsedImageRead>,
}

fn parse_tool_turn(message: &Message) -> ParsedToolTurn {
    let mut parsed = ParsedToolTurn::default();
    let mut saw_tool_call = false;

    for block in &message.content {
        let ContentBlock::ToolCall { name, input, .. } = block else {
            continue;
        };
        saw_tool_call = true;
        match name.as_str() {
            "execute_commands" => {
                if let Some(commands) = input.get("commands").and_then(|value| value.as_array()) {
                    for command in commands {
                        let Some(keystrokes) =
                            command.get("keystrokes").and_then(|value| value.as_str())
                        else {
                            parsed.feedback = Some(
                                "WARNINGS: execute_commands requires a keystrokes field.".into(),
                            );
                            continue;
                        };
                        let duration_ms = (command
                            .get("duration")
                            .and_then(|value| value.as_f64())
                            .unwrap_or(1.0)
                            .clamp(0.0, 60.0)
                            * 1000.0) as u64;
                        parsed.total_duration_ms =
                            parsed.total_duration_ms.saturating_add(duration_ms.max(1));
                        parsed.commands.push(keystrokes.to_owned());
                    }
                } else {
                    parsed.feedback =
                        Some("WARNINGS: execute_commands requires a commands array.".into());
                }
            }
            "task_complete" => {
                parsed.is_task_complete = true;
            }
            "image_read" => {
                let file_path = input
                    .get("file_path")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default();
                let instruction = input
                    .get("image_read_instruction")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default();
                if file_path.is_empty() || instruction.is_empty() {
                    parsed.feedback = Some(
                        "WARNINGS: image_read requires both file_path and image_read_instruction."
                            .into(),
                    );
                } else {
                    parsed.image_read = Some(ParsedImageRead {
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

    if !saw_tool_call && message.plain_text_lossy().trim().is_empty() {
        parsed.feedback = Some(
            "WARNINGS: Your response contained no tool calls. Please use execute_commands, image_read, or task_complete."
                .into(),
        );
    }

    parsed
}

fn extract_image_data_url(
    exec: Option<agent_runtime::PtyExecResult>,
    file_path: &str,
) -> Option<String> {
    let exec = exec?;
    let text = exec.snapshot.incremental_output?;
    let start = text.find(IMAGE_START_MARKER)?;
    let end = text.find(IMAGE_END_MARKER)?;
    if end <= start {
        return None;
    }
    let payload = text[start + IMAGE_START_MARKER.len()..end].trim();
    let mime = infer_image_mime(file_path)?;
    Some(format!("data:{mime};base64,{payload}"))
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

fn shell_single_quote(value: &str) -> String {
    value.replace('\'', "'\"'\"'")
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tool_turn_extracts_commands_and_completion() {
        let message = Message::new(
            MessageRole::Assistant,
            vec![ContentBlock::tool_call(
                "call-1",
                "execute_commands",
                json!({
                    "analysis": "inspect",
                    "plan": "run pwd",
                    "commands": [{ "keystrokes": "pwd", "duration": 0.1 }]
                }),
            )],
        );
        let parsed = parse_tool_turn(&message);
        assert_eq!(parsed.commands, vec!["pwd".to_string()]);
        assert!(!parsed.is_task_complete);
    }

    #[test]
    fn parse_tool_turn_extracts_image_read() {
        let message = Message::new(
            MessageRole::Assistant,
            vec![ContentBlock::tool_call(
                "call-2",
                "image_read",
                json!({
                    "file_path": "/tmp/test.png",
                    "image_read_instruction": "describe"
                }),
            )],
        );
        let parsed = parse_tool_turn(&message);
        assert_eq!(
            parsed
                .image_read
                .as_ref()
                .map(|image| image.file_path.as_str()),
            Some("/tmp/test.png")
        );
    }

    #[test]
    fn extract_image_data_url_reads_marked_payload() {
        let exec = agent_runtime::PtyExecResult {
            pty_id: agent_runtime::PtyId::new(),
            steps: vec!["cmd".into()],
            backgrounded: false,
            completed: true,
            interrupted: false,
            shell_exited: false,
            exit_code: None,
            snapshot: agent_runtime::PtySnapshot {
                pty_id: agent_runtime::PtyId::new(),
                visible_screen: String::new(),
                incremental_output: Some(format!("{IMAGE_START_MARKER}Zm9v{IMAGE_END_MARKER}")),
                output_cursor: 0,
            },
        };
        let data_url = extract_image_data_url(Some(exec), "/tmp/test.png").unwrap();
        assert_eq!(data_url, "data:image/png;base64,Zm9v");
    }
}
