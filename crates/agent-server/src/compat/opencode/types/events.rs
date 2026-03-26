use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::files::FileDiffDoc;
use super::global::EmptyPropertiesDoc;
use super::permission::PermissionRequestDoc;
use super::project::ProjectDoc;
use super::pty::PtyDoc;
use super::question::{QuestionAnswerDoc, QuestionRequestDoc};
use super::session::{
    CompatMessageErrorDoc, MessageDoc, PartDoc, SessionDoc, SessionStatusDoc, TodoDoc,
};

macro_rules! event_type_doc {
    ($name:ident, $value:literal) => {
        #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
        pub enum $name {
            #[serde(rename = $value)]
            Value,
        }
    };
}

macro_rules! event_doc {
    ($name:ident, $rename:literal, $kind:ident, $value:literal, $props:ty) => {
        event_type_doc!($kind, $value);

        #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
        #[schemars(rename = $rename)]
        pub struct $name {
            #[serde(rename = "type")]
            pub event_type: $kind,
            pub properties: $props,
        }
    };
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FileEditedPropertiesDoc {
    pub file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LspClientDiagnosticsPropertiesDoc {
    #[serde(rename = "serverID")]
    pub server_id: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SessionIdPropertiesDoc {
    #[serde(rename = "sessionID")]
    #[schemars(regex(pattern = "^ses.*"))]
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SessionInfoPropertiesDoc {
    pub info: SessionDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MessageInfoPropertiesDoc {
    pub info: MessageDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PartPropertiesDoc {
    pub part: PartDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MessageIdPropertiesDoc {
    #[serde(rename = "sessionID")]
    #[schemars(regex(pattern = "^ses.*"))]
    pub session_id: String,
    #[serde(rename = "messageID")]
    #[schemars(regex(pattern = "^msg.*"))]
    pub message_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MessagePartIdPropertiesDoc {
    #[serde(rename = "sessionID")]
    #[schemars(regex(pattern = "^ses.*"))]
    pub session_id: String,
    #[serde(rename = "messageID")]
    #[schemars(regex(pattern = "^msg.*"))]
    pub message_id: String,
    #[serde(rename = "partID")]
    #[schemars(regex(pattern = "^prt.*"))]
    pub part_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MessagePartDeltaPropertiesDoc {
    #[serde(rename = "sessionID")]
    #[schemars(regex(pattern = "^ses.*"))]
    pub session_id: String,
    #[serde(rename = "messageID")]
    #[schemars(regex(pattern = "^msg.*"))]
    pub message_id: String,
    #[serde(rename = "partID")]
    #[schemars(regex(pattern = "^prt.*"))]
    pub part_id: String,
    pub field: String,
    pub delta: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PermissionReplyEventPropertiesDoc {
    #[serde(rename = "sessionID")]
    #[schemars(regex(pattern = "^ses.*"))]
    pub session_id: String,
    #[serde(rename = "requestID")]
    #[schemars(regex(pattern = "^per.*"))]
    pub request_id: String,
    pub reply: PermissionReplyEventReplyDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum PermissionReplyEventReplyDoc {
    #[serde(rename = "once")]
    Once,
    #[serde(rename = "always")]
    Always,
    #[serde(rename = "reject")]
    Reject,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct QuestionReplyEventPropertiesDoc {
    #[serde(rename = "sessionID")]
    #[schemars(regex(pattern = "^ses.*"))]
    pub session_id: String,
    #[serde(rename = "requestID")]
    #[schemars(regex(pattern = "^que.*"))]
    pub request_id: String,
    pub answers: Vec<QuestionAnswerDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct QuestionRejectedEventPropertiesDoc {
    #[serde(rename = "sessionID")]
    #[schemars(regex(pattern = "^ses.*"))]
    pub session_id: String,
    #[serde(rename = "requestID")]
    #[schemars(regex(pattern = "^que.*"))]
    pub request_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TodoUpdatedPropertiesDoc {
    #[serde(rename = "sessionID")]
    #[schemars(regex(pattern = "^ses.*"))]
    pub session_id: String,
    pub todos: Vec<TodoDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PtyInfoPropertiesDoc {
    pub info: PtyDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PtyExitedPropertiesDoc {
    #[schemars(regex(pattern = "^pty.*"))]
    pub id: String,
    #[serde(rename = "exitCode")]
    pub exit_code: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PtyDeletedPropertiesDoc {
    #[schemars(regex(pattern = "^pty.*"))]
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FileWatcherUpdatedPropertiesDoc {
    pub file: String,
    pub event: FileWatcherEventDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum FileWatcherEventDoc {
    Add(FileWatcherAddDoc),
    Change(FileWatcherChangeDoc),
    Unlink(FileWatcherUnlinkDoc),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum FileWatcherAddDoc {
    #[serde(rename = "add")]
    Add,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum FileWatcherChangeDoc {
    #[serde(rename = "change")]
    Change,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum FileWatcherUnlinkDoc {
    #[serde(rename = "unlink")]
    Unlink,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpToolsChangedPropertiesDoc {
    pub server: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpBrowserOpenFailedPropertiesDoc {
    #[serde(rename = "mcpName")]
    pub mcp_name: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VcsBranchUpdatedPropertiesDoc {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SessionStatusPropertiesDoc {
    #[serde(rename = "sessionID")]
    #[schemars(regex(pattern = "^ses.*"))]
    pub session_id: String,
    pub status: SessionStatusDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SessionDiffPropertiesDoc {
    #[serde(rename = "sessionID")]
    #[schemars(regex(pattern = "^ses.*"))]
    pub session_id: String,
    pub diff: Vec<FileDiffDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SessionErrorPropertiesDoc {
    #[serde(default, rename = "sessionID", skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = "^ses.*"))]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<CompatMessageErrorDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CommandExecutedPropertiesDoc {
    pub name: String,
    #[serde(rename = "sessionID")]
    #[schemars(regex(pattern = "^ses.*"))]
    pub session_id: String,
    pub arguments: String,
    #[serde(rename = "messageID")]
    #[schemars(regex(pattern = "^msg.*"))]
    pub message_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct InstallationVersionPropertiesDoc {
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceReadyPropertiesDoc {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceFailedPropertiesDoc {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorktreeReadyPropertiesDoc {
    pub name: String,
    pub branch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorktreeFailedPropertiesDoc {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ServerInstanceDisposedPropertiesDoc {
    pub directory: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TuiPromptAppendPropertiesDoc {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum TuiCommandValueDoc {
    Known(TuiKnownCommandDoc),
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum TuiKnownCommandDoc {
    #[serde(rename = "session.list")]
    SessionList,
    #[serde(rename = "session.new")]
    SessionNew,
    #[serde(rename = "session.share")]
    SessionShare,
    #[serde(rename = "session.interrupt")]
    SessionInterrupt,
    #[serde(rename = "session.compact")]
    SessionCompact,
    #[serde(rename = "session.page.up")]
    SessionPageUp,
    #[serde(rename = "session.page.down")]
    SessionPageDown,
    #[serde(rename = "session.line.up")]
    SessionLineUp,
    #[serde(rename = "session.line.down")]
    SessionLineDown,
    #[serde(rename = "session.half.page.up")]
    SessionHalfPageUp,
    #[serde(rename = "session.half.page.down")]
    SessionHalfPageDown,
    #[serde(rename = "session.first")]
    SessionFirst,
    #[serde(rename = "session.last")]
    SessionLast,
    #[serde(rename = "prompt.clear")]
    PromptClear,
    #[serde(rename = "prompt.submit")]
    PromptSubmit,
    #[serde(rename = "agent.cycle")]
    AgentCycle,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TuiCommandExecutePropertiesDoc {
    pub command: TuiCommandValueDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum ToastVariantDoc {
    #[serde(rename = "info")]
    Info,
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "warning")]
    Warning,
    #[serde(rename = "error")]
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TuiToastShowPropertiesDoc {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub title: Option<String>,
    pub message: String,
    pub variant: ToastVariantDoc,
    #[serde(
        default = "toast_duration_default",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with = "f64")]
    #[schemars(
        description = "Duration in milliseconds",
        default = "toast_duration_default"
    )]
    pub duration: Option<f64>,
}

fn toast_duration_default() -> Option<f64> {
    Some(5000.0)
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TuiSessionSelectPropertiesDoc {
    #[serde(rename = "sessionID")]
    #[schemars(regex(pattern = "^ses.*"))]
    #[schemars(description = "Session ID to navigate to")]
    pub session_id: String,
}

event_doc!(
    EventServerConnectedDoc,
    "Event.server.connected",
    EventServerConnectedTypeDoc,
    "server.connected",
    EmptyPropertiesDoc
);
event_doc!(
    EventGlobalDisposedDoc,
    "Event.global.disposed",
    EventGlobalDisposedTypeDoc,
    "global.disposed",
    EmptyPropertiesDoc
);
event_doc!(
    EventTuiPromptAppendDoc,
    "Event.tui.prompt.append",
    EventTuiPromptAppendTypeDoc,
    "tui.prompt.append",
    TuiPromptAppendPropertiesDoc
);
event_doc!(
    EventTuiCommandExecuteDoc,
    "Event.tui.command.execute",
    EventTuiCommandExecuteTypeDoc,
    "tui.command.execute",
    TuiCommandExecutePropertiesDoc
);
event_doc!(
    EventTuiToastShowDoc,
    "Event.tui.toast.show",
    EventTuiToastShowTypeDoc,
    "tui.toast.show",
    TuiToastShowPropertiesDoc
);
event_doc!(
    EventTuiSessionSelectDoc,
    "Event.tui.session.select",
    EventTuiSessionSelectTypeDoc,
    "tui.session.select",
    TuiSessionSelectPropertiesDoc
);
event_doc!(
    EventInstallationUpdatedDoc,
    "Event.installation.updated",
    EventInstallationUpdatedTypeDoc,
    "installation.updated",
    InstallationVersionPropertiesDoc
);
event_doc!(
    EventInstallationUpdateAvailableDoc,
    "Event.installation.update-available",
    EventInstallationUpdateAvailableTypeDoc,
    "installation.update-available",
    InstallationVersionPropertiesDoc
);
event_doc!(
    EventProjectUpdatedDoc,
    "Event.project.updated",
    EventProjectUpdatedTypeDoc,
    "project.updated",
    ProjectDoc
);
event_doc!(
    EventWorkspaceReadyDoc,
    "Event.workspace.ready",
    EventWorkspaceReadyTypeDoc,
    "workspace.ready",
    WorkspaceReadyPropertiesDoc
);
event_doc!(
    EventWorkspaceFailedDoc,
    "Event.workspace.failed",
    EventWorkspaceFailedTypeDoc,
    "workspace.failed",
    WorkspaceFailedPropertiesDoc
);
event_doc!(
    EventServerInstanceDisposedDoc,
    "Event.server.instance.disposed",
    EventServerInstanceDisposedTypeDoc,
    "server.instance.disposed",
    ServerInstanceDisposedPropertiesDoc
);
event_doc!(
    EventWorktreeReadyDoc,
    "Event.worktree.ready",
    EventWorktreeReadyTypeDoc,
    "worktree.ready",
    WorktreeReadyPropertiesDoc
);
event_doc!(
    EventWorktreeFailedDoc,
    "Event.worktree.failed",
    EventWorktreeFailedTypeDoc,
    "worktree.failed",
    WorktreeFailedPropertiesDoc
);
event_doc!(
    EventFileEditedDoc,
    "Event.file.edited",
    EventFileEditedTypeDoc,
    "file.edited",
    FileEditedPropertiesDoc
);
event_doc!(
    EventLspClientDiagnosticsDoc,
    "Event.lsp.client.diagnostics",
    EventLspClientDiagnosticsTypeDoc,
    "lsp.client.diagnostics",
    LspClientDiagnosticsPropertiesDoc
);
event_doc!(
    EventPermissionAskedDoc,
    "Event.permission.asked",
    EventPermissionAskedTypeDoc,
    "permission.asked",
    PermissionRequestDoc
);
event_doc!(
    EventPermissionRepliedDoc,
    "Event.permission.replied",
    EventPermissionRepliedTypeDoc,
    "permission.replied",
    PermissionReplyEventPropertiesDoc
);
event_doc!(
    EventSessionStatusDoc,
    "Event.session.status",
    EventSessionStatusTypeDoc,
    "session.status",
    SessionStatusPropertiesDoc
);
event_doc!(
    EventSessionIdleDoc,
    "Event.session.idle",
    EventSessionIdleTypeDoc,
    "session.idle",
    SessionIdPropertiesDoc
);
event_doc!(
    EventQuestionAskedDoc,
    "Event.question.asked",
    EventQuestionAskedTypeDoc,
    "question.asked",
    QuestionRequestDoc
);
event_doc!(
    EventQuestionRepliedDoc,
    "Event.question.replied",
    EventQuestionRepliedTypeDoc,
    "question.replied",
    QuestionReplyEventPropertiesDoc
);
event_doc!(
    EventQuestionRejectedDoc,
    "Event.question.rejected",
    EventQuestionRejectedTypeDoc,
    "question.rejected",
    QuestionRejectedEventPropertiesDoc
);
event_doc!(
    EventTodoUpdatedDoc,
    "Event.todo.updated",
    EventTodoUpdatedTypeDoc,
    "todo.updated",
    TodoUpdatedPropertiesDoc
);
event_doc!(
    EventPtyCreatedDoc,
    "Event.pty.created",
    EventPtyCreatedTypeDoc,
    "pty.created",
    PtyInfoPropertiesDoc
);
event_doc!(
    EventPtyUpdatedDoc,
    "Event.pty.updated",
    EventPtyUpdatedTypeDoc,
    "pty.updated",
    PtyInfoPropertiesDoc
);
event_doc!(
    EventPtyExitedDoc,
    "Event.pty.exited",
    EventPtyExitedTypeDoc,
    "pty.exited",
    PtyExitedPropertiesDoc
);
event_doc!(
    EventPtyDeletedDoc,
    "Event.pty.deleted",
    EventPtyDeletedTypeDoc,
    "pty.deleted",
    PtyDeletedPropertiesDoc
);
event_doc!(
    EventFileWatcherUpdatedDoc,
    "Event.file.watcher.updated",
    EventFileWatcherUpdatedTypeDoc,
    "file.watcher.updated",
    FileWatcherUpdatedPropertiesDoc
);
event_doc!(
    EventMcpToolsChangedDoc,
    "Event.mcp.tools.changed",
    EventMcpToolsChangedTypeDoc,
    "mcp.tools.changed",
    McpToolsChangedPropertiesDoc
);
event_doc!(
    EventMcpBrowserOpenFailedDoc,
    "Event.mcp.browser.open.failed",
    EventMcpBrowserOpenFailedTypeDoc,
    "mcp.browser.open.failed",
    McpBrowserOpenFailedPropertiesDoc
);
event_doc!(
    EventLspUpdatedDoc,
    "Event.lsp.updated",
    EventLspUpdatedTypeDoc,
    "lsp.updated",
    EmptyPropertiesDoc
);
event_doc!(
    EventVcsBranchUpdatedDoc,
    "Event.vcs.branch.updated",
    EventVcsBranchUpdatedTypeDoc,
    "vcs.branch.updated",
    VcsBranchUpdatedPropertiesDoc
);
event_doc!(
    EventMessageUpdatedDoc,
    "Event.message.updated",
    EventMessageUpdatedTypeDoc,
    "message.updated",
    MessageInfoPropertiesDoc
);
event_doc!(
    EventMessageRemovedDoc,
    "Event.message.removed",
    EventMessageRemovedTypeDoc,
    "message.removed",
    MessageIdPropertiesDoc
);
event_doc!(
    EventMessagePartUpdatedDoc,
    "Event.message.part.updated",
    EventMessagePartUpdatedTypeDoc,
    "message.part.updated",
    PartPropertiesDoc
);
event_doc!(
    EventMessagePartDeltaDoc,
    "Event.message.part.delta",
    EventMessagePartDeltaTypeDoc,
    "message.part.delta",
    MessagePartDeltaPropertiesDoc
);
event_doc!(
    EventMessagePartRemovedDoc,
    "Event.message.part.removed",
    EventMessagePartRemovedTypeDoc,
    "message.part.removed",
    MessagePartIdPropertiesDoc
);
event_doc!(
    EventCommandExecutedDoc,
    "Event.command.executed",
    EventCommandExecutedTypeDoc,
    "command.executed",
    CommandExecutedPropertiesDoc
);
event_doc!(
    EventSessionCompactedDoc,
    "Event.session.compacted",
    EventSessionCompactedTypeDoc,
    "session.compacted",
    SessionIdPropertiesDoc
);
event_doc!(
    EventSessionCreatedDoc,
    "Event.session.created",
    EventSessionCreatedTypeDoc,
    "session.created",
    SessionInfoPropertiesDoc
);
event_doc!(
    EventSessionUpdatedDoc,
    "Event.session.updated",
    EventSessionUpdatedTypeDoc,
    "session.updated",
    SessionInfoPropertiesDoc
);
event_doc!(
    EventSessionDeletedDoc,
    "Event.session.deleted",
    EventSessionDeletedTypeDoc,
    "session.deleted",
    SessionInfoPropertiesDoc
);
event_doc!(
    EventSessionDiffDoc,
    "Event.session.diff",
    EventSessionDiffTypeDoc,
    "session.diff",
    SessionDiffPropertiesDoc
);
event_doc!(
    EventSessionErrorDoc,
    "Event.session.error",
    EventSessionErrorTypeDoc,
    "session.error",
    SessionErrorPropertiesDoc
);

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "Event")]
#[serde(untagged)]
pub enum EventDoc {
    ServerConnected(EventServerConnectedDoc),
    GlobalDisposed(EventGlobalDisposedDoc),
    TuiPromptAppend(EventTuiPromptAppendDoc),
    TuiCommandExecute(EventTuiCommandExecuteDoc),
    TuiToastShow(EventTuiToastShowDoc),
    TuiSessionSelect(EventTuiSessionSelectDoc),
    InstallationUpdated(EventInstallationUpdatedDoc),
    InstallationUpdateAvailable(EventInstallationUpdateAvailableDoc),
    ProjectUpdated(EventProjectUpdatedDoc),
    WorkspaceReady(EventWorkspaceReadyDoc),
    WorkspaceFailed(EventWorkspaceFailedDoc),
    ServerInstanceDisposed(EventServerInstanceDisposedDoc),
    WorktreeReady(EventWorktreeReadyDoc),
    WorktreeFailed(EventWorktreeFailedDoc),
    FileEdited(EventFileEditedDoc),
    LspClientDiagnostics(EventLspClientDiagnosticsDoc),
    PermissionAsked(EventPermissionAskedDoc),
    PermissionReplied(EventPermissionRepliedDoc),
    SessionStatus(EventSessionStatusDoc),
    SessionIdle(EventSessionIdleDoc),
    QuestionAsked(EventQuestionAskedDoc),
    QuestionReplied(EventQuestionRepliedDoc),
    QuestionRejected(EventQuestionRejectedDoc),
    TodoUpdated(EventTodoUpdatedDoc),
    PtyCreated(EventPtyCreatedDoc),
    PtyUpdated(EventPtyUpdatedDoc),
    PtyExited(EventPtyExitedDoc),
    PtyDeleted(EventPtyDeletedDoc),
    FileWatcherUpdated(EventFileWatcherUpdatedDoc),
    McpToolsChanged(EventMcpToolsChangedDoc),
    McpBrowserOpenFailed(EventMcpBrowserOpenFailedDoc),
    LspUpdated(EventLspUpdatedDoc),
    VcsBranchUpdated(EventVcsBranchUpdatedDoc),
    MessageUpdated(EventMessageUpdatedDoc),
    MessageRemoved(EventMessageRemovedDoc),
    MessagePartUpdated(EventMessagePartUpdatedDoc),
    MessagePartDelta(EventMessagePartDeltaDoc),
    MessagePartRemoved(EventMessagePartRemovedDoc),
    CommandExecuted(EventCommandExecutedDoc),
    SessionCompacted(EventSessionCompactedDoc),
    SessionCreated(EventSessionCreatedDoc),
    SessionUpdated(EventSessionUpdatedDoc),
    SessionDeleted(EventSessionDeletedDoc),
    SessionDiff(EventSessionDiffDoc),
    SessionError(EventSessionErrorDoc),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "GlobalEvent")]
pub struct GlobalEventDoc {
    pub directory: String,
    pub payload: EventDoc,
}
