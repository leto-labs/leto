//! Typed response models for the Audit Logs API.
//!
//! Official reference:
//! <https://platform.openai.com/docs/api-reference/audit-logs>

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Optional pagination parameters for `GET /organization/audit_logs`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditLogListParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

/// Paginated response returned by `GET /organization/audit_logs`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditLogPage {
    pub object: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub data: Vec<AuditLogEvent>,
    pub has_more: bool,
}

/// Single organization audit-log event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditLogEvent {
    pub id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub effective_at: i64,
    pub actor: AuditActor,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<AuditProject>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

/// Actor who performed the audit-logged action.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditActor {
    #[serde(rename = "type")]
    pub actor_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<AuditSession>,
}

/// Session actor details.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditSession {
    pub user: AuditUser,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
}

/// User reference inside audit-log payloads.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditUser {
    pub id: String,
    pub email: String,
}

/// Project reference inside audit-log payloads.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditProject {
    pub id: String,
    pub name: String,
}
