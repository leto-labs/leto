//! Integration tests for the shared mock provider.

use futures::StreamExt;
use provider::{Event, MockProvider, Provider, Request, ToolDefinition};

#[tokio::test]
async fn mock_provider_streams_text_blocks() {
    let provider = MockProvider::new();
    let request = Request::user_text("hello world");

    let mut stream = provider.stream(&request).await.unwrap();
    let mut saw_start = false;
    let mut saw_stop = false;
    let mut saw_completed = false;
    let mut text = String::new();

    while let Some(event) = stream.next().await {
        match event.unwrap() {
            Event::BlockStart { .. } => saw_start = true,
            Event::BlockDelta { delta, .. } => {
                if let provider::BlockDelta::Text { text: delta } = delta {
                    text.push_str(&delta);
                }
            }
            Event::BlockStop { .. } => saw_stop = true,
            Event::Completed { .. } => saw_completed = true,
            Event::ResponseStart { .. } | Event::Usage { .. } => {}
        }
    }

    assert!(saw_start);
    assert!(saw_stop);
    assert!(saw_completed);
    assert_eq!(text.trim(), "hello world");
}

#[tokio::test]
async fn mock_provider_can_emit_tool_calls() {
    let provider = MockProvider::new();
    let request = Request {
        messages: vec![provider::Message::user_text("tool: ping")],
        tools: vec![ToolDefinition::new(
            "echo",
            "Echo text",
            serde_json::json!({ "type": "object" }),
        )],
        ..Request::default()
    };

    let mut stream = provider.stream(&request).await.unwrap();
    let mut saw_tool_call = false;

    while let Some(event) = stream.next().await {
        if let Event::BlockStart { block } = event.unwrap() {
            if matches!(block.kind, provider::BlockKind::ToolCall { .. }) {
                saw_tool_call = true;
            }
        }
    }

    assert!(saw_tool_call);
}

#[test]
fn shared_request_preserves_tool_result_blocks() {
    let request = Request {
        messages: vec![provider::Message::new(
            provider::MessageRole::User,
            vec![provider::ContentBlock::tool_result(
                "call-1",
                serde_json::json!({ "ok": true }),
            )],
        )],
        ..Request::default()
    };

    let value = serde_json::to_value(&request).unwrap();
    assert_eq!(value["messages"][0]["content"][0]["type"], "tool_result");
    assert_eq!(value["messages"][0]["content"][0]["call_id"], "call-1");
}

#[test]
fn provider_info_resolves_default_model_from_catalog() {
    let info = MockProvider::new().info();

    assert_eq!(info.default_model_id.as_deref(), Some("mock-echo"));
    assert_eq!(
        info.default_model().map(|model| model.id.as_ref()),
        Some("mock-echo")
    );
    assert_eq!(
        info.default_model().map(|model| model.name.as_ref()),
        Some("Mock Echo")
    );
}
