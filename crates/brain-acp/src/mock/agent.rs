use std::{
    cell::RefCell,
    collections::HashMap,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use agent_client_protocol::{
    self as acp, AvailableCommand, AvailableCommandsUpdate, Client as _, CurrentModeUpdate, Plan,
    PlanEntry, PlanEntryPriority, PlanEntryStatus,
};
use chrono::{DateTime, Utc};
use serde_json::{json, value::to_raw_value};
use tokio::sync::{mpsc, oneshot};

pub const SEEDED_SESSION_ID: &str = "mock-seeded-session";

const AUTH_METHOD_ID: &str = "mock-browser-login";
const MODE_ASK: &str = "ask";
const MODE_ARCHITECT: &str = "architect";
const MODE_CODE: &str = "code";
const CONFIG_REASONING: &str = "reasoning_level";
const CONFIG_APPROVAL: &str = "approval_preset";
const REASONING_STANDARD: &str = "standard";
const REASONING_DEEP: &str = "deep";
const APPROVAL_DEFAULT: &str = "default";
const APPROVAL_FULL: &str = "full-access";
const SESSION_PAGE_SIZE: usize = 2;
#[cfg(feature = "unstable_session_model")]
const MODEL_FAST: &str = "brain-mock-fast";
#[cfg(feature = "unstable_session_model")]
const MODEL_DEEP: &str = "brain-mock-deep";

pub(super) type NotificationEnvelope = (acp::SessionNotification, oneshot::Sender<()>);

#[derive(Clone)]
struct MockMessage {
    role: MessageRole,
    content: String,
}

impl MockMessage {
    fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
        }
    }

    fn agent(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Agent,
            content: content.into(),
        }
    }
}

#[derive(Clone, Copy)]
enum MessageRole {
    User,
    Agent,
}

#[derive(Clone)]
struct MockSession {
    cwd: PathBuf,
    title: String,
    history: Vec<MockMessage>,
    updated_at: DateTime<Utc>,
    mode_id: acp::SessionModeId,
    reasoning_level: acp::SessionConfigValueId,
    approval_preset: acp::SessionConfigValueId,
    #[cfg(feature = "unstable_session_model")]
    model_id: acp::ModelId,
    closed: bool,
}

impl MockSession {
    fn seeded() -> Self {
        Self {
            cwd: PathBuf::from("/mock/seeded"),
            title: "Seeded Mock Session".to_owned(),
            history: vec![
                MockMessage::user("What can you do?"),
                MockMessage::agent(
                    "I am a mock brain-acp agent used to validate the ACP transport and lifecycle.",
                ),
            ],
            updated_at: Utc::now(),
            mode_id: acp::SessionModeId::new(MODE_ASK),
            reasoning_level: acp::SessionConfigValueId::new(REASONING_STANDARD),
            approval_preset: acp::SessionConfigValueId::new(APPROVAL_DEFAULT),
            #[cfg(feature = "unstable_session_model")]
            model_id: acp::ModelId::new(MODEL_FAST),
            closed: false,
        }
    }

    fn restored(cwd: PathBuf, session_id: &acp::SessionId) -> Self {
        let mut session = Self::new(cwd);
        session.title = format!("Restored Mock Session: {}", session_id.0.as_ref());
        session.history.push(MockMessage::agent(
            "This mock session was reconstructed to satisfy an external ACP client load request.",
        ));
        session
    }
}

struct MockState {
    next_session_index: u64,
    next_tool_call_index: u64,
    sessions: HashMap<acp::SessionId, MockSession>,
    active_turns: HashMap<acp::SessionId, Arc<AtomicBool>>,
}

impl MockState {
    fn seeded() -> Self {
        let mut sessions = HashMap::new();
        sessions.insert(
            acp::SessionId::new(SEEDED_SESSION_ID),
            MockSession::seeded(),
        );

        Self {
            next_session_index: 1,
            next_tool_call_index: 1,
            sessions,
            active_turns: HashMap::new(),
        }
    }

    fn create_session(&mut self, cwd: &Path) -> acp::SessionId {
        let session_id = acp::SessionId::new(format!("mock-session-{}", self.next_session_index));
        self.next_session_index += 1;
        self.sessions
            .insert(session_id.clone(), MockSession::new(cwd.to_path_buf()));
        session_id
    }

    fn get_session(&self, session_id: &acp::SessionId) -> Result<&MockSession, acp::Error> {
        self.sessions
            .get(session_id)
            .filter(|session| !session.closed)
            .ok_or_else(acp::Error::invalid_params)
    }

    fn get_session_mut(
        &mut self,
        session_id: &acp::SessionId,
    ) -> Result<&mut MockSession, acp::Error> {
        self.sessions
            .get_mut(session_id)
            .filter(|session| !session.closed)
            .ok_or_else(acp::Error::invalid_params)
    }

    fn visible_sessions(&self) -> Vec<(acp::SessionId, MockSession)> {
        let mut sessions = self
            .sessions
            .iter()
            .filter(|(_, session)| !session.closed)
            .map(|(session_id, session)| (session_id.clone(), session.clone()))
            .collect::<Vec<_>>();
        sessions.sort_by(|left, right| {
            right
                .1
                .updated_at
                .cmp(&left.1.updated_at)
                .then_with(|| left.0.0.as_ref().cmp(right.0.0.as_ref()))
        });
        sessions
    }

    fn restore_session_if_missing(
        &mut self,
        session_id: &acp::SessionId,
        cwd: &Path,
    ) -> Result<(), acp::Error> {
        if self.sessions.contains_key(session_id) {
            return Ok(());
        }

        let raw_id = session_id.0.as_ref();
        let is_mock_session = raw_id == SEEDED_SESSION_ID || raw_id.starts_with("mock-session-");
        if !is_mock_session {
            return Err(acp::Error::invalid_params());
        }

        if let Some(index) = raw_id
            .strip_prefix("mock-session-")
            .and_then(|suffix| suffix.parse::<u64>().ok())
        {
            self.next_session_index = self.next_session_index.max(index + 1);
        }

        self.sessions.insert(
            session_id.clone(),
            MockSession::restored(cwd.to_path_buf(), session_id),
        );
        Ok(())
    }
}

impl MockSession {
    fn new(cwd: PathBuf) -> Self {
        let default_title = cwd
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| format!("Mock Session: {name}"))
            .unwrap_or_else(|| "Mock Session".to_owned());

        Self {
            cwd,
            title: default_title,
            history: Vec::new(),
            updated_at: Utc::now(),
            mode_id: acp::SessionModeId::new(MODE_ASK),
            reasoning_level: acp::SessionConfigValueId::new(REASONING_STANDARD),
            approval_preset: acp::SessionConfigValueId::new(APPROVAL_DEFAULT),
            #[cfg(feature = "unstable_session_model")]
            model_id: acp::ModelId::new(MODEL_FAST),
            closed: false,
        }
    }
}

