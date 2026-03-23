use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use ulid::Ulid;

use crate::tool::ToolCall;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

impl MessageContent {
    pub fn text(value: impl Into<String>) -> Self {
        Self::Text(value.into())
    }

    pub fn parts(parts: Vec<ContentPart>) -> Self {
        Self::Parts(parts)
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            Self::Parts(_) => None,
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Self::Text(text) => text.is_empty(),
            Self::Parts(parts) => parts.is_empty(),
        }
    }

    pub fn as_parts(&self) -> Option<&[ContentPart]> {
        match self {
            Self::Text(_) => None,
            Self::Parts(parts) => Some(parts),
        }
    }

    pub fn to_plain_text_lossy(&self) -> String {
        match self {
            Self::Text(text) => text.clone(),
            Self::Parts(parts) => parts
                .iter()
                .filter_map(|part| match part {
                    ContentPart::Text { text } => Some(text.as_str()),
                    ContentPart::ImageUrl { .. } => None,
                })
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }
}

impl Default for MessageContent {
    fn default() -> Self {
        Self::Text(String::new())
    }
}

impl fmt::Display for MessageContent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_plain_text_lossy())
    }
}

impl From<String> for MessageContent {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<&str> for MessageContent {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text { text: String },
    ImageUrl { url: String },
}

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
    pub content: MessageContent,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
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

    pub fn user_parts(parts: Vec<ContentPart>) -> Self {
        Self::new_content(Role::User, MessageContent::Parts(parts))
    }

    pub fn assistant_parts(parts: Vec<ContentPart>) -> Self {
        Self::new_content(Role::Assistant, MessageContent::Parts(parts))
    }

    pub fn tool_result(call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: Ulid::new(),
            role: Role::Tool,
            content: content.into().into(),
            reasoning_content: None,
            tool_calls: vec![],
            tool_call_id: Some(call_id.into()),
            created_at: Utc::now(),
        }
    }

    pub fn content_text(&self) -> Option<&str> {
        self.content.as_text()
    }

    pub fn content_text_lossy(&self) -> String {
        self.content.to_plain_text_lossy()
    }

    pub fn set_text_content(&mut self, text: impl Into<String>) {
        self.content = MessageContent::Text(text.into());
    }

    fn new(role: Role, text: impl Into<String>) -> Self {
        Self::new_content(role, MessageContent::Text(text.into()))
    }

    fn new_content(role: Role, content: MessageContent) -> Self {
        Self {
            id: Ulid::new(),
            role,
            content,
            reasoning_content: None,
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
        assert_eq!(m.content, MessageContent::text("Be helpful"));
        assert!(m.reasoning_content.is_none());
        assert!(m.tool_calls.is_empty());
        assert!(m.tool_call_id.is_none());
    }

    #[test]
    fn user_message() {
        let m = Message::user("hello");
        assert_eq!(m.role, Role::User);
        assert_eq!(m.content, MessageContent::text("hello"));
    }

    #[test]
    fn assistant_message() {
        let m = Message::assistant("hi there");
        assert_eq!(m.role, Role::Assistant);
        assert_eq!(m.content, MessageContent::text("hi there"));
    }

    #[test]
    fn tool_result_message() {
        let m = Message::tool_result("call-1", "result data");
        assert_eq!(m.role, Role::Tool);
        assert_eq!(m.content, MessageContent::text("result data"));
        assert_eq!(m.tool_call_id.as_deref(), Some("call-1"));
    }

    #[test]
    fn user_multimodal_message() {
        let m = Message::user_parts(vec![
            ContentPart::Text {
                text: "describe".into(),
            },
            ContentPart::ImageUrl {
                url: "data:image/png;base64,abc".into(),
            },
        ]);
        assert_eq!(m.role, Role::User);
        assert!(matches!(m.content, MessageContent::Parts(_)));
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
        assert_eq!(deserialized.content, MessageContent::text("hello world"));
        assert!(deserialized.reasoning_content.is_none());
    }
}
