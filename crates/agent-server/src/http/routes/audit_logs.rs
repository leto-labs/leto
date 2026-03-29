use std::collections::BTreeMap;

use axum::Json;
use axum::extract::Query;
use axum::response::{IntoResponse, Response};
use provider_openai::{
    AuditActor, AuditLogEvent, AuditLogListParams, AuditLogPage, AuditProject, AuditSession,
    AuditUser,
};
use serde_json::json;

pub(in crate::http) async fn list_audit_logs(Query(params): Query<AuditLogListParams>) -> Response {
    let mut data = sample_audit_log_events();
    let limit = params.limit.unwrap_or(data.len() as u32) as usize;
    let has_more = limit < data.len();
    data.truncate(limit);

    Json(AuditLogPage {
        object: "list".to_owned(),
        data,
        has_more,
    })
    .into_response()
}

fn sample_audit_log_events() -> Vec<AuditLogEvent> {
    vec![
        AuditLogEvent {
            id: "req_agent_server_20240301".to_owned(),
            event_type: "api_key.created".to_owned(),
            effective_at: 1_720_804_090_i64,
            actor: AuditActor {
                actor_type: "session".to_owned(),
                session: Some(AuditSession {
                    user: AuditUser {
                        id: "user-agent-server".to_owned(),
                        email: "agent@example.com".to_owned(),
                    },
                    ip_address: Some("127.0.0.1".to_owned()),
                    user_agent: Some("agent-server-test".to_owned()),
                }),
            },
            project: Some(AuditProject {
                id: "proj_agent_server".to_owned(),
                name: "Default Project".to_owned(),
            }),
            extra: BTreeMap::from([(
                "api_key.created".to_owned(),
                json!({ "id": "key_agent_server" }),
            )]),
        },
        AuditLogEvent {
            id: "req_agent_server_20240302".to_owned(),
            event_type: "project.created".to_owned(),
            effective_at: 1_720_890_490_i64,
            actor: AuditActor {
                actor_type: "session".to_owned(),
                session: Some(AuditSession {
                    user: AuditUser {
                        id: "user-agent-server".to_owned(),
                        email: "agent@example.com".to_owned(),
                    },
                    ip_address: Some("127.0.0.1".to_owned()),
                    user_agent: Some("agent-server-test".to_owned()),
                }),
            },
            project: Some(AuditProject {
                id: "proj_agent_server".to_owned(),
                name: "Default Project".to_owned(),
            }),
            extra: BTreeMap::from([(
                "project.created".to_owned(),
                json!({ "id": "proj_agent_server" }),
            )]),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::sample_audit_log_events;

    #[test]
    fn sample_audit_log_events_include_flattened_metadata() {
        let events = sample_audit_log_events();

        assert_eq!(events.len(), 2);
        assert!(events[0].extra.contains_key("api_key.created"));
        assert!(events[1].extra.contains_key("project.created"));
    }
}
