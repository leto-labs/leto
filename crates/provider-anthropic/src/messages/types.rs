use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct MessageRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub max_tokens: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub messages: Vec<MessageParam>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<SystemPrompt>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ToolDefinition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_config: Option<MessageOutputConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stop_sequences: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(default, flatten, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, serde_json::Value>,
}

impl Default for MessageRequest {
    fn default() -> Self {
        Self {
            model: None,
            max_tokens: 1024,
            messages: Vec::new(),
            system: None,
            tools: Vec::new(),
            tool_choice: None,
            thinking: None,
            metadata: None,
            output_config: None,
            service_tier: None,
            stop_sequences: Vec::new(),
            stream: None,
            temperature: None,
            top_k: None,
            top_p: None,
            extra: BTreeMap::new(),
        }
    }
}

impl MessageRequest {
    pub fn user_text(text: impl Into<String>) -> Self {
        Self {
            messages: vec![MessageParam::user(text)],
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MessageParam {
    pub role: MessageRole,
    pub content: MessageContentParam,
}

impl MessageParam {
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: MessageContentParam::Text(text.into()),
        }
    }

    pub fn assistant(text: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: MessageContentParam::Text(text.into()),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum MessageContentParam {
    Text(String),
    Blocks(Vec<ContentBlockParam>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum SystemPrompt {
    Text(String),
    Blocks(Vec<TextBlockParam>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlockParam {
    Text {
        text: String,
    },
    Image {
        source: ImageSource,
    },
    Document {
        source: DocumentSource,
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        context: Option<String>,
    },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        caller: Option<serde_json::Value>,
    },
    ToolResult {
        tool_use_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<ToolResultContent>,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
    Thinking {
        thinking: String,
        signature: String,
    },
    RedactedThinking {
        data: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextBlockParam {
    pub text: String,
    #[serde(rename = "type")]
    pub block_type: String,
}

impl TextBlockParam {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            block_type: "text".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImageSource {
    Base64 { media_type: String, data: String },
    Url { url: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum DocumentSource {
    #[serde(rename = "base64")]
    Base64Pdf { media_type: String, data: String },
    #[serde(rename = "text")]
    PlainText { media_type: String, data: String },
    #[serde(rename = "url")]
    Url { url: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ToolResultContent {
    Text(String),
    Blocks(Vec<ToolResultContentBlockParam>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolResultContentBlockParam {
    Text {
        text: String,
    },
    Image {
        source: ImageSource,
    },
    Document {
        source: DocumentSource,
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ToolDefinition {
    Custom(CustomToolDefinition),
    Raw(serde_json::Value),
}

impl ToolDefinition {
    pub fn custom(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: serde_json::Value,
    ) -> Self {
        Self::Custom(CustomToolDefinition {
            name: name.into(),
            description: Some(description.into()),
            input_schema,
            tool_type: None,
            strict: None,
            extra: BTreeMap::new(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CustomToolDefinition {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub tool_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
    #[serde(default, flatten, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolChoice {
    Auto {
        #[serde(skip_serializing_if = "Option::is_none")]
        disable_parallel_tool_use: Option<bool>,
    },
    Any {
        #[serde(skip_serializing_if = "Option::is_none")]
        disable_parallel_tool_use: Option<bool>,
    },
    Tool {
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        disable_parallel_tool_use: Option<bool>,
    },
    None,
}

impl ToolChoice {
    pub fn tool(name: impl Into<String>) -> Self {
        Self::Tool {
            name: name.into(),
            disable_parallel_tool_use: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ThinkingConfig {
    Enabled {
        budget_tokens: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        display: Option<String>,
    },
    Adaptive {
        #[serde(skip_serializing_if = "Option::is_none")]
        display: Option<String>,
    },
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MessageOutputConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MessageObject {
    pub id: Option<String>,
    pub object: Option<String>,
    pub role: Option<String>,
    pub content: Vec<ContentBlock>,
    pub model: Option<String>,
    pub stop_reason: Option<StopReason>,
    pub stop_sequence: Option<String>,
    pub usage: Option<MessageUsage>,
    pub raw: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContentBlock {
    Text {
        text: String,
        citations: Option<serde_json::Value>,
    },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
        caller: Option<serde_json::Value>,
    },
    Thinking {
        thinking: String,
        signature: Option<String>,
    },
    RedactedThinking {
        data: String,
    },
    Unknown {
        block_type: String,
        raw: serde_json::Value,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    EndTurn,
    MaxTokens,
    StopSequence,
    ToolUse,
    PauseTurn,
    Refusal,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_creation_input_tokens: Option<u32>,
    pub cache_read_input_tokens: Option<u32>,
    pub inference_geo: Option<String>,
    pub service_tier: Option<String>,
    pub server_tool_use: Option<ServerToolUsage>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServerToolUsage {
    pub web_fetch_requests: u32,
    pub web_search_requests: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessageStreamEvent {
    MessageStart {
        message: MessageObject,
    },
    MessageDelta {
        delta: MessageDelta,
        usage: Option<MessageDeltaUsage>,
    },
    MessageStop,
    ContentBlockStart {
        index: u32,
        content_block: ContentBlock,
    },
    ContentBlockDelta {
        index: u32,
        delta: ContentBlockDelta,
    },
    ContentBlockStop {
        index: u32,
    },
    Ping,
    Error {
        error: MessageErrorInfo,
        raw: serde_json::Value,
    },
    Unknown {
        event_type: String,
        raw: serde_json::Value,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct MessageDelta {
    pub stop_reason: Option<StopReason>,
    pub stop_sequence: Option<String>,
    pub container: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageDeltaUsage {
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub cache_creation_input_tokens: Option<u32>,
    pub cache_read_input_tokens: Option<u32>,
    pub server_tool_use: Option<ServerToolUsage>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContentBlockDelta {
    TextDelta {
        text: String,
    },
    InputJsonDelta {
        partial_json: String,
    },
    ThinkingDelta {
        thinking: String,
    },
    SignatureDelta {
        signature: String,
    },
    Unknown {
        delta_type: String,
        raw: serde_json::Value,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct MessageErrorInfo {
    pub error_type: Option<String>,
    pub message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn serializes_text_and_image_message() {
        let request = MessageRequest {
            messages: vec![MessageParam {
                role: MessageRole::User,
                content: MessageContentParam::Blocks(vec![
                    ContentBlockParam::Image {
                        source: ImageSource::Url {
                            url: "https://example.com/cat.png".into(),
                        },
                    },
                    ContentBlockParam::Text {
                        text: "describe this".into(),
                    },
                ]),
            }],
            ..MessageRequest::default()
        };

        let serialized = serde_json::to_value(request).unwrap();
        let content = serialized["messages"][0]["content"].as_array().unwrap();
        assert_eq!(content.len(), 2);
        assert_eq!(content[0]["type"], "image");
        assert_eq!(content[1]["type"], "text");
    }

    #[test]
    fn serializes_tool_result_block() {
        let request = MessageRequest {
            messages: vec![MessageParam {
                role: MessageRole::User,
                content: MessageContentParam::Blocks(vec![ContentBlockParam::ToolResult {
                    tool_use_id: "toolu_123".into(),
                    content: Some(ToolResultContent::Text("hello".into())),
                    is_error: Some(false),
                }]),
            }],
            ..MessageRequest::default()
        };

        let serialized = serde_json::to_value(request).unwrap();
        assert_eq!(
            serialized["messages"][0]["content"][0]["type"],
            "tool_result"
        );
        assert_eq!(
            serialized["messages"][0]["content"][0]["tool_use_id"],
            "toolu_123"
        );
    }

    #[test]
    fn serializes_custom_tool_definition() {
        let tool = ToolDefinition::custom(
            "echo",
            "Echo text",
            json!({
                "type": "object",
                "properties": {
                    "text": { "type": "string" }
                },
                "required": ["text"]
            }),
        );

        let serialized = serde_json::to_value(tool).unwrap();
        assert_eq!(serialized["name"], "echo");
        assert_eq!(serialized["input_schema"]["type"], "object");
    }
}