enum PromptBehavior {
    Echo(String),
    CreatePlan,
    SummarizeSession,
    Think { topic: String },
    Search { query: String },
    Fetch { resource: String },
    EditFile { path: Option<PathBuf> },
    DeleteFile { path: Option<PathBuf> },
    MoveFile { from: Option<PathBuf>, to: Option<PathBuf> },
    ReadFile { path: Option<PathBuf> },
    WriteFile { path: Option<PathBuf>, content: Option<String> },
    RequestPermission,
    Terminal {
        command: Option<String>,
        args: Vec<String>,
    },
    TerminalKill {
        command: Option<String>,
        args: Vec<String>,
    },
}

struct MockPromptOutcome {
    assistant_message: Option<String>,
}

impl MockPromptOutcome {
    fn assistant(message: impl Into<String>) -> Self {
        Self {
            assistant_message: Some(message.into()),
        }
    }
}

#[derive(Clone)]
pub(super) struct MockAgent {
    state: Arc<Mutex<MockState>>,
    session_update_tx: mpsc::UnboundedSender<NotificationEnvelope>,
    client_connection: Rc<RefCell<Option<Rc<acp::AgentSideConnection>>>>,
}

impl MockAgent {
    pub(super) fn new(session_update_tx: mpsc::UnboundedSender<NotificationEnvelope>) -> Self {
        Self {
            state: Arc::new(Mutex::new(MockState::seeded())),
            session_update_tx,
            client_connection: Rc::new(RefCell::new(None)),
        }
    }

    pub(super) fn set_client_connection(&self, connection: Rc<acp::AgentSideConnection>) {
        *self.client_connection.borrow_mut() = Some(connection);
    }

    fn client_connection(&self) -> Result<Rc<acp::AgentSideConnection>, acp::Error> {
        self.client_connection
            .borrow()
            .as_ref()
            .cloned()
            .ok_or_else(acp::Error::internal_error)
    }

    async fn emit(
        &self,
        session_id: &acp::SessionId,
        update: acp::SessionUpdate,
    ) -> Result<(), acp::Error> {
        let (ack_tx, ack_rx) = oneshot::channel();
        self.session_update_tx
            .send((
                acp::SessionNotification::new(session_id.clone(), update),
                ack_tx,
            ))
            .map_err(|_| acp::Error::internal_error())?;
        ack_rx.await.map_err(|_| acp::Error::internal_error())
    }

    fn prompt_text(blocks: &[acp::ContentBlock]) -> String {
        let parts = blocks
            .iter()
            .map(|block| match block {
                acp::ContentBlock::Text(content) => content.text.clone(),
                acp::ContentBlock::ResourceLink(link) => format!("[resource:{}]", link.uri),
                acp::ContentBlock::Resource(_) => "[embedded-resource]".to_owned(),
                acp::ContentBlock::Image(_) => "[image]".to_owned(),
                acp::ContentBlock::Audio(_) => "[audio]".to_owned(),
                _ => "[unsupported-content]".to_owned(),
            })
            .collect::<Vec<_>>();

        let text = parts.join(" ").trim().to_owned();
        if text.is_empty() {
            "(empty prompt)".to_owned()
        } else {
            text
        }
    }

    fn response_chunks(response: &str) -> Vec<String> {
        if response.len() <= 24 {
            return vec![response.to_owned()];
        }

        let midpoint = response.len() / 2;
        let split_index = response
            .char_indices()
            .find_map(|(index, _)| (index >= midpoint).then_some(index))
            .unwrap_or(response.len());
        let split_index = response[split_index..]
            .find(' ')
            .map(|offset| split_index + offset + 1)
            .unwrap_or(split_index);

        if split_index == 0 || split_index >= response.len() {
            return vec![response.to_owned()];
        }

        vec![
            response[..split_index].to_owned(),
            response[split_index..].to_owned(),
        ]
    }

    fn session_title_from_prompt(prompt_text: &str) -> String {
        let trimmed = prompt_text.trim();
        let candidate = if trimmed.len() > 40 {
            format!("Mock: {}...", &trimmed[..40])
        } else {
            format!("Mock: {trimmed}")
        };

        if candidate.trim().is_empty() {
            "Mock Session".to_owned()
        } else {
            candidate
        }
    }

    fn parse_prompt_behavior(prompt_text: &str) -> PromptBehavior {
        let trimmed = prompt_text.trim();

        if Self::matches_mock_command(trimmed, "create-plan") {
            return PromptBehavior::CreatePlan;
        }

        if Self::matches_mock_command(trimmed, "summarize-session") {
            return PromptBehavior::SummarizeSession;
        }

        if let Some(topic) = Self::mock_command_arg_or_default(trimmed, "think", "the current task")
        {
            return PromptBehavior::Think {
                topic: topic.to_owned(),
            };
        }

        if let Some(query) = Self::mock_command_arg_or_default(trimmed, "search", "mock-query") {
            return PromptBehavior::Search {
                query: query.to_owned(),
            };
        }

        if let Some(resource) =
            Self::mock_command_arg_or_default(trimmed, "fetch", "https://example.invalid/mock")
        {
            return PromptBehavior::Fetch {
                resource: resource.to_owned(),
            };
        }

        if let Some(rest) = Self::strip_mock_command_arg(trimmed, "edit-file") {
            return PromptBehavior::EditFile {
                path: Some(PathBuf::from(rest.trim())),
            };
        }
        if Self::matches_mock_command(trimmed, "edit-file") {
            return PromptBehavior::EditFile { path: None };
        }

        if let Some(rest) = Self::strip_mock_command_arg(trimmed, "delete-file") {
            return PromptBehavior::DeleteFile {
                path: Some(PathBuf::from(rest.trim())),
            };
        }
        if Self::matches_mock_command(trimmed, "delete-file") {
            return PromptBehavior::DeleteFile { path: None };
        }

        if let Some(rest) = Self::strip_mock_command_arg(trimmed, "move-file") {
            let mut parts = rest.trim().splitn(2, ' ');
            return PromptBehavior::MoveFile {
                from: parts.next().map(PathBuf::from),
                to: parts.next().map(PathBuf::from),
            };
        }
        if Self::matches_mock_command(trimmed, "move-file") {
            return PromptBehavior::MoveFile {
                from: None,
                to: None,
            };
        }

        if let Some(rest) = Self::strip_mock_command_arg(trimmed, "read-file") {
            return PromptBehavior::ReadFile {
                path: Some(PathBuf::from(rest.trim())),
            };
        }
        if Self::matches_mock_command(trimmed, "read-file") {
            return PromptBehavior::ReadFile { path: None };
        }

        if let Some(rest) = Self::strip_mock_command_arg(trimmed, "write-file") {
            let mut parts = rest.trim().splitn(2, ' ');
            return PromptBehavior::WriteFile {
                path: parts.next().map(PathBuf::from),
                content: parts.next().map(ToOwned::to_owned),
            };
        }
        if Self::matches_mock_command(trimmed, "write-file") {
            return PromptBehavior::WriteFile {
                path: None,
                content: None,
            };
        }

        if Self::matches_mock_command(trimmed, "request-permission") {
            return PromptBehavior::RequestPermission;
        }

        if let Some(rest) = Self::strip_mock_command_arg(trimmed, "terminal-kill") {
            let parts = rest
                .split_whitespace()
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>();
            let (command, args) = if let Some((command, args)) = parts.split_first() {
                (Some(command.clone()), args.to_vec())
            } else {
                (None, Vec::new())
            };
            return PromptBehavior::TerminalKill { command, args };
        }
        if Self::matches_mock_command(trimmed, "terminal-kill") {
            return PromptBehavior::TerminalKill {
                command: None,
                args: Vec::new(),
            };
        }

        if let Some(rest) = Self::strip_mock_command_arg(trimmed, "terminal") {
            let parts = rest
                .split_whitespace()
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>();
            let (command, args) = if let Some((command, args)) = parts.split_first() {
                (Some(command.clone()), args.to_vec())
            } else {
                (None, Vec::new())
            };
            return PromptBehavior::Terminal { command, args };
        }
        if Self::matches_mock_command(trimmed, "terminal") {
            return PromptBehavior::Terminal {
                command: None,
                args: Vec::new(),
            };
        }

        PromptBehavior::Echo(prompt_text.to_owned())
    }

