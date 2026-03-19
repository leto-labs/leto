use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, atomic::AtomicBool},
};

use agent_client_protocol as acp;
use chrono::{DateTime, Utc};

pub const SEEDED_SESSION_ID: &str = "mock-seeded-session";

#[derive(Clone)]
pub(super) struct MockMessage {
    pub(super) role: MessageRole,
    pub(super) content: String,
}

impl MockMessage {
    pub(super) fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
        }
    }

    pub(super) fn agent(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Agent,
            content: content.into(),
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum MessageRole {
    User,
    Agent,
}

#[derive(Clone)]
pub(super) struct MockSession {
    pub(super) cwd: PathBuf,
    pub(super) title: String,
    pub(super) history: Vec<MockMessage>,
    pub(super) updated_at: DateTime<Utc>,
    pub(super) mode_id: acp::SessionModeId,
    pub(super) reasoning_level: acp::SessionConfigValueId,
    pub(super) approval_preset: acp::SessionConfigValueId,
    #[cfg(feature = "unstable_session_model")]
    pub(super) model_id: acp::ModelId,
    pub(super) closed: bool,
}

impl MockSession {
    pub(super) fn seeded(mode_ask: &str, reasoning_standard: &str, approval_default: &str) -> Self {
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
            mode_id: acp::SessionModeId::new(mode_ask),
            reasoning_level: acp::SessionConfigValueId::new(reasoning_standard),
            approval_preset: acp::SessionConfigValueId::new(approval_default),
            #[cfg(feature = "unstable_session_model")]
            model_id: acp::ModelId::new("brain-mock-fast"),
            closed: false,
        }
    }

    pub(super) fn restored(
        cwd: PathBuf,
        session_id: &acp::SessionId,
        mode_ask: &str,
        reasoning_standard: &str,
        approval_default: &str,
    ) -> Self {
        let mut session = Self::new(cwd, mode_ask, reasoning_standard, approval_default);
        session.title = format!("Restored Mock Session: {}", session_id.0.as_ref());
        session.history.push(MockMessage::agent(
            "This mock session was reconstructed to satisfy an external ACP client load request.",
        ));
        session
    }

    pub(super) fn new(
        cwd: PathBuf,
        mode_ask: &str,
        reasoning_standard: &str,
        approval_default: &str,
    ) -> Self {
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
            mode_id: acp::SessionModeId::new(mode_ask),
            reasoning_level: acp::SessionConfigValueId::new(reasoning_standard),
            approval_preset: acp::SessionConfigValueId::new(approval_default),
            #[cfg(feature = "unstable_session_model")]
            model_id: acp::ModelId::new("brain-mock-fast"),
            closed: false,
        }
    }
}

pub(super) struct MockState {
    pub(super) next_session_index: u64,
    pub(super) next_tool_call_index: u64,
    pub(super) sessions: HashMap<acp::SessionId, MockSession>,
    pub(super) active_turns: HashMap<acp::SessionId, Arc<AtomicBool>>,
}

impl MockState {
    pub(super) fn seeded(mode_ask: &str, reasoning_standard: &str, approval_default: &str) -> Self {
        let mut sessions = HashMap::new();
        sessions.insert(
            acp::SessionId::new(SEEDED_SESSION_ID),
            MockSession::seeded(mode_ask, reasoning_standard, approval_default),
        );

        Self {
            next_session_index: 1,
            next_tool_call_index: 1,
            sessions,
            active_turns: HashMap::new(),
        }
    }

    pub(super) fn create_session(
        &mut self,
        cwd: &Path,
        mode_ask: &str,
        reasoning_standard: &str,
        approval_default: &str,
    ) -> acp::SessionId {
        let session_id = acp::SessionId::new(format!("mock-session-{}", self.next_session_index));
        self.next_session_index += 1;
        self.sessions.insert(
            session_id.clone(),
            MockSession::new(
                cwd.to_path_buf(),
                mode_ask,
                reasoning_standard,
                approval_default,
            ),
        );
        session_id
    }

    pub(super) fn get_session(
        &self,
        session_id: &acp::SessionId,
    ) -> Result<&MockSession, acp::Error> {
        self.sessions
            .get(session_id)
            .filter(|session| !session.closed)
            .ok_or_else(acp::Error::invalid_params)
    }

    pub(super) fn get_session_mut(
        &mut self,
        session_id: &acp::SessionId,
    ) -> Result<&mut MockSession, acp::Error> {
        self.sessions
            .get_mut(session_id)
            .filter(|session| !session.closed)
            .ok_or_else(acp::Error::invalid_params)
    }

    pub(super) fn visible_sessions(&self) -> Vec<(acp::SessionId, MockSession)> {
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

    pub(super) fn restore_session_if_missing(
        &mut self,
        session_id: &acp::SessionId,
        cwd: &Path,
        mode_ask: &str,
        reasoning_standard: &str,
        approval_default: &str,
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
            MockSession::restored(
                cwd.to_path_buf(),
                session_id,
                mode_ask,
                reasoning_standard,
                approval_default,
            ),
        );
        Ok(())
    }
}
