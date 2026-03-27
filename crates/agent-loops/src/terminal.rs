use agent_runtime::{Message, MessageRole, PtyCaptureResult, PtyId, PtySessionState};
use serde::{Deserialize, Serialize};

pub(crate) const TERMINUS2_PROMPT_TEMPLATE: &str =
    include_str!("templates/terminus-json-plain.txt");
pub(crate) const KIRA_PROMPT_TEMPLATE: &str = include_str!("templates/terminus-kira.txt");
pub(crate) const TERMINUS_TIMEOUT_TEMPLATE: &str = include_str!("templates/timeout.txt");

const TERMINUS_STATE_TAG: &str = "terminus_state";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TerminusVariant {
    Terminus2,
    TerminusKira,
}

impl TerminusVariant {
    pub(crate) fn label_prefix(self) -> &'static str {
        match self {
            Self::Terminus2 => "terminus2",
            Self::TerminusKira => "terminus_kira",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub(crate) enum TerminusPhase {
    NeedProvider {
        sequence: u64,
    },
    PendingCommandExecution {
        sequence: u64,
        commands: Vec<TerminusCommand>,
        task_complete: bool,
        warning: Option<String>,
    },
    PendingObservation {
        sequence: u64,
        task_complete: bool,
        warning: Option<String>,
    },
    PollingCommand {
        sequence: u64,
        commands: Vec<String>,
        marker: String,
        deadline_ms: u64,
        command_text: String,
        started_cursor: u64,
        task_complete: bool,
        warning: Option<String>,
        interrupt_sent: bool,
    },
    PendingImageRead {
        sequence: u64,
        file_path: String,
        instruction: String,
    },
    ClosingPty,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TerminusState {
    pub variant: TerminusVariant,
    pub original_instruction: String,
    pub pty_id: PtyId,
    pub pending_completion: bool,
    pub phase: TerminusPhase,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TerminusCommand {
    pub keystrokes: String,
    pub duration_ms: u64,
}

pub(crate) fn render_state_message(state: &TerminusState) -> Message {
    Message::developer_text(format!(
        "<{TERMINUS_STATE_TAG}>{}</{TERMINUS_STATE_TAG}>",
        serde_json::to_string(state).unwrap_or_else(|_| "{}".into())
    ))
}

pub(crate) fn split_transcript_state(
    transcript: &[Message],
    variant: TerminusVariant,
) -> (Vec<Message>, Option<TerminusState>, Vec<Message>) {
    let mut leading = Vec::new();
    let mut conversation = Vec::new();
    let mut state = None;
    let mut saw_non_instruction = false;

    for message in transcript {
        if state.is_none()
            && message.role == MessageRole::Developer
            && let Some(parsed) = parse_state_message(message)
            && parsed.variant == variant
        {
            state = Some(parsed);
            saw_non_instruction = true;
            continue;
        }

        if !saw_non_instruction
            && matches!(message.role, MessageRole::System | MessageRole::Developer)
        {
            leading.push(message.clone());
        } else {
            saw_non_instruction = true;
            conversation.push(message.clone());
        }
    }

    (leading, state, conversation)
}

pub(crate) fn parse_state_message(message: &Message) -> Option<TerminusState> {
    let text = message.plain_text_lossy();
    let start = text.find(&format!("<{TERMINUS_STATE_TAG}>"))?;
    let end = text.find(&format!("</{TERMINUS_STATE_TAG}>"))?;
    let json = &text[start + TERMINUS_STATE_TAG.len() + 2..end];
    serde_json::from_str(json).ok()
}

pub(crate) fn build_transcript(
    leading: &[Message],
    state: &TerminusState,
    conversation: Vec<Message>,
) -> Vec<Message> {
    let mut transcript = leading.to_vec();
    transcript.push(render_state_message(state));
    transcript.extend(conversation);
    transcript
}

pub(crate) fn find_owned_pty(
    mut ptys: impl Iterator<Item = PtySessionState>,
    variant: TerminusVariant,
) -> Option<PtySessionState> {
    let prefix = variant.label_prefix();
    ptys.find(|pty| {
        pty.label
            .as_deref()
            .is_some_and(|label| label.starts_with(prefix))
    })
}

pub(crate) fn initial_prompt(
    variant: TerminusVariant,
    instruction: &str,
    terminal_state: &str,
) -> String {
    render_template(
        match variant {
            TerminusVariant::Terminus2 => TERMINUS2_PROMPT_TEMPLATE,
            TerminusVariant::TerminusKira => KIRA_PROMPT_TEMPLATE,
        },
        &[
            ("instruction", instruction),
            ("terminal_state", terminal_state),
        ],
    )
}

pub(crate) fn completion_confirmation(instruction: &str, terminal_state: &str) -> String {
    render_template(
        r#"Original task:
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

After this point, solution grading will begin and no further edits will be possible. If everything looks good, call task_complete tool again."#,
        &[
            ("instruction", instruction),
            ("terminal_state", terminal_state),
        ],
    )
}

pub(crate) fn timeout_observation(command: &str, timeout_sec: f64, terminal_state: &str) -> String {
    render_template(
        TERMINUS_TIMEOUT_TEMPLATE,
        &[
            ("command", command),
            ("timeout_sec", &format!("{timeout_sec:.1}")),
            ("terminal_state", terminal_state),
        ],
    )
}

pub(crate) fn render_capture_observation(capture: &PtyCaptureResult) -> String {
    let screen = capture.snapshot.visible_screen.trim_end();
    if let Some(incremental) = capture.snapshot.incremental_output.as_deref() {
        let incremental = incremental.trim_end();
        if !incremental.is_empty() {
            return format!(
                "Current terminal state:\n{}\n\nNew terminal output:\n{}",
                screen, incremental
            );
        }
    }
    format!("Current terminal state:\n{screen}")
}

pub(crate) fn render_template(template: &str, replacements: &[(&str, &str)]) -> String {
    let mut rendered = template.to_owned();
    for (key, value) in replacements {
        rendered = rendered.replace(&format!("{{{key}}}"), value);
    }
    rendered.replace("{{", "{").replace("}}", "}")
}

pub(crate) fn parse_logical_keys(keystrokes: &str) -> Option<Vec<String>> {
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

pub(crate) fn normalize_command_keystrokes(keystrokes: &str) -> String {
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

pub(crate) fn strip_marker(value: &str, marker: &str) -> String {
    value
        .lines()
        .filter(|line| !line.contains(marker))
        .collect::<Vec<_>>()
        .join("\n")
}
