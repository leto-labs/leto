use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::tool::ToolCall;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Ulid,
    pub role: Role,
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    pub tool_call_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Message {
    pub fn system(text: impl Into<String>) -> Self {
        Self::new(Role::System, text)
    }

    pub fn user(text: impl Into<String>) -> Self {
        Self::new(Role::User, text)
    }

    pub fn assistant(text: impl Into<String>) -> Self {
        Self::new(Role::Assistant, text)
    }

    pub fn tool_result(call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: Ulid::new(),
            role: Role::Tool,
            content: content.into(),
            tool_calls: vec![],
            tool_call_id: Some(call_id.into()),
            created_at: Utc::now(),
        }
    }

    fn new(role: Role, text: impl Into<String>) -> Self {
        Self {
            id: Ulid::new(),
            role,
            content: text.into(),
            tool_calls: vec![],
            tool_call_id: None,
            created_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_message() {
        let m = Message::system("Be helpful");
        assert_eq!(m.role, Role::System);
        assert_eq!(m.content, "Be helpful");
        assert!(m.tool_calls.is_empty());
        assert!(m.tool_call_id.is_none());
    }

    #[test]
    fn user_message() {
        let m = Message::user("hello");
        assert_eq!(m.role, Role::User);
        assert_eq!(m.content, "hello");
    }

    #[test]
    fn assistant_message() {
        let m = Message::assistant("hi there");
        assert_eq!(m.role, Role::Assistant);
        assert_eq!(m.content, "hi there");
    }

    #[test]
    fn tool_result_message() {
        let m = Message::tool_result("call-1", "result data");
        assert_eq!(m.role, Role::Tool);
        assert_eq!(m.content, "result data");
        assert_eq!(m.tool_call_id.as_deref(), Some("call-1"));
    }

    #[test]
    fn role_serde_roundtrip() {
        for role in [Role::System, Role::User, Role::Assistant, Role::Tool] {
            let json = serde_json::to_string(&role).unwrap();
            let deserialized: Role = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, role);
        }
    }

    #[test]
    fn role_serializes_lowercase() {
        assert_eq!(serde_json::to_string(&Role::System).unwrap(), r#""system""#);
        assert_eq!(serde_json::to_string(&Role::User).unwrap(), r#""user""#);
        assert_eq!(
            serde_json::to_string(&Role::Assistant).unwrap(),
            r#""assistant""#
        );
        assert_eq!(serde_json::to_string(&Role::Tool).unwrap(), r#""tool""#);
    }

    #[test]
    fn message_serde_roundtrip() {
        let m = Message::user("hello world");
        let json = serde_json::to_string(&m).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, m.id);
        assert_eq!(deserialized.role, Role::User);
        assert_eq!(deserialized.content, "hello world");
    }
}
