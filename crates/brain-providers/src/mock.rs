use std::time::Duration;

use async_stream::stream;
use futures::future::BoxFuture;

use brain_types::*;

pub struct MockProvider {
    pub delay_ms: u64,
}

impl MockProvider {
    pub fn new() -> Self {
        Self { delay_ms: 0 }
    }

    pub fn with_delay(mut self, ms: u64) -> Self {
        self.delay_ms = ms;
        self
    }
}

impl Default for MockProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl Provider for MockProvider {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "mock".into(),
            default_model: Some("mock-echo".into()),
            models: vec![ProviderModelInfo {
                id: "mock-echo".into(),
                name: "Mock Echo".into(),
                provider: None,
                reasoning: false,
                tool_call: true,
            }],
        }
    }

    fn chat<'a>(
        &'a self,
        messages: &'a [Message],
        tools: &'a [ToolDef],
        _config: &'a InferenceConfig,
        _session_id: Option<ulid::Ulid>,
    ) -> BoxFuture<'a, Result<ChatStream, BrainError>> {
        Box::pin(async move {
            let last_user = messages
                .iter()
                .rev()
                .find(|m| m.role == Role::User)
                .map(|m| m.content.clone())
                .unwrap_or_default();

            let delay = self.delay_ms;
            let has_tools = !tools.is_empty();
            let first_tool_name = tools.first().map(|t| t.name.clone());

            let s = stream! {
                if last_user.starts_with("tool:") && has_tools {
                    let tool_name = first_tool_name.unwrap_or_else(|| "unknown".into());
                    let arg_text = last_user.strip_prefix("tool:").unwrap_or("").trim().to_string();
                    yield Ok(ChatChunk::ToolCall {
                        id: "mock-call-1".into(),
                        name: tool_name,
                        arguments: serde_json::json!({ "message": arg_text }),
                    });
                } else {
                    for word in last_user.split_whitespace() {
                        if delay > 0 {
                            tokio::time::sleep(Duration::from_millis(delay)).await;
                        }
                        yield Ok(ChatChunk::Delta { content: format!("{word} ") });
                    }
                }

                let approx_tokens = (last_user.len() as u32) / 4;
                yield Ok(ChatChunk::Done {
                    usage: Some(TokenUsage {
                        prompt: approx_tokens,
                        completion: approx_tokens,
                        total: approx_tokens * 2,
                    }),
                });
            };

            Ok(Box::pin(s) as ChatStream)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    fn echo_tool_def() -> ToolDef {
        ToolDef {
            name: "echo".into(),
            description: "echoes".into(),
            parameters: serde_json::json!({}),
        }
    }

    #[tokio::test]
    async fn echo_returns_tokens() {
        let provider = MockProvider::new();
        let msgs = [Message::user("hello world")];
        let config = InferenceConfig::default();

        let mut stream = provider.chat(&msgs, &[], &config, None).await.unwrap();
        let mut text = String::new();
        while let Some(chunk) = stream.next().await {
            match chunk.unwrap() {
                ChatChunk::Delta { content } => text.push_str(&content),
                ChatChunk::Done { usage } => {
                    assert!(usage.is_some());
                }
                _ => {}
            }
        }
        assert_eq!(text.trim(), "hello world");
    }

    #[tokio::test]
    async fn tool_prefix_triggers_tool_call() {
        let provider = MockProvider::new();
        let msgs = [Message::user("tool: do something")];
        let tools = [echo_tool_def()];
        let config = InferenceConfig::default();

        let mut stream = provider.chat(&msgs, &tools, &config, None).await.unwrap();
        let mut saw_tool_call = false;
        while let Some(chunk) = stream.next().await {
            if let ChatChunk::ToolCall {
                id,
                name,
                arguments,
            } = chunk.unwrap()
            {
                assert_eq!(id, "mock-call-1");
                assert_eq!(name, "echo");
                assert_eq!(arguments["message"], "do something");
                saw_tool_call = true;
            }
        }
        assert!(saw_tool_call);
    }

    #[tokio::test]
    async fn tool_prefix_without_tools_echoes_normally() {
        let provider = MockProvider::new();
        let msgs = [Message::user("tool: do something")];
        let config = InferenceConfig::default();

        let mut stream = provider.chat(&msgs, &[], &config, None).await.unwrap();
        let mut text = String::new();
        while let Some(chunk) = stream.next().await {
            if let ChatChunk::Delta { content } = chunk.unwrap() {
                text.push_str(&content);
            }
        }
        assert!(text.contains("tool:"));
    }

    #[tokio::test]
    async fn empty_messages_returns_done() {
        let provider = MockProvider::new();
        let config = InferenceConfig::default();

        let mut stream = provider.chat(&[], &[], &config, None).await.unwrap();
        let mut saw_done = false;
        while let Some(chunk) = stream.next().await {
            if matches!(chunk.unwrap(), ChatChunk::Done { .. }) {
                saw_done = true;
            }
        }
        assert!(saw_done);
    }

    #[tokio::test]
    async fn with_delay_produces_same_output() {
        let provider = MockProvider::new().with_delay(1);
        let msgs = [Message::user("a b")];
        let config = InferenceConfig::default();

        let mut stream = provider.chat(&msgs, &[], &config, None).await.unwrap();
        let mut text = String::new();
        while let Some(chunk) = stream.next().await {
            if let ChatChunk::Delta { content } = chunk.unwrap() {
                text.push_str(&content);
            }
        }
        assert_eq!(text.trim(), "a b");
    }

    #[tokio::test]
    async fn default_impl() {
        let provider = MockProvider::default();
        assert_eq!(provider.delay_ms, 0);
    }
}
