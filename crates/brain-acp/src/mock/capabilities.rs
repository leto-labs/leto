use agent_client_protocol::{
    self as acp, AvailableCommand, AvailableCommandsUpdate, CurrentModeUpdate,
};

use super::session_registry::MockSession;

pub(super) const AUTH_METHOD_ID: &str = "mock-browser-login";
pub(super) const MODE_ASK: &str = "ask";
pub(super) const MODE_ARCHITECT: &str = "architect";
pub(super) const MODE_CODE: &str = "code";
pub(super) const CONFIG_REASONING: &str = "reasoning_level";
pub(super) const CONFIG_APPROVAL: &str = "approval_preset";
pub(super) const CONFIG_LOOP: &str = "loop";
pub(super) const CATEGORY_LOOP: &str = "_loop";
pub(super) const REASONING_STANDARD: &str = "standard";
pub(super) const REASONING_DEEP: &str = "deep";
pub(super) const APPROVAL_DEFAULT: &str = "default";
pub(super) const APPROVAL_FULL: &str = "full-access";
pub(super) const LOOP_SIMPLE: &str = "simple";
pub(super) const LOOP_PLANNER: &str = "planner";
pub(super) const SESSION_PAGE_SIZE: usize = 2;
#[cfg(feature = "unstable_session_model")]
pub(super) const MODEL_FAST: &str = "brain-mock-fast";
#[cfg(feature = "unstable_session_model")]
pub(super) const MODEL_DEEP: &str = "brain-mock-deep";

pub(super) fn session_modes() -> Vec<acp::SessionMode> {
    vec![
        acp::SessionMode::new(MODE_ASK, "Ask")
            .description("Answer questions without pretending to modify project state."),
        acp::SessionMode::new(MODE_ARCHITECT, "Architect")
            .description("Plan the work and explain the intended implementation shape."),
        acp::SessionMode::new(MODE_CODE, "Code")
            .description("Act like an implementation-focused coding agent."),
    ]
}

pub(super) fn default_available_commands() -> Vec<AvailableCommand> {
    vec![
        AvailableCommand::new(
            "create-plan",
            "Trigger with `mock:create-plan` to emit a small mock execution plan.",
        ),
        AvailableCommand::new(
            "summarize-session",
            "Trigger with `mock:summarize-session` to summarize the current mock session state.",
        ),
        AvailableCommand::new(
            "think",
            "Trigger with `mock:think [topic]` to emit mock reasoning chunks and a synthetic thinking tool result.",
        ),
        AvailableCommand::new(
            "search",
            "Trigger with `mock:search [query]` to emit a synthetic search tool flow.",
        ),
        AvailableCommand::new(
            "fetch",
            "Trigger with `mock:fetch [resource]` to emit a synthetic fetch tool flow.",
        ),
        AvailableCommand::new(
            "edit-file",
            "Trigger with `mock:edit-file [/absolute/path]` to emit a synthetic patch-style edit flow.",
        ),
        AvailableCommand::new(
            "delete-file",
            "Trigger with `mock:delete-file [/absolute/path]` to emit a synthetic patch-style delete flow.",
        ),
        AvailableCommand::new(
            "move-file",
            "Trigger with `mock:move-file [/absolute/from /absolute/to]` to emit a synthetic move flow.",
        ),
        AvailableCommand::new(
            "read-file",
            "Trigger the mock ACP file-read request via `mock:read-file [/absolute/path]`.",
        ),
        AvailableCommand::new(
            "write-file",
            "Trigger the mock ACP file-write request via `mock:write-file [/absolute/path] [content]`.",
        ),
        AvailableCommand::new(
            "request-permission",
            "Trigger the mock ACP permission request flow via `mock:request-permission`.",
        ),
        AvailableCommand::new(
            "terminal",
            "Trigger the mock ACP terminal flow via `mock:terminal [command] [args...]`.",
        ),
        AvailableCommand::new(
            "terminal-kill",
            "Trigger the mock ACP terminal kill flow via `mock:terminal-kill [command] [args...]`.",
        ),
    ]
}

pub(super) fn available_commands_update() -> acp::SessionUpdate {
    acp::SessionUpdate::AvailableCommandsUpdate(AvailableCommandsUpdate::new(
        default_available_commands(),
    ))
}

pub(super) fn config_options(session: &MockSession) -> Vec<acp::SessionConfigOption> {
    vec![
        acp::SessionConfigOption::select(
            CONFIG_REASONING,
            "Reasoning Level",
            session.reasoning_level.clone(),
            vec![
                acp::SessionConfigSelectOption::new(REASONING_STANDARD, "Standard"),
                acp::SessionConfigSelectOption::new(REASONING_DEEP, "Deep"),
            ],
        )
        .description("Controls how thorough the mock agent claims to be.")
        .category(acp::SessionConfigOptionCategory::ThoughtLevel),
        acp::SessionConfigOption::select(
            CONFIG_APPROVAL,
            "Approval Preset",
            session.approval_preset.clone(),
            vec![
                acp::SessionConfigSelectOption::new(APPROVAL_DEFAULT, "Default"),
                acp::SessionConfigSelectOption::new(APPROVAL_FULL, "Full Access"),
            ],
        )
        .description("Mock permission posture used only for ACP UI validation."),
        acp::SessionConfigOption::select(
            CONFIG_LOOP,
            "Loop",
            session.loop_name.clone(),
            vec![
                acp::SessionConfigSelectOption::new(LOOP_SIMPLE, "Simple"),
                acp::SessionConfigSelectOption::new(LOOP_PLANNER, "Planner"),
            ],
        )
        .description("Mock loop selector used to validate ACP session loop UX.")
        .category(acp::SessionConfigOptionCategory::Other(CATEGORY_LOOP.to_owned())),
    ]
}

pub(super) fn session_mode_state(session: &MockSession) -> acp::SessionModeState {
    acp::SessionModeState::new(session.mode_id.clone(), session_modes())
}

#[cfg(feature = "unstable_session_model")]
pub(super) fn model_state(session: &MockSession) -> acp::SessionModelState {
    acp::SessionModelState::new(
        session.model_id.clone(),
        vec![
            acp::ModelInfo::new(MODEL_FAST, "Brain Mock Fast")
                .description("Fast mock model optimized for simple echo replies."),
            acp::ModelInfo::new(MODEL_DEEP, "Brain Mock Deep")
                .description("Slower mock model with more verbose canned output."),
        ],
    )
}

pub(super) fn info_update(session: &MockSession) -> acp::SessionInfoUpdate {
    acp::SessionInfoUpdate::new()
        .title(session.title.clone())
        .updated_at(session.updated_at.to_rfc3339())
}

pub(super) fn list_info(session_id: acp::SessionId, session: &MockSession) -> acp::SessionInfo {
    acp::SessionInfo::new(session_id, session.cwd.clone())
        .title(session.title.clone())
        .updated_at(session.updated_at.to_rfc3339())
}

pub(super) fn current_mode_update(session: &MockSession) -> acp::SessionUpdate {
    acp::SessionUpdate::CurrentModeUpdate(CurrentModeUpdate::new(session.mode_id.clone()))
}
