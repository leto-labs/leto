use std::time::{SystemTime, UNIX_EPOCH};

use crate::terminal::{
    TerminusCommand, TerminusPhase, TerminusState, TerminusVariant, build_transcript,
    completion_confirmation, find_owned_pty, initial_prompt, render_capture_observation,
    split_transcript_state, timeout_observation,
};
use agent_runtime::{
    LoopContext, LoopDecision, LoopStrategy, Message, OpenPtyRequest, PtyCaptureMode,
    PtyCaptureRequest, PtyCaptureResult, PtyExecRequest, RuntimeError, RuntimeOperationResult,
    SubcallRequest,
};
use futures::future::BoxFuture;

/// Harbor-style text-protocol loop strategy backed by the runtime PTY surface.
pub struct Terminus2Loop;

impl LoopStrategy for Terminus2Loop {
    fn name(&self) -> &'static str {
        "terminus2"
    }

    fn decide<'a>(&'a self, ctx: LoopContext) -> BoxFuture<'a, Result<LoopDecision, RuntimeError>> {
        Box::pin(async move { decide_terminus2(ctx) })
    }
}

fn decide_terminus2(ctx: LoopContext) -> Result<LoopDecision, RuntimeError> {
    let state = ctx.state();
    if !state.active_turn {
        return Ok(LoopDecision::WaitForInput);
    }

    let (leading, loop_state, conversation) =
        split_transcript_state(&state.transcript, TerminusVariant::Terminus2);

    let Some(loop_state) = loop_state else {
        let Some(pty) = find_owned_pty(state.ptys.values().cloned(), TerminusVariant::Terminus2)
        else {
            let request = OpenPtyRequest {
                label: Some(format!(
                    "{}:{}",
                    TerminusVariant::Terminus2.label_prefix(),
                    state.session_id
                )),
                ..OpenPtyRequest::default()
            };
            return Ok(LoopDecision::OpenPty { request });
        };

        let Some(capture) = latest_capture(state, pty.pty_id) else {
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
            .find(|message| message.role == agent_runtime::MessageRole::User)
            .map(|message| message.plain_text_lossy())
            .unwrap_or_default();
        let prompt = initial_prompt(
            TerminusVariant::Terminus2,
            &original_instruction,
            &capture.snapshot.visible_screen,
        );
        let loop_state = TerminusState {
            variant: TerminusVariant::Terminus2,
            original_instruction,
            pty_id: pty.pty_id,
            pending_completion: false,
            phase: TerminusPhase::NeedProvider { sequence: 1 },
        };
        return Ok(LoopDecision::RewriteTranscript {
            rewrite: agent_runtime::TranscriptRewrite {
                purpose: "terminus2_initialize".into(),
                messages: build_transcript(&leading, &loop_state, vec![Message::user_text(prompt)]),
            },
        });
    };

    match &loop_state.phase {
        TerminusPhase::NeedProvider { sequence } => {
            let purpose = main_purpose(*sequence);
            if state
                .last_subcall
                .as_ref()
                .is_none_or(|result| result.purpose != purpose)
            {
                return Ok(LoopDecision::RunSubcall {
                    request: SubcallRequest {
                        purpose,
                        messages: provider_messages(&leading, &conversation),
                        tools: Vec::new(),
                        model_override: None,
                    },
                });
            }

            let result = state.last_subcall.as_ref().expect("checked above");
            if let Some(error) = result.error.as_deref() {
                if state
                    .context_pressure
                    .as_ref()
                    .is_some_and(|pressure| pressure.should_compact)
                {
                    return Ok(LoopDecision::CompactContext);
                }
                return Err(RuntimeError::Internal(format!(
                    "terminus2 provider subcall failed: {error}"
                )));
            }

            let assistant = result
                .message
                .clone()
                .unwrap_or_else(|| Message::assistant_text(""));
            let parsed = parse_json_response(&assistant.plain_text_lossy());
            let mut next_conversation = conversation.clone();
            next_conversation.push(assistant);

            let next_state = if let Some(error) = parsed.error {
                next_conversation.push(Message::user_text(repair_prompt(
                    &error,
                    parsed.warning.as_deref(),
                )));
                TerminusState {
                    pending_completion: loop_state.pending_completion,
                    phase: TerminusPhase::NeedProvider {
                        sequence: sequence + 1,
                    },
                    ..loop_state.clone()
                }
            } else if parsed.commands.is_empty() {
                TerminusState {
                    pending_completion: loop_state.pending_completion,
                    phase: TerminusPhase::PendingObservation {
                        sequence: sequence + 1,
                        task_complete: parsed.is_task_complete,
                        warning: parsed.warning,
                    },
                    ..loop_state.clone()
                }
            } else {
                TerminusState {
                    pending_completion: loop_state.pending_completion,
                    phase: TerminusPhase::PendingCommandExecution {
                        sequence: sequence + 1,
                        commands: parsed.commands,
                        task_complete: parsed.is_task_complete,
                        warning: parsed.warning,
                    },
                    ..loop_state.clone()
                }
            };

            Ok(LoopDecision::RewriteTranscript {
                rewrite: agent_runtime::TranscriptRewrite {
                    purpose: format!("terminus2_apply_subcall_{}", sequence),
                    messages: build_transcript(&leading, &next_state, next_conversation),
                },
            })
        }
        TerminusPhase::PendingCommandExecution {
            sequence,
            commands,
            task_complete,
            warning,
        } => {
            let normalized_steps = commands
                .iter()
                .map(|command| crate::terminal::normalize_command_keystrokes(&command.keystrokes))
                .collect::<Vec<_>>();
            if latest_exec(state, loop_state.pty_id)
                .is_none_or(|result| result.steps != normalized_steps)
            {
                let wait_ms = commands
                    .iter()
                    .fold(0u64, |total, command| {
                        total.saturating_add(command.duration_ms)
                    })
                    .clamp(0, 60_000);
                return Ok(LoopDecision::ExecutePtyBatch {
                    request: PtyExecRequest {
                        pty_id: loop_state.pty_id,
                        steps: normalized_steps,
                        wait_ms: Some(wait_ms),
                        background: false,
                    },
                });
            }

            let exec = latest_exec(state, loop_state.pty_id).expect("checked above");
            let observation = observation_from_snapshot(
                exec.snapshot.clone(),
                warning.as_deref(),
                false,
                commands
                    .last()
                    .map(|command| command.duration_ms)
                    .unwrap_or(0),
            );
            transition_from_observation(
                &leading,
                &conversation,
                &loop_state,
                *sequence,
                *task_complete,
                observation,
            )
        }
        TerminusPhase::PendingObservation {
            sequence,
            task_complete,
            warning,
        } => {
            if !matches!(
                state.recent_operations.back(),
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

            let capture = latest_capture(state, loop_state.pty_id).expect("checked above");
            let observation =
                observation_from_snapshot(capture.snapshot.clone(), warning.as_deref(), false, 0);
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
            if state
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
            "terminus2 entered unsupported phase: {other:?}"
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
            purpose: format!("terminus2_apply_observation_{next_sequence}"),
            messages: build_transcript(leading, &next_state, next_conversation),
        },
    })
}

fn observation_from_snapshot(
    snapshot: agent_runtime::PtySnapshot,
    warning: Option<&str>,
    timed_out: bool,
    duration_ms: u64,
) -> String {
    let capture = PtyCaptureResult {
        pty_id: snapshot.pty_id,
        snapshot,
    };
    let mut observation = render_capture_observation(&capture);
    if timed_out {
        observation = timeout_observation(
            "command batch",
            duration_ms as f64 / 1000.0,
            &capture.snapshot.visible_screen,
        );
    }
    if let Some(warning) = warning {
        observation =
            format!("Previous response had warnings:\nWARNINGS: {warning}\n\n{observation}");
    }
    observation
}

fn provider_messages(leading: &[Message], conversation: &[Message]) -> Vec<Message> {
    let mut messages = leading.to_vec();
    messages.extend(conversation.iter().cloned());
    messages
}

fn latest_capture(
    state: &agent_runtime::SessionState,
    pty_id: agent_runtime::PtyId,
) -> Option<agent_runtime::PtyCaptureResult> {
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

fn main_purpose(sequence: u64) -> String {
    format!("terminus2_main_{sequence}")
}

#[derive(Debug, Clone)]
struct ParseResult {
    commands: Vec<TerminusCommand>,
    is_task_complete: bool,
    error: Option<String>,
    warning: Option<String>,
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
            corrected.warning = combine_warning_parts(
                Some(format!(
                    "AUTO-CORRECTED: {warning} - please fix this in future responses"
                )),
                corrected.warning,
            );
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
            };
        }
    };

    let Some(obj) = parsed.as_object() else {
        return ParseResult {
            commands: Vec::new(),
            is_task_complete: false,
            error: Some("Response must be a JSON object".into()),
            warning: combine_warnings(warnings),
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
        };
    }

    let Some(commands_array) = obj["commands"].as_array() else {
        return ParseResult {
            commands: Vec::new(),
            is_task_complete: false,
            error: Some("Field 'commands' must be an array".into()),
            warning: combine_warnings(warnings),
        };
    };

    check_field_order(&json_content, &mut warnings);
    let is_task_complete = obj
        .get("task_complete")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);

    let mut commands = Vec::with_capacity(commands_array.len());
    for (index, value) in commands_array.iter().enumerate() {
        let Some(command) = value.as_object() else {
            return ParseResult {
                commands: Vec::new(),
                is_task_complete: false,
                error: Some(format!("Command {} must be an object", index + 1)),
                warning: combine_warnings(warnings),
            };
        };
        let Some(keystrokes) = command.get("keystrokes").and_then(|value| value.as_str()) else {
            return ParseResult {
                commands: Vec::new(),
                is_task_complete: false,
                error: Some(format!(
                    "Command {} missing required 'keystrokes' field",
                    index + 1
                )),
                warning: combine_warnings(warnings),
            };
        };
        let duration_ms = (command
            .get("duration")
            .and_then(|value| value.as_f64())
            .unwrap_or(1.0)
            .clamp(0.0, 60.0)
            * 1000.0) as u64;
        commands.push(TerminusCommand {
            keystrokes: keystrokes.to_owned(),
            duration_ms,
        });
    }

    ParseResult {
        commands,
        is_task_complete,
        error: None,
        warning: combine_warnings(warnings),
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

#[allow(dead_code)]
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
    fn parse_json_response_extracts_commands() {
        let parsed = parse_json_response(
            r#"{"analysis":"inspect","plan":"run pwd","commands":[{"keystrokes":"pwd\n","duration":0.1}]}"#,
        );
        assert!(parsed.error.is_none());
        assert_eq!(parsed.commands.len(), 1);
        assert_eq!(parsed.commands[0].keystrokes, "pwd\n");
        assert_eq!(parsed.commands[0].duration_ms, 100);
    }

    #[test]
    fn parse_json_response_reports_missing_fields() {
        let parsed = parse_json_response(r#"{"analysis":"inspect","commands":[]}"#);
        assert!(parsed.error.is_some());
        assert!(
            parsed
                .error
                .as_deref()
                .unwrap_or_default()
                .contains("Missing required fields")
        );
    }
}
