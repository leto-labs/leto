use futures::StreamExt;
use provider_anthropic::{
    Client, Config, ContentBlock, CustomToolDefinition, Error, MessageParam, MessageRequest,
    StopReason, ToolChoice, ToolDefinition,
};
use serde_json::json;

async fn collect_stream_events(
    mut stream: provider_anthropic::MessageStream,
) -> Result<Vec<provider_anthropic::MessageStreamEvent>, Error> {
    let mut events = Vec::new();
    while let Some(event) = stream.next().await {
        events.push(event?);
    }
    Ok(events)
}

fn anthropic_client() -> Option<Client> {
    dotenvy::dotenv().ok();
    std::env::var("ANTHROPIC_API_KEY")
        .ok()
        .map(|api_key| Client::new(Config::new(api_key).with_model("claude-haiku-4-5")))
}

fn is_skippable_live_error(err: &Error) -> bool {
    let message = err.to_string();
    message.contains("401")
        || message.contains("403")
        || message.contains("404")
        || message.contains("429")
        || message.contains("529")
        || message.contains("rate limit")
        || message.contains("quota")
        || message.contains("overloaded")
}

#[tokio::test]
async fn create_message_non_streaming() {
    let Some(client) = anthropic_client() else {
        eprintln!("  SKIP provider-anthropic create_message: ANTHROPIC_API_KEY not set");
        return;
    };

    let response = match client
        .messages()
        .create(&MessageRequest::user_text(
            "Say just the word 'hello' and nothing else.",
        ))
        .await
    {
        Ok(response) => response,
        Err(err) if is_skippable_live_error(&err) => {
            eprintln!("  SKIP provider-anthropic create_message auth/billing: {err}");
            return;
        }
        Err(err) => panic!("provider-anthropic create_message failed: {err}"),
    };

    assert!(response.id.is_some());
    assert!(!response.content.is_empty());
}

#[tokio::test]
async fn streams_messages_over_sse() {
    let Some(client) = anthropic_client() else {
        eprintln!("  SKIP provider-anthropic stream: ANTHROPIC_API_KEY not set");
        return;
    };

    let stream = match client
        .messages()
        .stream(&MessageRequest::user_text(
            "Say just the word 'hello' and nothing else.",
        ))
        .await
    {
        Ok(stream) => stream,
        Err(err) if is_skippable_live_error(&err) => {
            eprintln!("  SKIP provider-anthropic stream auth/billing: {err}");
            return;
        }
        Err(err) => panic!("provider-anthropic stream failed to start: {err}"),
    };

    let events = collect_stream_events(stream).await.unwrap();
    assert!(events.iter().any(|event| matches!(
        event,
        provider_anthropic::MessageStreamEvent::MessageStart { .. }
    )));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, provider_anthropic::MessageStreamEvent::MessageStop))
    );
}

#[tokio::test]
async fn supports_client_tool_use() {
    let Some(client) = anthropic_client() else {
        eprintln!("  SKIP provider-anthropic tool use: ANTHROPIC_API_KEY not set");
        return;
    };

    let request = MessageRequest {
        messages: vec![MessageParam::user(
            "Use the echo tool and pass the text hello. Do not answer normally.",
        )],
        tools: vec![ToolDefinition::Custom(CustomToolDefinition {
            name: "echo".into(),
            description: Some("Echo the provided text.".into()),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "text": { "type": "string" }
                },
                "required": ["text"],
                "additionalProperties": false
            }),
            tool_type: None,
            strict: Some(true),
            extra: Default::default(),
        })],
        tool_choice: Some(ToolChoice::tool("echo")),
        ..MessageRequest::default()
    };

    let response = match client.messages().create(&request).await {
        Ok(response) => response,
        Err(err) if is_skippable_live_error(&err) => {
            eprintln!("  SKIP provider-anthropic tool use auth/billing: {err}");
            return;
        }
        Err(err) => panic!("provider-anthropic tool use failed: {err}"),
    };

    assert_eq!(response.stop_reason, Some(StopReason::ToolUse));
    assert!(
        response
            .content
            .iter()
            .any(|block| matches!(block, ContentBlock::ToolUse { .. }))
    );
}
