mod routes;
pub(crate) mod state;
#[cfg(test)]
pub(crate) mod test_utils;
pub(crate) mod types;

use std::collections::BTreeMap;
use std::convert::Infallible;
use std::fs;
use std::path::{Path as FsPath, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use agent_store::{
    CredentialEntry, OAuthCredentials, Project, ProviderCredential, Session, SessionId,
    SessionUpdate, StoredMessage,
};
use axum::Router;
use axum::extract::{Path, Query, State};
use axum::http::header::AUTHORIZATION;
use axum::http::{HeaderMap, StatusCode};
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use axum::response::{IntoResponse, Json, Response};
use chrono::{DateTime, Utc};
use futures::StreamExt;
use futures::stream;
use provider::{ContentBlock, Message, MessageRole};
use serde_json::{Value, json};
use ulid::Ulid;

use self::state::{
    CompatPty, CompatSessionMeta, default_directory, default_directory_string,
    mcp_connected_status, mcp_disabled_status,
};
use self::types::common::{CompatQuery, ExperimentalSessionListQueryDoc};
use self::types::errors::{
    BadRequestErrorDoc, NotFoundDataDoc, NotFoundErrorDoc, NotFoundErrorNameDoc,
};
use self::types::permission::PermissionRuleset;
use self::types::project::{
    ProjectCommandsDoc, ProjectDoc, ProjectSummaryDoc, ProjectTimeDoc, ProjectVcsDoc,
};
use self::types::provider::{
    ApiAuthRequest, MessageTokensCacheDoc, ModelApiDoc, ModelCapabilitiesDoc, ModelCostDoc,
    ModelDoc, ModelInterleavedDoc, ModelIoDoc, ModelLimitDoc, ModelStatusDoc, OAuthAuthRequest,
    ProviderDoc, ProviderSourceDoc, WellKnownAuthRequest,
};
use self::types::session::{
    AssistantMessageDoc, AssistantMessageRoleDoc, AssistantMessageWithPartsDoc, FilePartDoc,
    FilePartKindDoc, GlobalSessionDoc, MessageDoc, MessageModelDoc, MessagePathDoc,
    MessageTimeCreatedCompletedDoc, MessageTimeCreatedDoc, MessageTokensDoc, MessageWithPartsDoc,
    NullableProjectSummaryDoc, PartDoc, SessionDoc, SessionShareDoc, SessionSummaryDoc,
    SessionTimeDoc, TextPartDoc, TextPartKindDoc, ToolPartDoc, ToolPartKindDoc,
    ToolStateCompletedDoc, ToolStateCompletedKindDoc, ToolStateCompletedTimeRangeDoc, ToolStateDoc,
    ToolStatePendingDoc, ToolStatePendingKindDoc, UserMessageDoc, UserMessageRoleDoc,
};
use self::types::session::{CommandRequest, PromptPartInputDoc, PromptRequest, ShellRequest};
use self::types::tui::{
    ExecuteCommandRequest, PromptAppendRequest, PublishRequest, SelectSessionRequest, ToastRequest,
};
use crate::server::AgentServer;
type AppState = Arc<AgentServer>;

pub fn router() -> Router<AppState> {
    routes::build_router()
}

async fn filtered_sessions(
    server: &AppState,
    params: &ExperimentalSessionListQueryDoc,
) -> Result<Vec<(Session, Option<Project>, CompatSessionMeta)>, Response> {
    let sessions = server
        .core()
        .store()
        .sessions()
        .list()
        .await
        .map_err(|error| compat_error("store_error", error.to_string()))?;
    let directory_filter = params.directory.clone();
    let roots_only = params.roots.unwrap_or(false);
    let search = params.search.as_ref().map(|value| value.to_lowercase());
    let start = params.start.map(|value| value as i64);
    let archived = params.archived.unwrap_or(false);
    let limit = params.limit.map(|value| value as usize);
    let mut rows = Vec::new();
    for session in sessions {
        let project = server.core().project(session.project_id).await.ok();
        let meta = session_meta(server, session.id).await;
        if roots_only && meta.parent_id.is_some() {
            continue;
        }
        if !archived && meta.archived_at.is_some() {
            continue;
        }
        if let Some(start) = start {
            if session.updated_at.timestamp_millis() < start {
                continue;
            }
        }
        if let Some(search) = &search {
            let title = session.title.clone().unwrap_or_default().to_lowercase();
            if !title.contains(search) {
                continue;
            }
        }
        if let Some(directory) = &directory_filter {
            let session_directory = project
                .as_ref()
                .and_then(|project| project.root.as_ref())
                .map(|root| root.display().to_string())
                .unwrap_or_else(default_directory_string);
            if &session_directory != directory {
                continue;
            }
        }
        rows.push((session, project, meta));
    }
    rows.sort_by_key(|(session, _, _)| std::cmp::Reverse(session.updated_at));
    if let Some(limit) = limit {
        rows.truncate(limit);
    }
    Ok(rows)
}

async fn resolve_current_project(
    server: &AppState,
    params: &CompatQuery,
) -> Result<Project, Response> {
    let root = query_directory(params.directory.as_deref());
    server
        .core()
        .resolve_or_create_project(root)
        .await
        .map_err(|error| compat_error("core_error", error.to_string()))
}

async fn session_meta(server: &AppState, session_id: SessionId) -> CompatSessionMeta {
    server
        .compat()
        .session_meta
        .read()
        .await
        .get(&compat_session_id(session_id))
        .cloned()
        .unwrap_or_else(|| CompatSessionMeta {
            permission: json!([]),
            ..CompatSessionMeta::default()
        })
}

fn compat_project(
    project: &Project,
    compat: &Arc<crate::compat::opencode::state::CompatState>,
) -> ProjectDoc {
    let sandboxes = compat
        .worktrees
        .blocking_read()
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    let fallback = default_directory();
    let root = project.root.as_deref().unwrap_or(fallback.as_path());
    ProjectDoc {
        id: project.id.to_string(),
        worktree: project
            .root
            .as_ref()
            .map(|root| root.display().to_string())
            .unwrap_or_else(default_directory_string),
        vcs: is_git_root(root).then_some(ProjectVcsDoc::Git),
        name: Some(
            project
                .name
                .clone()
                .unwrap_or_else(|| project.id.to_string()),
        ),
        icon: None,
        commands: ProjectCommandsDoc::default(),
        time: ProjectTimeDoc {
            created: project.created_at.timestamp_millis() as f64,
            updated: project.updated_at.timestamp_millis() as f64,
            initialized: None,
        },
        sandboxes,
    }
}

fn compat_session(
    session: &Session,
    project: Option<&Project>,
    meta: &CompatSessionMeta,
) -> SessionDoc {
    let directory = project
        .and_then(|project| project.root.as_ref())
        .map(|root| root.display().to_string())
        .unwrap_or_else(default_directory_string);
    SessionDoc {
        id: compat_session_id(session.id),
        slug: compat_session_id(session.id),
        project_id: session.project_id.to_string(),
        workspace_id: meta.workspace_id.clone(),
        directory,
        parent_id: meta.parent_id.clone(),
        summary: Some(SessionSummaryDoc {
            additions: 0.0,
            deletions: 0.0,
            files: 0.0,
            diffs: Vec::new(),
        }),
        share: meta
            .share_url
            .as_ref()
            .map(|url| SessionShareDoc { url: url.clone() }),
        title: session
            .title
            .clone()
            .unwrap_or_else(|| format!("Session {}", &compat_session_id(session.id)[3..11])),
        version: "v1".to_owned(),
        time: SessionTimeDoc {
            created: session.created_at.timestamp_millis() as f64,
            updated: session.updated_at.timestamp_millis() as f64,
            compacting: None,
            archived: meta.archived_at.map(|value| value as f64),
        },
        permission: serde_json::from_value::<PermissionRuleset>(meta.permission.clone()).ok(),
        revert: None,
    }
}

fn compat_global_session(
    session: &Session,
    project: Option<&Project>,
    meta: &CompatSessionMeta,
) -> GlobalSessionDoc {
    let session_doc = compat_session(session, project, meta);
    GlobalSessionDoc {
        id: session_doc.id,
        slug: session_doc.slug,
        project_id: session_doc.project_id,
        workspace_id: session_doc.workspace_id,
        directory: session_doc.directory,
        parent_id: session_doc.parent_id,
        summary: session_doc.summary,
        share: session_doc.share,
        title: session_doc.title,
        version: session_doc.version,
        time: session_doc.time,
        permission: session_doc.permission,
        revert: session_doc.revert,
        project: match project {
            Some(project) => NullableProjectSummaryDoc::Project(ProjectSummaryDoc {
                id: project.id.to_string(),
                name: project.name.clone(),
                worktree: project
                    .root
                    .as_ref()
                    .map(|root| root.display().to_string())
                    .unwrap_or_else(default_directory_string),
            }),
            None => NullableProjectSummaryDoc::Null(()),
        },
    }
}

fn compat_message(message: &StoredMessage, parent_id: Option<Ulid>) -> MessageDoc {
    match message.message.role {
        MessageRole::Assistant => MessageDoc::Assistant(AssistantMessageDoc {
            id: compat_message_id(message.id),
            session_id: compat_session_id(message.session_id),
            role: AssistantMessageRoleDoc::Assistant,
            time: MessageTimeCreatedCompletedDoc {
                created: message.created_at.timestamp_millis() as f64,
                completed: Some(message.created_at.timestamp_millis() as f64),
            },
            error: None,
            parent_id: compat_message_id(parent_id.unwrap_or(message.id)),
            model_id: "compat".to_owned(),
            provider_id: "compat".to_owned(),
            mode: "chat".to_owned(),
            agent: "general".to_owned(),
            path: MessagePathDoc {
                cwd: default_directory_string(),
                root: default_directory_string(),
            },
            summary: None,
            cost: 0.0,
            tokens: MessageTokensDoc {
                total: Some(0.0),
                input: 0.0,
                output: 0.0,
                reasoning: 0.0,
                cache: MessageTokensCacheDoc {
                    read: 0.0,
                    write: 0.0,
                },
            },
            structured: None,
            variant: None,
            finish: None,
        }),
        _ => MessageDoc::User(UserMessageDoc {
            id: compat_message_id(message.id),
            session_id: compat_session_id(message.session_id),
            role: UserMessageRoleDoc::User,
            time: MessageTimeCreatedDoc {
                created: message.created_at.timestamp_millis() as f64,
            },
            format: None,
            summary: None,
            agent: "general".to_owned(),
            model: MessageModelDoc {
                provider_id: "compat".to_owned(),
                model_id: "compat".to_owned(),
            },
            system: None,
            tools: None,
            variant: None,
        }),
    }
}

fn compat_message_with_parts(
    message: &StoredMessage,
    parent_id: Option<Ulid>,
) -> MessageWithPartsDoc {
    MessageWithPartsDoc {
        info: compat_message(message, parent_id),
        parts: compat_parts(message.id, message.session_id, &message.message),
    }
}

fn compat_messages_with_parts(messages: &[StoredMessage]) -> Vec<MessageWithPartsDoc> {
    messages
        .iter()
        .enumerate()
        .map(|(index, message)| {
            let parent_id = index.checked_sub(1).map(|parent| messages[parent].id);
            compat_message_with_parts(message, parent_id)
        })
        .collect()
}

fn compat_assistant_message_with_parts(
    message: &StoredMessage,
    parent_id: Option<Ulid>,
) -> Option<AssistantMessageWithPartsDoc> {
    match compat_message(message, parent_id) {
        MessageDoc::Assistant(info) => Some(AssistantMessageWithPartsDoc {
            info,
            parts: compat_parts(message.id, message.session_id, &message.message),
        }),
        MessageDoc::User(_) => None,
    }
}

fn compat_parts(message_id: Ulid, session_id: SessionId, message: &Message) -> Vec<PartDoc> {
    message
        .content
        .iter()
        .enumerate()
        .map(|(index, block)| match block {
            ContentBlock::Text { text }
            | ContentBlock::Reasoning { text }
            | ContentBlock::Refusal { text } => PartDoc::Text(TextPartDoc {
                id: compat_part_id(message_id, index),
                session_id: compat_session_id(session_id),
                message_id: compat_message_id(message_id),
                part_type: TextPartKindDoc::Text,
                text: text.clone(),
                synthetic: None,
                ignored: None,
                time: None,
                metadata: None,
            }),
            ContentBlock::ImageUrl { url } => PartDoc::File(FilePartDoc {
                id: compat_part_id(message_id, index),
                session_id: compat_session_id(session_id),
                message_id: compat_message_id(message_id),
                part_type: FilePartKindDoc::File,
                mime: "application/octet-stream".to_owned(),
                filename: None,
                url: url.clone(),
                source: None,
            }),
            ContentBlock::ToolCall { id, name, input } => PartDoc::Tool(ToolPartDoc {
                id: compat_part_id(message_id, index),
                session_id: compat_session_id(session_id),
                message_id: compat_message_id(message_id),
                part_type: ToolPartKindDoc::Tool,
                call_id: id.clone(),
                tool: name.clone(),
                state: ToolStateDoc::Pending(ToolStatePendingDoc {
                    status: ToolStatePendingKindDoc::Pending,
                    input: json_object(input),
                    raw: json_string(input),
                }),
                metadata: None,
            }),
            ContentBlock::ToolResult {
                call_id,
                output,
                is_error,
            } => PartDoc::Tool(ToolPartDoc {
                id: compat_part_id(message_id, index),
                session_id: compat_session_id(session_id),
                message_id: compat_message_id(message_id),
                part_type: ToolPartKindDoc::Tool,
                call_id: call_id.clone(),
                tool: "tool".to_owned(),
                state: ToolStateDoc::Completed(ToolStateCompletedDoc {
                    status: ToolStateCompletedKindDoc::Completed,
                    input: BTreeMap::new(),
                    output: json_string(output),
                    title: if is_error.unwrap_or(false) {
                        "error".to_owned()
                    } else {
                        "completed".to_owned()
                    },
                    metadata: BTreeMap::new(),
                    time: ToolStateCompletedTimeRangeDoc {
                        start: 0.0,
                        end: 0.0,
                        compacted: None,
                    },
                    attachments: None,
                }),
                metadata: None,
            }),
        })
        .collect()
}

enum PromptKind {
    Message,
}

async fn prompt_like(
    server: &AppState,
    session_id: &str,
    body: PromptRequest,
    kind: PromptKind,
) -> Response {
    let internal = match parse_compat_session_id(session_id) {
        Ok(id) => id,
        Err(response) => return response,
    };
    let input = match kind {
        PromptKind::Message => prompt_input_from_body(&body),
    };
    let stream = match server.core().turn(internal, input).await {
        Ok(stream) => stream,
        Err(error) => return compat_error("core_error", error.to_string()),
    };
    futures::pin_mut!(stream);
    while stream.next().await.is_some() {}
    match server.core().messages(internal).await {
        Ok(messages) => match messages.len().checked_sub(1) {
            Some(index) => {
                let parent_id = index.checked_sub(1).map(|parent| messages[parent].id);
                match compat_assistant_message_with_parts(&messages[index], parent_id) {
                    Some(message) => Json(message).into_response(),
                    None => Json(AssistantMessageWithPartsDoc {
                        info: AssistantMessageDoc {
                            id: compat_message_id(messages[index].id),
                            session_id: compat_session_id(messages[index].session_id),
                            role: AssistantMessageRoleDoc::Assistant,
                            time: MessageTimeCreatedCompletedDoc {
                                created: messages[index].created_at.timestamp_millis() as f64,
                                completed: Some(
                                    messages[index].created_at.timestamp_millis() as f64
                                ),
                            },
                            error: None,
                            parent_id: compat_message_id(parent_id.unwrap_or(messages[index].id)),
                            model_id: "compat".to_owned(),
                            provider_id: "compat".to_owned(),
                            mode: "chat".to_owned(),
                            agent: "general".to_owned(),
                            path: MessagePathDoc {
                                cwd: default_directory_string(),
                                root: default_directory_string(),
                            },
                            summary: None,
                            cost: 0.0,
                            tokens: MessageTokensDoc {
                                total: Some(0.0),
                                input: 0.0,
                                output: 0.0,
                                reasoning: 0.0,
                                cache: MessageTokensCacheDoc {
                                    read: 0.0,
                                    write: 0.0,
                                },
                            },
                            structured: None,
                            variant: None,
                            finish: None,
                        },
                        parts: compat_parts(
                            messages[index].id,
                            messages[index].session_id,
                            &messages[index].message,
                        ),
                    })
                    .into_response(),
                }
            }
            None => Json(json!({})).into_response(),
        },
        Err(error) => compat_error("core_error", error.to_string()),
    }
}

async fn prompt_command_like(
    server: &AppState,
    session_id: &str,
    body: CommandRequest,
) -> Response {
    let prompt = format!("/{} {}", body.command, body.arguments)
        .trim()
        .to_owned();
    prompt_text_like(server, session_id, prompt).await
}

async fn prompt_shell_like(server: &AppState, session_id: &str, body: ShellRequest) -> Response {
    let internal = match parse_compat_session_id(session_id) {
        Ok(id) => id,
        Err(response) => return response,
    };
    let input = vec![Message::user_text(format!(
        "Run shell command: {}",
        body.command
    ))];
    let stream = match server.core().turn(internal, input).await {
        Ok(stream) => stream,
        Err(error) => return compat_error("core_error", error.to_string()),
    };
    futures::pin_mut!(stream);
    while stream.next().await.is_some() {}
    match server.core().messages(internal).await {
        Ok(messages) => match messages.len().checked_sub(1) {
            Some(index) => {
                let parent_id = index.checked_sub(1).map(|parent| messages[parent].id);
                match compat_message(&messages[index], parent_id) {
                    MessageDoc::Assistant(message) => Json(message).into_response(),
                    MessageDoc::User(_) => compat_error(
                        "invalid_state",
                        "latest message is not an assistant message",
                    ),
                }
            }
            None => Json(json!({})).into_response(),
        },
        Err(error) => compat_error("core_error", error.to_string()),
    }
}

async fn prompt_text_like(server: &AppState, session_id: &str, prompt: String) -> Response {
    let internal = match parse_compat_session_id(session_id) {
        Ok(id) => id,
        Err(response) => return response,
    };
    let input = vec![Message::user_text(prompt)];
    let stream = match server.core().turn(internal, input).await {
        Ok(stream) => stream,
        Err(error) => return compat_error("core_error", error.to_string()),
    };
    futures::pin_mut!(stream);
    while stream.next().await.is_some() {}
    match server.core().messages(internal).await {
        Ok(messages) => match messages.len().checked_sub(1) {
            Some(index) => {
                let parent_id = index.checked_sub(1).map(|parent| messages[parent].id);
                match compat_assistant_message_with_parts(&messages[index], parent_id) {
                    Some(message) => Json(message).into_response(),
                    None => compat_error(
                        "invalid_state",
                        "latest message is not an assistant message",
                    ),
                }
            }
            None => Json(json!({})).into_response(),
        },
        Err(error) => compat_error("core_error", error.to_string()),
    }
}

fn prompt_input_from_body(body: &PromptRequest) -> Vec<Message> {
    let blocks = body
        .parts
        .iter()
        .map(|part| match part {
            PromptPartInputDoc::Text(part) => ContentBlock::text(part.text.clone()),
            PromptPartInputDoc::File(part) => ContentBlock::ImageUrl {
                url: part.url.clone(),
            },
            PromptPartInputDoc::Agent(part) => ContentBlock::text(format!("@{}", part.name)),
            PromptPartInputDoc::Subtask(part) => {
                ContentBlock::text(format!("{}\n{}", part.description, part.prompt))
            }
        })
        .collect::<Vec<_>>();
    let blocks = if blocks.is_empty() {
        vec![ContentBlock::text("")]
    } else {
        blocks
    };
    vec![Message::new(MessageRole::User, blocks)]
}

async fn mutate_message_part(
    server: &AppState,
    session_id: &str,
    message_id: &str,
    part_id: &str,
    body: Option<Value>,
) -> Response {
    let session_id = match parse_compat_session_id(session_id) {
        Ok(id) => id,
        Err(response) => return response,
    };
    let message_id = match parse_compat_message_id(message_id) {
        Ok(id) => id,
        Err(response) => return response,
    };
    let part_index = match parse_compat_part_index(message_id, part_id) {
        Ok(index) => index,
        Err(response) => return response,
    };
    let mut messages = match server.core().messages(session_id).await {
        Ok(messages) => messages,
        Err(error) => return compat_error("core_error", error.to_string()),
    };
    let Some(message) = messages.iter_mut().find(|message| message.id == message_id) else {
        return compat_error("not_found", "message not found");
    };
    if part_index >= message.message.content.len() {
        return compat_error("not_found", "part not found");
    }
    match body {
        Some(ref body) => {
            message.message.content[part_index] = content_block_from_part(&body);
        }
        None => {
            message.message.content.remove(part_index);
        }
    }
    match server
        .core()
        .store()
        .messages()
        .replace_for_session(session_id, messages)
        .await
    {
        Ok(updated) => {
            if body.is_some() {
                match updated
                    .into_iter()
                    .find(|stored| stored.id == message_id)
                    .map(|stored| compat_parts(stored.id, stored.session_id, &stored.message))
                    .and_then(|parts| parts.into_iter().nth(part_index))
                {
                    Some(part) => Json(part).into_response(),
                    None => compat_error("not_found", "part not found"),
                }
            } else {
                Json(true).into_response()
            }
        }
        Err(error) => compat_error("store_error", error.to_string()),
    }
}

fn content_block_from_part(body: &Value) -> ContentBlock {
    match body.get("type").and_then(Value::as_str) {
        Some("file") => ContentBlock::ImageUrl {
            url: body
                .get("url")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
        },
        _ => ContentBlock::text(body.get("text").and_then(Value::as_str).unwrap_or_default()),
    }
}

async fn upsert_auth_credential(
    server: &AppState,
    provider_id: &str,
    entry: CredentialEntry,
) -> Result<(), Response> {
    let core = server.core();
    let store = core.store().credentials();
    match store
        .create(
            (provider_id.to_owned(), "default".to_owned()),
            entry.clone(),
        )
        .await
    {
        Ok(_) => Ok(()),
        Err(_) => store
            .update((provider_id.to_owned(), "default".to_owned()), entry)
            .await
            .map(|_| ())
            .map_err(|error| compat_error("store_error", error.to_string())),
    }
}

async fn clear_compat_state(server: &AppState) {
    server.compat().permissions.write().await.clear();
    server.compat().questions.write().await.clear();
    server.compat().workspaces.write().await.clear();
    server.compat().worktrees.write().await.clear();
    server.compat().mcp_servers.write().await.clear();
    server.compat().ptys.write().await.clear();
    server.compat().tui_requests.write().await.clear();
    server.compat().tui_responses.write().await.clear();
}

async fn tui_enqueue(server: &AppState, path: &str, body: Value) {
    server
        .compat()
        .tui_requests
        .write()
        .await
        .push_back(json!({ "path": path, "body": body }));
}

fn compat_session_id(session_id: SessionId) -> String {
    format!("ses{session_id}")
}

fn parse_compat_session_id(value: &str) -> Result<SessionId, Response> {
    value
        .strip_prefix("ses")
        .unwrap_or(value)
        .parse()
        .map_err(|_| invalid_request("invalid_session_id"))
}

fn compat_message_id(message_id: Ulid) -> String {
    format!("msg{message_id}")
}

fn parse_compat_message_id(value: &str) -> Result<Ulid, Response> {
    value
        .strip_prefix("msg")
        .unwrap_or(value)
        .parse()
        .map_err(|_| invalid_request("invalid_message_id"))
}

fn compat_part_id(message_id: Ulid, index: usize) -> String {
    format!("prt{}_{}", compat_message_id(message_id), index)
}

fn parse_compat_part_index(message_id: Ulid, value: &str) -> Result<usize, Response> {
    let stripped = value.strip_prefix("prt").unwrap_or(value);
    let Some((message, index)) = stripped.rsplit_once('_') else {
        return Err(invalid_request("invalid_part_id"));
    };
    let parsed_message = parse_compat_message_id(message)?;
    if parsed_message != message_id {
        return Err(invalid_request("invalid_part_id"));
    }
    index
        .parse::<usize>()
        .map_err(|_| invalid_request("invalid_part_id"))
}

fn query_directory(directory: Option<&str>) -> PathBuf {
    directory
        .map(PathBuf::from)
        .unwrap_or_else(default_directory)
}

fn provider_array(server: &AppState) -> Vec<ProviderDoc> {
    let mut providers = BTreeMap::<String, Vec<crate::types::ProviderModelRecord>>::new();
    for model in server
        .core()
        .list_models()
        .into_iter()
        .map(crate::types::ProviderModelRecord::from)
    {
        providers
            .entry(model.provider_name.clone())
            .or_default()
            .push(model);
    }
    providers
        .into_iter()
        .map(|(name, models)| {
            let models = models
                .into_iter()
                .map(|model| {
                    let model_id = model.model.id.clone();
                    let cost = model.model.cost.clone();
                    let limit = model.model.limit.clone();
                    (
                        model_id.clone(),
                        ModelDoc {
                            id: model_id,
                            provider_id: model.provider_name.clone(),
                            api: ModelApiDoc {
                                id: "compat".to_owned(),
                                url: "https://example.invalid".to_owned(),
                                npm: "compat".to_owned(),
                            },
                            name: model.model.name.clone(),
                            family: model.model.family.clone(),
                            capabilities: ModelCapabilitiesDoc {
                                temperature: model.model.temperature.unwrap_or(false),
                                reasoning: !model.model.reasoning_efforts.is_empty(),
                                attachment: model.model.attachment,
                                toolcall: model.model.tool_call,
                                input: ModelIoDoc {
                                    text: true,
                                    audio: false,
                                    image: false,
                                    video: false,
                                    pdf: false,
                                },
                                output: ModelIoDoc {
                                    text: true,
                                    audio: false,
                                    image: false,
                                    video: false,
                                    pdf: false,
                                },
                                interleaved: ModelInterleavedDoc::Bool(false),
                            },
                            cost: ModelCostDoc {
                                input: cost.as_ref().map(|cost| cost.input).unwrap_or_default(),
                                output: cost.as_ref().map(|cost| cost.output).unwrap_or_default(),
                                cache: MessageTokensCacheDoc {
                                    read: cost
                                        .as_ref()
                                        .and_then(|cost| cost.cache_read)
                                        .unwrap_or_default(),
                                    write: cost
                                        .as_ref()
                                        .and_then(|cost| cost.cache_write)
                                        .unwrap_or_default(),
                                },
                                experimental_over_200k: None,
                            },
                            limit: ModelLimitDoc {
                                context: limit
                                    .as_ref()
                                    .map(|limit| limit.context as f64)
                                    .unwrap_or_default(),
                                input: limit
                                    .as_ref()
                                    .and_then(|limit| limit.input.map(|value| value as f64)),
                                output: limit
                                    .as_ref()
                                    .map(|limit| limit.output as f64)
                                    .unwrap_or_default(),
                            },
                            status: ModelStatusDoc::Active,
                            options: BTreeMap::new(),
                            headers: BTreeMap::new(),
                            release_date: model.model.release_date.clone().unwrap_or_default(),
                            variants: None,
                        },
                    )
                })
                .collect::<BTreeMap<String, ModelDoc>>();
            ProviderDoc {
                id: name.clone(),
                name,
                source: ProviderSourceDoc::Api,
                env: Vec::new(),
                key: None,
                options: BTreeMap::new(),
                models,
            }
        })
        .collect()
}

fn provider_default_map(providers: &[ProviderDoc]) -> BTreeMap<String, String> {
    let mut defaults = BTreeMap::new();
    for provider in providers {
        if let Some(model_id) = provider.models.keys().next() {
            defaults.insert(provider.id.clone(), model_id.clone());
        }
    }
    defaults
}

fn collect_paths(base: &FsPath, results: &mut Vec<String>, query: &str, limit: usize) {
    if results.len() >= limit {
        return;
    }
    let Ok(read_dir) = fs::read_dir(base) else {
        return;
    };
    for entry in read_dir.flatten() {
        if results.len() >= limit {
            break;
        }
        let path = entry.path();
        let display = path.display().to_string();
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if query.is_empty() || name.contains(query) {
            results.push(display);
        }
        if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            collect_paths(&path, results, query, limit);
        }
    }
}

