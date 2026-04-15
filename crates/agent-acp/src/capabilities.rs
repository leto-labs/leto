//! ACP capabilities helper functions.

use agent_client_protocol as acp;

/// Configuration option IDs used by ACP.
pub const CONFIG_MODEL: &str = "model";
pub const CONFIG_THOUGHT_LEVEL: &str = "thought_level";

/// Built-in ACP slash-command names supported by the real backend.
pub const COMMAND_PLAN: &str = "plan";
pub const COMMAND_PTY: &str = "pty";
pub const COMMAND_WORKTREE: &str = "worktree";
pub const COMMAND_SUBAGENTS: &str = "subagents";

/// Builds the initialize response with agent capabilities.
pub fn initialize_response(protocol_version: acp::ProtocolVersion) -> acp::InitializeResponse {
    let mut session_capabilities =
        acp::SessionCapabilities::new().list(acp::SessionListCapabilities::new());
    #[cfg(feature = "unstable_session_fork")]
    {
        session_capabilities = session_capabilities.fork(acp::SessionForkCapabilities::new());
    }
    #[cfg(feature = "unstable_session_resume")]
    {
        session_capabilities = session_capabilities.resume(acp::SessionResumeCapabilities::new());
    }
    #[cfg(feature = "unstable_session_close")]
    {
        session_capabilities = session_capabilities.close(acp::SessionCloseCapabilities::new());
    }

    acp::InitializeResponse::new(protocol_version)
        .agent_info(
            acp::Implementation::new("agent-acp", env!("CARGO_PKG_VERSION")).title("Agent ACP"),
        )
        .agent_capabilities(
            acp::AgentCapabilities::new()
                .load_session(true)
                .session_capabilities(session_capabilities),
        )
}

pub fn session_mode_state(
    current_mode_id: impl Into<String>,
    available_modes: &[String],
) -> Option<acp::SessionModeState> {
    if available_modes.is_empty() {
        return None;
    }

    let current_mode_id = current_mode_id.into();
    let mut modes: Vec<_> = available_modes
        .iter()
        .map(|mode| {
            acp::SessionMode::new(mode.clone(), title_case_mode_label(mode))
                .description(format!("Use the `{mode}` loop strategy for future turns."))
        })
        .collect();

    if !modes
        .iter()
        .any(|mode| mode.id.0.as_ref() == current_mode_id)
    {
        modes.push(
            acp::SessionMode::new(
                current_mode_id.clone(),
                title_case_mode_label(&current_mode_id),
            )
            .description(format!(
                "Use the `{current_mode_id}` loop strategy for future turns."
            )),
        );
    }

    Some(acp::SessionModeState::new(current_mode_id, modes))
}

pub fn current_mode_update(current_mode_id: impl Into<String>) -> acp::SessionUpdate {
    acp::SessionUpdate::CurrentModeUpdate(acp::CurrentModeUpdate::new(current_mode_id.into()))
}

pub fn available_commands_update() -> acp::SessionUpdate {
    acp::SessionUpdate::AvailableCommandsUpdate(acp::AvailableCommandsUpdate::new(vec![
        acp::AvailableCommand::new(
            COMMAND_PLAN,
            "Prefix a prompt with `/plan` to ask the agent to outline a concise execution plan before acting.",
        ),
        acp::AvailableCommand::new(
            COMMAND_PTY,
            "Prefix a prompt with `/pty` to steer the agent toward PTY-backed terminal tools such as `open_pty` and `execute_pty_command`.",
        ),
        acp::AvailableCommand::new(
            COMMAND_WORKTREE,
            "Prefix a prompt with `/worktree` to steer the agent toward git worktree tools such as `create_worktree`, `bind_worktree`, and `remove_worktree`.",
        ),
        acp::AvailableCommand::new(
            COMMAND_SUBAGENTS,
            "Prefix a prompt with `/subagents` to steer the agent toward child-agent and delegation tools such as `spawn_agent`.",
        ),
    ]))
}

fn title_case_mode_label(mode: &str) -> String {
    mode.split(['-', '_'])
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            let mut chars = segment.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