    fn matches_mock_command(prompt_text: &str, command: &str) -> bool {
        prompt_text == format!("mock:{command}")
    }

    fn strip_mock_command_arg<'a>(prompt_text: &'a str, command: &str) -> Option<&'a str> {
        prompt_text.strip_prefix(&format!("mock:{command} "))
    }

    fn mock_command_arg_or_default<'a>(
        prompt_text: &'a str,
        command: &str,
        default: &'a str,
    ) -> Option<&'a str> {
        if Self::matches_mock_command(prompt_text, command) {
            Some(default)
        } else {
            Self::strip_mock_command_arg(prompt_text, command).map(str::trim)
        }
    }

    fn next_tool_call_id(&self) -> Result<acp::ToolCallId, acp::Error> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| acp::Error::internal_error())?;
        let id = acp::ToolCallId::new(format!("mock-tool-call-{}", state.next_tool_call_index));
        state.next_tool_call_index += 1;
        Ok(id)
    }

    fn session_modes() -> Vec<acp::SessionMode> {
        vec![
            acp::SessionMode::new(MODE_ASK, "Ask")
                .description("Answer questions without pretending to modify project state."),
            acp::SessionMode::new(MODE_ARCHITECT, "Architect")
                .description("Plan the work and explain the intended implementation shape."),
            acp::SessionMode::new(MODE_CODE, "Code")
                .description("Act like an implementation-focused coding agent."),
        ]
    }

    fn default_available_commands() -> Vec<AvailableCommand> {
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

    fn config_options(session: &MockSession) -> Vec<acp::SessionConfigOption> {
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
        ]
    }

    fn session_mode_state(session: &MockSession) -> acp::SessionModeState {
        acp::SessionModeState::new(session.mode_id.clone(), Self::session_modes())
    }

    #[cfg(feature = "unstable_session_model")]
    fn model_state(session: &MockSession) -> acp::SessionModelState {
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

    fn info_update(session: &MockSession) -> acp::SessionInfoUpdate {
        acp::SessionInfoUpdate::new()
            .title(session.title.clone())
            .updated_at(session.updated_at.to_rfc3339())
    }

    fn list_info(session_id: acp::SessionId, session: &MockSession) -> acp::SessionInfo {
        acp::SessionInfo::new(session_id, session.cwd.clone())
            .title(session.title.clone())
            .updated_at(session.updated_at.to_rfc3339())
    }

    fn ext_response(
        &self,
        method: &str,
        params: &serde_json::Value,
    ) -> Result<acp::ExtResponse, acp::Error> {
        let payload = match method {
            "brain/ping" => json!({
                "ok": true,
                "agent": "brain-acp-mock",
                "echo": params,
            }),
            "brain/session-count" => {
                let session_count = self
                    .state
                    .lock()
                    .map_err(|_| acp::Error::internal_error())?
                    .visible_sessions()
                    .len();
                json!({
                    "session_count": session_count,
                })
            }
            _ => return Err(acp::Error::method_not_found()),
        };

        let raw = to_raw_value(&payload).map_err(|_| acp::Error::internal_error())?;
        Ok(acp::ExtResponse::new(raw.into()))
    }

    fn parse_ext_params(params: &std::sync::Arc<serde_json::value::RawValue>) -> serde_json::Value {
        serde_json::from_str(params.get()).unwrap_or(serde_json::Value::Null)
    }

    async fn emit_prompt_metadata(&self, session_id: &acp::SessionId) -> Result<(), acp::Error> {
        self.emit(
            session_id,
            acp::SessionUpdate::AvailableCommandsUpdate(AvailableCommandsUpdate::new(
                Self::default_available_commands(),
            )),
        )
        .await
    }

    async fn run_client_prompt_behavior(
        &self,
        session_id: &acp::SessionId,
        behavior: PromptBehavior,
    ) -> Result<MockPromptOutcome, acp::Error> {
        match behavior {
            PromptBehavior::Echo(prompt_text) => {
                Ok(MockPromptOutcome::assistant(format!("Mock brain-acp response: {prompt_text}")))
            }
            PromptBehavior::CreatePlan => {
                self.emit_mock_plan(session_id).await?;
                Ok(MockPromptOutcome::assistant("Mock plan emitted."))
            }
            PromptBehavior::SummarizeSession => Ok(MockPromptOutcome::assistant(
                self.summarize_session(session_id)?,
            )),
            PromptBehavior::Think { topic } => self.emit_mock_think(session_id, &topic).await,
            PromptBehavior::Search { query } => self.emit_mock_search(session_id, &query).await,
            PromptBehavior::Fetch { resource } => self.emit_mock_fetch(session_id, &resource).await,
            PromptBehavior::EditFile { path } => self.emit_mock_edit_file(session_id, path).await,
            PromptBehavior::DeleteFile { path } => {
                self.emit_mock_delete_file(session_id, path).await
            }
            PromptBehavior::MoveFile { from, to } => {
                self.emit_mock_move_file(session_id, from, to).await
            }
            PromptBehavior::ReadFile { path } => Ok(self.read_file_via_client(session_id, path).await),
            PromptBehavior::WriteFile { path, content } => {
                Ok(self.write_file_via_client(session_id, path, content).await)
            }
            PromptBehavior::RequestPermission => Ok(self.request_permission_via_client(session_id).await),
            PromptBehavior::Terminal { command, args } => {
                Ok(self
                    .run_terminal_via_client(session_id, command, args, false)
                    .await)
            }
            PromptBehavior::TerminalKill { command, args } => {
                Ok(self
                    .run_terminal_via_client(session_id, command, args, true)
                    .await)
            }
        }
    }

    fn summarize_session(&self, session_id: &acp::SessionId) -> Result<String, acp::Error> {
        let state = self
            .state
            .lock()
            .map_err(|_| acp::Error::internal_error())?;
        let session = state.get_session(session_id)?;
        let user_messages = session
            .history
            .iter()
            .filter(|message| matches!(message.role, MessageRole::User))
            .count();
        let agent_messages = session
            .history
            .iter()
            .filter(|message| matches!(message.role, MessageRole::Agent))
            .count();

        Ok(format!(
            "Mock session summary: title=\"{}\", cwd=\"{}\", user_messages={}, agent_messages={}, mode={}, approval={}",
            session.title,
            session.cwd.display(),
            user_messages,
            agent_messages,
            session.mode_id.0.as_ref(),
            session.approval_preset.0.as_ref()
        ))
    }

    fn session_cwd(&self, session_id: &acp::SessionId) -> Result<PathBuf, acp::Error> {
        let state = self
            .state
            .lock()
            .map_err(|_| acp::Error::internal_error())?;
        Ok(state.get_session(session_id)?.cwd.clone())
    }

    fn default_read_path(&self, session_id: &acp::SessionId) -> Result<PathBuf, acp::Error> {
        let cwd = self.session_cwd(session_id)?;
        for candidate in ["Cargo.toml", "README.md", "AGENTS.md"] {
            let path = cwd.join(candidate);
            if path.exists() {
                return Ok(path);
            }
        }
        Ok(cwd.join("Cargo.toml"))
    }

    fn default_mock_path(
        &self,
        session_id: &acp::SessionId,
        leaf: &str,
    ) -> Result<PathBuf, acp::Error> {
        Ok(self.session_cwd(session_id)?.join(leaf))
    }

    async fn emit_mock_plan(&self, session_id: &acp::SessionId) -> Result<(), acp::Error> {
        self.emit(
            session_id,
            acp::SessionUpdate::Plan(Plan::new(vec![
                PlanEntry::new(
                    "Capture the requested mock workflow".to_owned(),
                    PlanEntryPriority::High,
                    PlanEntryStatus::Completed,
                ),
                PlanEntry::new(
                    "Emit deterministic ACP activity".to_owned(),
                    PlanEntryPriority::High,
                    PlanEntryStatus::InProgress,
                ),
                PlanEntry::new(
                    "Finalize the mock turn".to_owned(),
                    PlanEntryPriority::Medium,
                    PlanEntryStatus::Pending,
                ),
            ])),
        )
        .await?;

        tokio::time::sleep(Duration::from_millis(30)).await;

        self.emit(
            session_id,
            acp::SessionUpdate::Plan(Plan::new(vec![
                PlanEntry::new(
                    "Capture the requested mock workflow".to_owned(),
                    PlanEntryPriority::High,
                    PlanEntryStatus::Completed,
                ),
                PlanEntry::new(
                    "Emit deterministic ACP activity".to_owned(),
                    PlanEntryPriority::High,
                    PlanEntryStatus::Completed,
                ),
                PlanEntry::new(
                    "Finalize the mock turn".to_owned(),
                    PlanEntryPriority::Medium,
                    PlanEntryStatus::Completed,
                ),
            ])),
        )
        .await
    }

    async fn emit_mock_tool_result(
        &self,
        session_id: &acp::SessionId,
        title: &str,
        kind: acp::ToolKind,
        raw_input: serde_json::Value,
        locations: Vec<acp::ToolCallLocation>,
        output: impl Into<String>,
        raw_output: Option<serde_json::Value>,
    ) -> Result<(), acp::Error> {
        let tool_call_id = self.next_tool_call_id()?;
        let title_owned = title.to_owned();
        let output = output.into();

        self.emit(
            session_id,
            acp::SessionUpdate::ToolCall(
                acp::ToolCall::new(tool_call_id.clone(), title_owned.clone())
                    .kind(kind)
                    .status(acp::ToolCallStatus::Pending)
                    .raw_input(raw_input.clone())
                    .locations(locations.clone()),
            ),
        )
        .await?;

        self.emit(
            session_id,
            acp::SessionUpdate::ToolCallUpdate(acp::ToolCallUpdate::new(
                tool_call_id.clone(),
                acp::ToolCallUpdateFields::new()
                    .title(title_owned.clone())
                    .kind(kind)
                    .status(acp::ToolCallStatus::InProgress)
                    .raw_input(raw_input.clone())
                    .locations(locations.clone()),
            )),
        )
        .await?;

        self.emit(
            session_id,
            acp::SessionUpdate::ToolCallUpdate(acp::ToolCallUpdate::new(
                tool_call_id,
                {
                    let fields = acp::ToolCallUpdateFields::new()
                    .title(title_owned)
                    .kind(kind)
                    .status(acp::ToolCallStatus::Completed)
                    .raw_input(raw_input)
                    .locations(locations)
                    .content(vec![output.into()]);
                    if let Some(raw_output) = raw_output {
                        fields.raw_output(raw_output)
                    } else {
                        fields
                    }
                },
            )),
        )
        .await
    }

    async fn emit_mock_think(
        &self,
        session_id: &acp::SessionId,
        topic: &str,
    ) -> Result<MockPromptOutcome, acp::Error> {
        for thought in [
            format!("Considering a synthetic plan for: {topic}"),
            "Checking which higher-level ACP updates should be shown.".to_owned(),
            "Settling on a deterministic mock answer.".to_owned(),
        ] {
            self.emit(
                session_id,
                acp::SessionUpdate::AgentThoughtChunk(acp::ContentChunk::new(thought.into())),
            )
            .await?;
            tokio::time::sleep(Duration::from_millis(20)).await;
        }

        self.emit_mock_tool_result(
            session_id,
            "Think",
            acp::ToolKind::Think,
            json!({ "topic": topic }),
            Vec::new(),
            format!("mock reasoning summary for {topic}"),
            Some(json!({ "steps": 3, "topic": topic })),
        )
        .await?;

        Ok(MockPromptOutcome::assistant(format!(
            "Mock reasoning probe completed for: {topic}."
        )))
    }

    async fn emit_mock_search(
        &self,
        session_id: &acp::SessionId,
        query: &str,
    ) -> Result<MockPromptOutcome, acp::Error> {
        self.emit_mock_tool_result(
            session_id,
            "Search workspace",
            acp::ToolKind::Search,
            json!({ "pattern": query }),
            Vec::new(),
            format!("mock search results for `{query}`: 3 synthetic matches"),
            Some(json!({ "matches": 3, "pattern": query })),
        )
        .await?;

        Ok(MockPromptOutcome::assistant(format!(
            "Mock search probe completed for `{query}`."
        )))
    }

    async fn emit_mock_fetch(
        &self,
        session_id: &acp::SessionId,
        resource: &str,
    ) -> Result<MockPromptOutcome, acp::Error> {
        self.emit_mock_tool_result(
            session_id,
            "Fetch resource",
            acp::ToolKind::Fetch,
            json!({ "resource": resource }),
            Vec::new(),
            format!("mock fetched content for {resource}"),
            Some(json!({ "bytes": 128, "resource": resource, "status": "ok" })),
        )
        .await?;

        Ok(MockPromptOutcome::assistant(format!(
            "Mock fetch probe completed for `{resource}`."
        )))
    }

    async fn emit_mock_edit_file(
        &self,
        session_id: &acp::SessionId,
        path: Option<PathBuf>,
    ) -> Result<MockPromptOutcome, acp::Error> {
        let path = path.unwrap_or(self.default_mock_path(session_id, "mock-edit.rs")?);
        let path_text = path.display().to_string();
        self.emit_mock_tool_result(
            session_id,
            "Edit",
            acp::ToolKind::Edit,
            json!({
                "file_path": path_text,
                "old_string": "fn old() {}\n",
                "new_string": "fn new() {\n    println!(\"mock edit\");\n}\n"
            }),
            vec![acp::ToolCallLocation::new(path_text.clone())],
            format!("mock edit prepared for {path_text}"),
            Some(json!({ "changed_lines": 3, "file_path": path_text })),
        )
        .await?;

        Ok(MockPromptOutcome::assistant(format!(
            "Mock edit probe completed for {}.",
            path.display()
        )))
    }

    async fn emit_mock_delete_file(
        &self,
        session_id: &acp::SessionId,
        path: Option<PathBuf>,
    ) -> Result<MockPromptOutcome, acp::Error> {
        let path = path.unwrap_or(self.default_mock_path(session_id, "mock-delete.rs")?);
        let path_text = path.display().to_string();
        self.emit_mock_tool_result(
            session_id,
            "Delete",
            acp::ToolKind::Delete,
            json!({
                "file_path": path_text,
                "content": "// mock deleted content\n"
            }),
            vec![acp::ToolCallLocation::new(path_text.clone())],
            format!("mock delete prepared for {path_text}"),
            Some(json!({ "deleted": true, "file_path": path_text })),
        )
        .await?;

        Ok(MockPromptOutcome::assistant(format!(
            "Mock delete probe completed for {}.",
            path.display()
        )))
    }

    async fn emit_mock_move_file(
        &self,
        session_id: &acp::SessionId,
        from: Option<PathBuf>,
        to: Option<PathBuf>,
    ) -> Result<MockPromptOutcome, acp::Error> {
        let from = from.unwrap_or(self.default_mock_path(session_id, "mock-old.rs")?);
        let to = to.unwrap_or(self.default_mock_path(session_id, "mock-new.rs")?);
        let from_text = from.display().to_string();
        let to_text = to.display().to_string();
        self.emit_mock_tool_result(
            session_id,
            "Move",
            acp::ToolKind::Move,
            json!({
                "from": from_text,
                "to": to_text
            }),
            vec![
                acp::ToolCallLocation::new(from_text.clone()),
                acp::ToolCallLocation::new(to_text.clone()),
            ],
            format!("mock move prepared from {from_text} to {to_text}"),
            Some(json!({ "moved": true, "from": from_text, "to": to_text })),
        )
        .await?;

        Ok(MockPromptOutcome::assistant(format!(
            "Mock move probe completed from {} to {}.",
            from.display(),
            to.display()
        )))
    }

    async fn read_file_via_client(
        &self,
        session_id: &acp::SessionId,
        path: Option<PathBuf>,
    ) -> MockPromptOutcome {
        let client = match self.client_connection() {
            Ok(client) => client,
            Err(_) => {
                return MockPromptOutcome::assistant(
                    "Mock ACP error: client connection was not initialized.",
                );
            }
        };

        let path = match path {
            Some(path) => path,
            None => match self.default_read_path(session_id) {
                Ok(path) => path,
                Err(error) => {
                    return MockPromptOutcome::assistant(format!(
                        "Mock ACP read file probe could not determine a default path: {error}"
                    ));
                }
            },
        };
        let path_text = path.display().to_string();
        match client
            .read_text_file(acp::ReadTextFileRequest::new(
                session_id.clone(),
                path.clone(),
            ))
            .await
        {
            Ok(_) => {
                if let Err(error) = self
                    .emit_mock_tool_result(
                        session_id,
                        "Reading file",
                        acp::ToolKind::Read,
                        json!({ "path": path_text }),
                        vec![acp::ToolCallLocation::new(path_text.clone())],
                        format!("mock content for {path_text}"),
                        Some(json!({ "lines": 12, "path": path_text })),
                    )
                    .await
                {
                    return MockPromptOutcome::assistant(format!(
                        "Mock ACP read file probe failed while streaming tool output for {}: {error}",
                        path.display()
                    ));
                }

                MockPromptOutcome::assistant(format!(
                    "Mock file read probe completed for {}.",
                    path.display()
                ))
            }
            Err(error) => MockPromptOutcome::assistant(format!(
                "Mock ACP read file request failed for {}: {error}",
                path.display()
            )),
        }
    }

    async fn write_file_via_client(
        &self,
        session_id: &acp::SessionId,
        path: Option<PathBuf>,
        content: Option<String>,
    ) -> MockPromptOutcome {
        let path = match path {
            Some(path) => path,
            None => match self.default_mock_path(session_id, "mock-output.txt") {
                Ok(path) => path,
                Err(error) => {
                    return MockPromptOutcome::assistant(format!(
                        "Mock ACP write file probe could not determine a default path: {error}"
                    ));
                }
            },
        };
        let content = content.unwrap_or_else(|| "hello-from-mock-write".to_owned());
        let path_text = path.display().to_string();
        if let Err(error) = self
            .emit_mock_tool_result(
                session_id,
                "Writing file",
                acp::ToolKind::Execute,
                json!({ "path": path_text, "bytes": content.len() }),
                vec![acp::ToolCallLocation::new(path_text.clone())],
                format!("mock write completed for {path_text}"),
                Some(json!({ "bytes": content.len(), "path": path_text, "synthetic": true })),
            )
            .await
        {
            return MockPromptOutcome::assistant(format!(
                "Mock ACP write file probe failed while streaming tool output for {}: {error}",
                path.display()
            ));
        }

        MockPromptOutcome::assistant(format!(
            "Mock file write probe completed for {}.",
            path.display()
        ))
    }

    async fn request_permission_via_client(&self, session_id: &acp::SessionId) -> MockPromptOutcome {
        let client = match self.client_connection() {
            Ok(client) => client,
            Err(_) => {
                return MockPromptOutcome::assistant(
                    "Mock ACP error: client connection was not initialized.",
                );
            }
        };

        let tool_call_id = match self.next_tool_call_id() {
            Ok(tool_call_id) => tool_call_id,
            Err(_) => {
                return MockPromptOutcome::assistant(
                    "Mock ACP error: could not allocate permission request ID.",
                );
            }
        };

        let tool_call_title = "Mock permission probe";

        if let Err(error) = self
            .emit(
                session_id,
                acp::SessionUpdate::ToolCall(
                    acp::ToolCall::new(tool_call_id.clone(), tool_call_title)
                        .kind(acp::ToolKind::Execute)
                        .status(acp::ToolCallStatus::Pending)
                        .raw_input(json!({ "command": "mock:request-permission" })),
                ),
            )
            .await
        {
            return MockPromptOutcome::assistant(format!(
                "Mock ACP permission request failed before client approval: {error}"
            ));
        }

        let request = acp::RequestPermissionRequest::new(
            session_id.clone(),
            acp::ToolCallUpdate::new(
                tool_call_id.clone(),
                acp::ToolCallUpdateFields::new()
                    .status(acp::ToolCallStatus::Pending)
                    .title(tool_call_title)
                    .kind(acp::ToolKind::Execute)
                    .raw_input(json!({ "command": "mock:request-permission" })),
            ),
            vec![
                acp::PermissionOption::new(
                    "allow-once",
                    "Allow Once",
                    acp::PermissionOptionKind::AllowOnce,
                ),
                acp::PermissionOption::new(
                    "reject-once",
                    "Reject Once",
                    acp::PermissionOptionKind::RejectOnce,
                ),
            ],
        );

        match client.request_permission(request).await {
            Ok(response) => {
                let output = match &response.outcome {
                    acp::RequestPermissionOutcome::Cancelled => {
                        "mock permission outcome: cancelled".to_owned()
                    }
                    acp::RequestPermissionOutcome::Selected(selected) => format!(
                        "mock permission outcome: {}",
                        selected.option_id.0.as_ref()
                    ),
                    _ => "mock permission outcome: unknown".to_owned(),
                };

                if let Err(error) = self
                    .emit(
                        session_id,
                        acp::SessionUpdate::ToolCallUpdate(acp::ToolCallUpdate::new(
                            tool_call_id,
                            acp::ToolCallUpdateFields::new()
                                .title(tool_call_title)
                                .kind(acp::ToolKind::Execute)
                                .status(acp::ToolCallStatus::Completed)
                                .raw_input(json!({ "command": "mock:request-permission" }))
                                .content(vec![output.clone().into()]),
                        )),
                    )
                    .await
                {
                    return MockPromptOutcome::assistant(format!(
                        "Mock ACP permission request completed, but tool output streaming failed: {error}"
                    ));
                }

                match response.outcome {
                    acp::RequestPermissionOutcome::Cancelled => MockPromptOutcome::assistant(
                        "Mock permission probe completed: cancelled.",
                    ),
                    acp::RequestPermissionOutcome::Selected(selected) => {
                        MockPromptOutcome::assistant(format!(
                            "Mock permission probe completed: {}.",
                            selected.option_id.0.as_ref()
                        ))
                    }
                    _ => MockPromptOutcome::assistant(
                        "Mock permission probe completed: unknown outcome.",
                    ),
                }
            }
            Err(error) => MockPromptOutcome::assistant(format!(
                "Mock ACP permission request failed: {error}"
            )),
        }
    }

    async fn run_terminal_via_client(
        &self,
        session_id: &acp::SessionId,
        command: Option<String>,
        args: Vec<String>,
        kill_after_create: bool,
    ) -> MockPromptOutcome {
        let client = match self.client_connection() {
            Ok(client) => client,
            Err(_) => {
                return MockPromptOutcome::assistant(
                    "Mock ACP error: client connection was not initialized.",
                );
            }
        };

        let (command, args) = if let Some(command) = command {
            (command, args)
        } else if kill_after_create {
            ("sleep".to_owned(), vec!["5".to_owned()])
        } else {
            ("echo".to_owned(), vec!["hello-from-mock-terminal".to_owned()])
        };

        let create = client
            .create_terminal(
                acp::CreateTerminalRequest::new(session_id.clone(), command.clone())
                    .args(args.clone()),
            )
            .await;

        let terminal_id = match create {
            Ok(response) => response.terminal_id,
            Err(error) => {
                return MockPromptOutcome::assistant(format!(
                    "Mock ACP terminal create failed: {error}"
                ));
            }
        };

        if kill_after_create {
            if let Err(error) = client
                .kill_terminal(acp::KillTerminalRequest::new(
                    session_id.clone(),
                    terminal_id.clone(),
                ))
                .await
            {
                return MockPromptOutcome::assistant(format!(
                    "Mock ACP terminal kill failed: {error}"
                ));
            }
        }

        let output = match client
            .terminal_output(acp::TerminalOutputRequest::new(
                session_id.clone(),
                terminal_id.clone(),
            ))
            .await
        {
            Ok(output) => output,
            Err(error) => {
                return MockPromptOutcome::assistant(format!(
                    "Mock ACP terminal output failed: {error}"
                ));
            }
        };

        let exit_status = match output.exit_status.clone() {
            Some(exit_status) => exit_status,
            None => match client
                .wait_for_terminal_exit(acp::WaitForTerminalExitRequest::new(
                    session_id.clone(),
                    terminal_id.clone(),
                ))
                .await
            {
                Ok(response) => response.exit_status,
                Err(error) => {
                    return MockPromptOutcome::assistant(format!(
                        "Mock ACP terminal wait failed: {error}"
                    ));
                }
            },
        };

        if let Err(error) = client
            .release_terminal(acp::ReleaseTerminalRequest::new(
                session_id.clone(),
                terminal_id,
            ))
            .await
        {
            return MockPromptOutcome::assistant(format!(
                "Mock ACP terminal release failed: {error}"
            ));
        }

        let exit_code = exit_status
            .exit_code
            .map(|code| code.to_string())
            .or(exit_status.signal)
            .unwrap_or_else(|| "unknown".to_owned());

        let mut command_parts = vec![command.clone()];
        command_parts.extend(args.clone());
        let command_string = command_parts.join(" ");
        let output_text = format!("mock terminal output: {command_string}\nexit={exit_code}");
        if let Err(error) = self
            .emit_mock_tool_result(
                session_id,
                if kill_after_create {
                    "Running terminal command with kill"
                } else {
                    "Running terminal command"
                },
                acp::ToolKind::Execute,
                json!({ "command": command, "args": args }),
                Vec::new(),
                output_text,
                Some(json!({ "exit_code": exit_code, "synthetic": true })),
            )
            .await
        {
            return MockPromptOutcome::assistant(format!(
                "Mock ACP terminal flow completed, but tool output streaming failed: {error}"
            ));
        }

        MockPromptOutcome::assistant(format!(
            "Mock terminal probe completed for `{}` with exit={exit_code}.",
            command_string
        ))
    }

    async fn replay_session_history(
        &self,
        session_id: &acp::SessionId,
        session: &MockSession,
    ) -> Result<(), acp::Error> {
        self.emit(
            session_id,
            acp::SessionUpdate::SessionInfoUpdate(Self::info_update(session)),
        )
        .await?;

        self.emit(
            session_id,
            acp::SessionUpdate::AvailableCommandsUpdate(AvailableCommandsUpdate::new(
                Self::default_available_commands(),
            )),
        )
        .await?;

        for message in &session.history {
            let update = match message.role {
                MessageRole::User => acp::SessionUpdate::UserMessageChunk(acp::ContentChunk::new(
                    message.content.clone().into(),
                )),
                MessageRole::Agent => acp::SessionUpdate::AgentMessageChunk(
                    acp::ContentChunk::new(message.content.clone().into()),
                ),
            };
            self.emit(session_id, update).await?;
        }

        Ok(())
    }
}