fn guess_mime(path: &FsPath) -> &'static str {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
    {
        "rs" | "ts" | "tsx" | "js" | "json" | "md" | "txt" | "toml" | "yaml" | "yml" => {
            "text/plain"
        }
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}

fn is_git_root(path: &FsPath) -> bool {
    path.join(".git").exists()
}

fn compat_error(code: impl Into<String>, message: impl Into<String>) -> Response {
    let code = code.into();
    let message = message.into();
    if code == "not_found" {
        return (
            StatusCode::NOT_FOUND,
            Json(NotFoundErrorDoc {
                name: NotFoundErrorNameDoc::NotFoundError,
                data: NotFoundDataDoc { message },
            }),
        )
            .into_response();
    }
    (
        StatusCode::BAD_REQUEST,
        Json(BadRequestErrorDoc {
            data: json!({ "code": code, "message": message }),
            errors: Vec::new(),
            success: false,
        }),
    )
        .into_response()
}

fn require_bearer_token(headers: &HeaderMap) -> Result<(), Response> {
    match headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .filter(|value| value.starts_with("Bearer "))
    {
        Some(_) => Ok(()),
        None => Err((
            StatusCode::UNAUTHORIZED,
            Json(BadRequestErrorDoc {
                data: json!({ "code": "unauthorized", "message": "missing bearer token" }),
                errors: Vec::new(),
                success: false,
            }),
        )
            .into_response()),
    }
}

fn invalid_request(code: &str) -> Response {
    compat_error(code, "invalid request")
}

fn json_object(value: &Value) -> BTreeMap<String, Value> {
    value
        .clone()
        .as_object()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .collect()
}

fn json_string(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "null".to_owned())
}
