mod auth {
    use super::shared::*;

    include!("auth.rs");
}
mod convert {
    use super::auth::compat_error;
    use super::helpers::{is_git_root, json_object, json_string, query_directory};
    use super::ids::{compat_message_id, compat_part_id, compat_session_id};
    use super::shared::*;

    include!("convert.rs");
}
mod helpers {
    use super::shared::*;

    include!("helpers.rs");
}
mod ids {
    use super::auth::CompatRequestError;
    use super::shared::*;

    include!("ids.rs");
}
mod prompts {
    use super::auth::compat_error;
    use super::convert::{compat_assistant_message_with_parts, compat_message, compat_parts};
    use super::ids::{
        compat_message_id, compat_session_id, parse_compat_message_id, parse_compat_part_index,
        parse_compat_session_id,
    };
    use super::shared::*;

    include!("prompts.rs");
}
mod routes;
pub(crate) mod state;
#[cfg(test)]
pub(crate) mod test_utils;
pub(crate) mod types;

use std::sync::Arc;

use axum::Router;

use crate::server::AgentServer;

type AppState = Arc<AgentServer>;

mod shared {
    pub(super) use std::collections::BTreeMap;
    pub(super) use std::convert::Infallible;
    pub(super) use std::fs;
    pub(super) use std::path::{Path as FsPath, PathBuf};
    pub(super) use std::process::Command;
    pub(super) use std::sync::Arc;
    pub(super) use std::time::Duration;

    pub(super) use agent_store::{
        CredentialEntry, OAuthCredentials, Project, ProviderCredential, Session, SessionId,
        SessionUpdate, StoredMessage,
    };
    pub(super) use axum::extract::{Path, Query, State};
    pub(super) use axum::http::header::AUTHORIZATION;
    pub(super) use axum::http::{HeaderMap, StatusCode};
    pub(super) use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
    pub(super) use axum::response::{IntoResponse, Json, Response};
    pub(super) use chrono::{DateTime, Utc};
    pub(super) use futures::StreamExt;
    pub(super) use futures::stream;
    pub(super) use provider::{ContentBlock, Message, MessageRole};
    pub(super) use serde_json::{Value, json};
    pub(super) use ulid::Ulid;

    pub(super) use super::AppState;
    pub(super) use super::state::{
        CompatPty, CompatSessionMeta, default_directory, default_directory_string,
        mcp_connected_status, mcp_disabled_status,
    };
    pub(super) use super::types::common::{CompatQuery, ExperimentalSessionListQueryDoc};
    pub(super) use super::types::errors::{
        BadRequestErrorDoc, NotFoundDataDoc, NotFoundErrorDoc, NotFoundErrorNameDoc,
    };
    pub(super) use super::types::permission::PermissionRuleset;
    pub(super) use super::types::project::{
        ProjectCommandsDoc, ProjectDoc, ProjectSummaryDoc, ProjectTimeDoc, ProjectVcsDoc,
    };
    pub(super) use super::types::provider::{
        ApiAuthRequest, MessageTokensCacheDoc, ModelApiDoc, ModelCapabilitiesDoc, ModelCostDoc,
        ModelDoc, ModelInterleavedDoc, ModelIoDoc, ModelLimitDoc, ModelStatusDoc, OAuthAuthRequest,
        ProviderDoc, ProviderSourceDoc, WellKnownAuthRequest,
    };
    pub(super) use super::types::session::{
        AssistantMessageDoc, AssistantMessageRoleDoc, AssistantMessageWithPartsDoc, FilePartDoc,
        FilePartKindDoc, GlobalSessionDoc, MessageDoc, MessageModelDoc, MessagePathDoc,
        MessageTimeCreatedCompletedDoc, MessageTimeCreatedDoc, MessageTokensDoc,
        MessageWithPartsDoc, NullableProjectSummaryDoc, PartDoc, PromptPartInputDoc, SessionDoc,
        SessionShareDoc, SessionSummaryDoc, SessionTimeDoc, TextPartDoc, TextPartKindDoc,
        ToolPartDoc, ToolPartKindDoc, ToolStateCompletedDoc, ToolStateCompletedKindDoc,
        ToolStateCompletedTimeRangeDoc, ToolStateDoc, ToolStatePendingDoc, ToolStatePendingKindDoc,
        UserMessageDoc, UserMessageRoleDoc,
    };
    pub(super) use super::types::session::{CommandRequest, PromptRequest, ShellRequest};
    pub(super) use super::types::tui::{
        ExecuteCommandRequest, PromptAppendRequest, PublishRequest, SelectSessionRequest,
        ToastRequest,
    };
}

use auth::{compat_error, invalid_request, require_bearer_token, upsert_auth_credential};
use convert::{
    compat_global_session, compat_message_with_parts, compat_messages_with_parts, compat_project,
    compat_session, filtered_sessions, provider_array, provider_default_map,
    resolve_current_project, session_meta,
};
use helpers::{clear_compat_state, collect_paths, guess_mime, query_directory, tui_enqueue};
use ids::{compat_session_id, parse_compat_message_id, parse_compat_session_id};
use prompts::{
    PromptKind, mutate_message_part, prompt_command_like, prompt_input_from_body, prompt_like,
    prompt_shell_like,
};
use shared::*;

pub fn router() -> Router<AppState> {
    routes::build_router()
}