#[async_trait::async_trait(?Send)]
impl acp::Agent for MockAgent {
    async fn initialize(
        &self,
        arguments: acp::InitializeRequest,
    ) -> Result<acp::InitializeResponse, acp::Error> {
        let mut capabilities = acp::AgentCapabilities::new().load_session(true);
        let mut session_capabilities =
            acp::SessionCapabilities::new().list(acp::SessionListCapabilities::new());
        #[cfg(feature = "unstable_session_fork")]
        {
            session_capabilities = session_capabilities.fork(acp::SessionForkCapabilities::new());
        }
        #[cfg(feature = "unstable_session_resume")]
        {
            session_capabilities =
                session_capabilities.resume(acp::SessionResumeCapabilities::new());
        }
        #[cfg(feature = "unstable_session_close")]
        {
            session_capabilities = session_capabilities.close(acp::SessionCloseCapabilities::new());
        }
        capabilities.session_capabilities = session_capabilities;

        Ok(acp::InitializeResponse::new(arguments.protocol_version)
            .agent_capabilities(capabilities)
            .auth_methods(vec![acp::AuthMethod::Agent(
                acp::AuthMethodAgent::new(AUTH_METHOD_ID, "Mock Browser Login")
                    .description("No-op mock authentication method for ACP client validation."),
            )])
            .agent_info(
                acp::Implementation::new("brain-acp-mock", env!("CARGO_PKG_VERSION"))
                    .title("Brain ACP Mock"),
            ))
    }

