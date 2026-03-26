use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;

use super::types::experimental::{WorkspaceDoc, WorktreeDoc};
use super::types::mcp::{
    McpStatusConnectedDoc, McpStatusConnectedKindDoc, McpStatusDisabledDoc,
    McpStatusDisabledKindDoc, McpStatusDoc,
};
use super::types::permission::PermissionRequestDoc;
use super::types::pty::PtyCreateRequest;
use super::types::pty::{PtyDoc, PtyStatusDoc};
use super::types::question::QuestionRequestDoc;
use serde_json::{Value, json};
use tokio::sync::RwLock;
use ulid::Ulid;

#[derive(Debug, Default)]
pub(crate) struct CompatState {
    pub config: RwLock<Value>,
    pub global_config: RwLock<Value>,
    pub session_meta: RwLock<BTreeMap<String, CompatSessionMeta>>,
    pub permissions: RwLock<BTreeMap<String, PermissionRequestDoc>>,
    pub questions: RwLock<BTreeMap<String, QuestionRequestDoc>>,
    pub workspaces: RwLock<BTreeMap<String, WorkspaceDoc>>,
    pub worktrees: RwLock<BTreeMap<String, WorktreeDoc>>,
    pub mcp_servers: RwLock<BTreeMap<String, McpStatusDoc>>,
    pub ptys: RwLock<BTreeMap<String, CompatPty>>,
    pub tui_requests: RwLock<VecDeque<Value>>,
    pub tui_responses: RwLock<VecDeque<Value>>,
}

impl CompatState {
    pub fn new() -> Self {
        Self {
            config: RwLock::new(default_compat_config()),
            global_config: RwLock::new(default_compat_config()),
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct CompatSessionMeta {
    pub parent_id: Option<String>,
    pub workspace_id: Option<String>,
    pub archived_at: Option<i64>,
    pub share_url: Option<String>,
    pub permission: Value,
}

#[derive(Debug, Clone)]
pub(crate) struct CompatPty {
    pub id: String,
    pub title: String,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub status: String,
    pub pid: u32,
}

impl CompatPty {
    pub fn new(body: &PtyCreateRequest) -> Self {
        let command = body.command.clone().unwrap_or_else(|| "sh".to_owned());
        let args = body.args.clone().unwrap_or_default();
        let cwd = body.cwd.clone().unwrap_or_else(default_directory_string);
        let title = body.title.clone().unwrap_or_else(|| command.clone());
        Self {
            id: format!("pty{}", Ulid::new()),
            title,
            command,
            args,
            cwd,
            status: "running".to_owned(),
            pid: std::process::id(),
        }
    }

    pub fn as_doc(&self) -> PtyDoc {
        PtyDoc {
            id: self.id.clone(),
            title: self.title.clone(),
            command: self.command.clone(),
            args: self.args.clone(),
            cwd: self.cwd.clone(),
            status: match self.status.as_str() {
                "exited" => PtyStatusDoc::Exited,
                _ => PtyStatusDoc::Running,
            },
            pid: self.pid as f64,
        }
    }
}

pub(crate) fn mcp_connected_status() -> McpStatusDoc {
    McpStatusDoc::Connected(McpStatusConnectedDoc {
        status: McpStatusConnectedKindDoc::Connected,
    })
}

pub(crate) fn mcp_disabled_status() -> McpStatusDoc {
    McpStatusDoc::Disabled(McpStatusDisabledDoc {
        status: McpStatusDisabledKindDoc::Disabled,
    })
}

pub(crate) fn default_directory() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

pub(crate) fn default_directory_string() -> String {
    default_directory().display().to_string()
}

fn default_compat_config() -> Value {
    json!({
        "$schema": "https://opencode.ai/config.json",
        "theme": "opencode",
        "share": "manual",
        "plugin": [],
        "command": {},
        "skills": {
            "paths": [],
            "urls": [],
        },
        "watcher": {
            "ignore": [],
        },
        "agent": {
            "plan": {},
            "build": {},
            "general": {},
            "explore": {},
            "title": {},
            "summary": {},
            "compaction": {},
        },
        "provider": {},
        "mcp": {},
        "formatter": {},
        "lsp": {},
        "experimental": {},
    })
}