    async fn authenticate(
        &self,
        arguments: acp::AuthenticateRequest,
    ) -> Result<acp::AuthenticateResponse, acp::Error> {
        if arguments.method_id.0.as_ref() != AUTH_METHOD_ID {
            return Err(acp::Error::invalid_params());
        }
        Ok(acp::AuthenticateResponse::new())
    }

    async fn new_session(
        &self,
        arguments: acp::NewSessionRequest,
    ) -> Result<acp::NewSessionResponse, acp::Error> {
        let (session_id, session, modes, config_options) = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| acp::Error::internal_error())?;
            let session_id = state.create_session(&arguments.cwd);
            let session = state.get_session(&session_id)?.clone();
            (
                session_id,
                session.clone(),
                Self::session_mode_state(&session),
                Self::config_options(&session),
            )
        };

        self.emit(
            &session_id,
            acp::SessionUpdate::SessionInfoUpdate(Self::info_update(&session)),
        )
        .await?;
        self.emit(
            &session_id,
            acp::SessionUpdate::AvailableCommandsUpdate(AvailableCommandsUpdate::new(
                Self::default_available_commands(),
            )),
        )
        .await?;

        let response = acp::NewSessionResponse::new(session_id)
            .modes(modes)
            .config_options(config_options);
        #[cfg(feature = "unstable_session_model")]
        let response = {
            let state = self
                .state
                .lock()
                .map_err(|_| acp::Error::internal_error())?;
            let session = state.get_session(&response.session_id)?.clone();
            response.models(Self::model_state(&session))
        };

        Ok(response)
    }

    async fn load_session(
        &self,
        arguments: acp::LoadSessionRequest,
    ) -> Result<acp::LoadSessionResponse, acp::Error> {
        let session = {
            let state = self
                .state
                .lock()
                .map_err(|_| acp::Error::internal_error())?;
            drop(state);

            let mut state = self
                .state
                .lock()
                .map_err(|_| acp::Error::internal_error())?;
            state.restore_session_if_missing(&arguments.session_id, &arguments.cwd)?;
            state.get_session(&arguments.session_id)?.clone()
        };

        self.replay_session_history(&arguments.session_id, &session)
            .await?;

        let response = acp::LoadSessionResponse::new()
            .modes(Self::session_mode_state(&session))
            .config_options(Self::config_options(&session));
        #[cfg(feature = "unstable_session_model")]
        let response = response.models(Self::model_state(&session));

        Ok(response)
    }

    async fn set_session_mode(
        &self,
        arguments: acp::SetSessionModeRequest,
    ) -> Result<acp::SetSessionModeResponse, acp::Error> {
        let supported = Self::session_modes()
            .into_iter()
            .any(|mode| mode.id == arguments.mode_id);
        if !supported {
            return Err(acp::Error::invalid_params());
        }

        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| acp::Error::internal_error())?;
            let session = state.get_session_mut(&arguments.session_id)?;
            session.mode_id = arguments.mode_id.clone();
            session.updated_at = Utc::now();
        }

        self.emit(
            &arguments.session_id,
            acp::SessionUpdate::CurrentModeUpdate(CurrentModeUpdate::new(arguments.mode_id)),
        )
        .await?;

        Ok(acp::SetSessionModeResponse::new())
    }

    async fn prompt(
        &self,
        arguments: acp::PromptRequest,
    ) -> Result<acp::PromptResponse, acp::Error> {
        let prompt_text = Self::prompt_text(&arguments.prompt);
        let outcome = self
            .run_client_prompt_behavior(
                &arguments.session_id,
                Self::parse_prompt_behavior(&prompt_text),
            )
            .await;
        let outcome = outcome?;

        let (cancel_flag, title_update) = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| acp::Error::internal_error())?;
            let session = state.get_session_mut(&arguments.session_id)?;

            session.history.push(MockMessage::user(prompt_text.clone()));
            session.updated_at = Utc::now();

            let new_title = Self::session_title_from_prompt(&prompt_text);
            let title_update = if session.title != new_title {
                session.title = new_title.clone();
                Some(Self::info_update(session))
            } else {
                Some(Self::info_update(session))
            };

            let cancel_flag = Arc::new(AtomicBool::new(false));
            state
                .active_turns
                .insert(arguments.session_id.clone(), cancel_flag.clone());

            (cancel_flag, title_update)
        };

        if let Some(info_update) = title_update {
            self.emit(
                &arguments.session_id,
                acp::SessionUpdate::SessionInfoUpdate(info_update),
            )
                .await?;
        }

        self.emit_prompt_metadata(&arguments.session_id).await?;

        if let Some(assistant_message) = &outcome.assistant_message {
            for chunk in Self::response_chunks(assistant_message) {
                if cancel_flag.load(Ordering::SeqCst) {
                    let mut state = self
                        .state
                        .lock()
                        .map_err(|_| acp::Error::internal_error())?;
                    state.active_turns.remove(&arguments.session_id);
                    return Ok(acp::PromptResponse::new(acp::StopReason::Cancelled));
                }

                self.emit(
                    &arguments.session_id,
                    acp::SessionUpdate::AgentMessageChunk(acp::ContentChunk::new(chunk.into())),
                )
                .await?;
                tokio::time::sleep(Duration::from_millis(30)).await;
            }
        }

        let cancelled = cancel_flag.load(Ordering::SeqCst);
        let mut state = self
            .state
            .lock()
            .map_err(|_| acp::Error::internal_error())?;
        state.active_turns.remove(&arguments.session_id);

        if cancelled {
            return Ok(acp::PromptResponse::new(acp::StopReason::Cancelled));
        }

        if let Ok(session) = state.get_session_mut(&arguments.session_id) {
            if let Some(assistant_message) = outcome.assistant_message {
                session.history.push(MockMessage::agent(assistant_message));
            }
            session.updated_at = Utc::now();
        }

        Ok(acp::PromptResponse::new(acp::StopReason::EndTurn))
    }

    async fn cancel(&self, arguments: acp::CancelNotification) -> Result<(), acp::Error> {
        let state = self
            .state
            .lock()
            .map_err(|_| acp::Error::internal_error())?;
        if let Some(flag) = state.active_turns.get(&arguments.session_id) {
            flag.store(true, Ordering::SeqCst);
        } else {
            tracing::debug!(
                session_id = %arguments.session_id,
                "received ACP cancel for inactive mock session"
            );
        }
        Ok(())
    }

    async fn list_sessions(
        &self,
        arguments: acp::ListSessionsRequest,
    ) -> Result<acp::ListSessionsResponse, acp::Error> {
        let sessions = {
            let state = self
                .state
                .lock()
                .map_err(|_| acp::Error::internal_error())?;
            state.visible_sessions()
        };

        let filtered = sessions
            .into_iter()
            .filter(|(_, session)| match arguments.cwd.as_ref() {
                Some(cwd) => session.cwd.starts_with(cwd),
                None => true,
            })
            .collect::<Vec<_>>();

        let offset = arguments
            .cursor
            .as_deref()
            .map(|cursor| {
                cursor
                    .parse::<usize>()
                    .map_err(|_| acp::Error::invalid_params())
            })
            .transpose()?
            .unwrap_or(0);

        let page = filtered
            .iter()
            .skip(offset)
            .take(SESSION_PAGE_SIZE)
            .map(|(session_id, session)| Self::list_info(session_id.clone(), session))
            .collect::<Vec<_>>();

        let next_offset = offset + page.len();
        let response = if next_offset < filtered.len() {
            acp::ListSessionsResponse::new(page).next_cursor(next_offset.to_string())
        } else {
            acp::ListSessionsResponse::new(page)
        };

        Ok(response)
    }

    async fn set_session_config_option(
        &self,
        arguments: acp::SetSessionConfigOptionRequest,
    ) -> Result<acp::SetSessionConfigOptionResponse, acp::Error> {
        let config_options = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| acp::Error::internal_error())?;
            let session = state.get_session_mut(&arguments.session_id)?;

            match arguments.config_id.0.as_ref() {
                CONFIG_REASONING => {
                    let value = arguments.value.clone();
                    if value.0.as_ref() != REASONING_STANDARD && value.0.as_ref() != REASONING_DEEP
                    {
                        return Err(acp::Error::invalid_params());
                    }
                    session.reasoning_level = value;
                }
                CONFIG_APPROVAL => {
                    let value = arguments.value.clone();
                    if value.0.as_ref() != APPROVAL_DEFAULT && value.0.as_ref() != APPROVAL_FULL {
                        return Err(acp::Error::invalid_params());
                    }
                    session.approval_preset = value;
                }
                _ => return Err(acp::Error::invalid_params()),
            }

            session.updated_at = Utc::now();
            Self::config_options(session)
        };

        Ok(acp::SetSessionConfigOptionResponse::new(config_options))
    }

    #[cfg(feature = "unstable_session_model")]
    async fn set_session_model(
        &self,
        arguments: acp::SetSessionModelRequest,
    ) -> Result<acp::SetSessionModelResponse, acp::Error> {
        if arguments.model_id.0.as_ref() != MODEL_FAST
            && arguments.model_id.0.as_ref() != MODEL_DEEP
        {
            return Err(acp::Error::invalid_params());
        }

        let mut state = self
            .state
            .lock()
            .map_err(|_| acp::Error::internal_error())?;
        let session = state.get_session_mut(&arguments.session_id)?;
        session.model_id = arguments.model_id;
        session.updated_at = Utc::now();

        Ok(acp::SetSessionModelResponse::new())
    }

    #[cfg(feature = "unstable_session_fork")]
    async fn fork_session(
        &self,
        arguments: acp::ForkSessionRequest,
    ) -> Result<acp::ForkSessionResponse, acp::Error> {
        let (forked_session_id, forked_session) = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| acp::Error::internal_error())?;
            let source = state.get_session(&arguments.session_id)?.clone();
            let new_session_id =
                acp::SessionId::new(format!("mock-session-{}", state.next_session_index));
            state.next_session_index += 1;

            let mut forked = source.clone();
            forked.cwd = arguments.cwd;
            forked.title = format!("{} (Fork)", source.title);
            forked.updated_at = Utc::now();
            state
                .sessions
                .insert(new_session_id.clone(), forked.clone());

            (new_session_id, forked)
        };

        let response = acp::ForkSessionResponse::new(forked_session_id)
            .modes(Self::session_mode_state(&forked_session))
            .config_options(Self::config_options(&forked_session));
        #[cfg(feature = "unstable_session_model")]
        let response = response.models(Self::model_state(&forked_session));

        Ok(response)
    }

    #[cfg(feature = "unstable_session_resume")]
    async fn resume_session(
        &self,
        arguments: acp::ResumeSessionRequest,
    ) -> Result<acp::ResumeSessionResponse, acp::Error> {
        let session = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| acp::Error::internal_error())?;
            let session = state.get_session_mut(&arguments.session_id)?;
            session.cwd = arguments.cwd;
            session.updated_at = Utc::now();
            session.clone()
        };

        let response = acp::ResumeSessionResponse::new()
            .modes(Self::session_mode_state(&session))
            .config_options(Self::config_options(&session));
        #[cfg(feature = "unstable_session_model")]
        let response = response.models(Self::model_state(&session));

        Ok(response)
    }

    #[cfg(feature = "unstable_session_close")]
    async fn close_session(
        &self,
        arguments: acp::CloseSessionRequest,
    ) -> Result<acp::CloseSessionResponse, acp::Error> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| acp::Error::internal_error())?;
        let session = state
            .sessions
            .get_mut(&arguments.session_id)
            .ok_or_else(acp::Error::invalid_params)?;
        session.closed = true;
        session.updated_at = Utc::now();

        if let Some(flag) = state.active_turns.remove(&arguments.session_id) {
            flag.store(true, Ordering::SeqCst);
        }

        Ok(acp::CloseSessionResponse::new())
    }

    async fn ext_method(&self, arguments: acp::ExtRequest) -> Result<acp::ExtResponse, acp::Error> {
        let params = Self::parse_ext_params(&arguments.params);
        self.ext_response(arguments.method.as_ref(), &params)
    }

    async fn ext_notification(&self, arguments: acp::ExtNotification) -> Result<(), acp::Error> {
        tracing::debug!(
            method = %arguments.method,
            params = arguments.params.get(),
            "received mock ACP extension notification"
        );
        Ok(())
    }
}
