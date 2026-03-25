use chrono::Utc;
use futures::StreamExt;

use chat::{ChatAdapter, ChatEvent, ConversationRef, OutboundMessage};
use chat_slack::{SlackAdapter, SlackConfig, SlackEvent, SlackFile};

#[tokio::test]
async fn slack_event_normalizes_thread_and_files() {
    let adapter = SlackAdapter::new(SlackConfig {
        bot_token: Some("token".into()),
        signing_secret: Some("secret".into()),
    });
    let mut stream = adapter.subscribe().unwrap();

    adapter.push_event(SlackEvent {
        channel_id: "C123".into(),
        ts: "1700.10".into(),
        user_id: "U123".into(),
        text: "hello slack".into(),
        username: Some("alice".into()),
        thread_ts: Some("1700.00".into()),
        edited: false,
        files: vec![SlackFile {
            id: "F123".into(),
            filename: Some("image.png".into()),
            content_type: Some("image/png".into()),
            url: None,
            size_bytes: Some(512),
        }],
        sent_at: Utc::now(),
    });

    let event = stream.next().await.unwrap().unwrap();
    let ChatEvent::MessageReceived { message } = event else {
        panic!("expected message-received event");
    };

    assert_eq!(message.message.conversation.adapter, "slack");
    assert_eq!(message.message.conversation.conversation_id, "C123");
    assert_eq!(
        message.message.conversation.thread_id.as_deref(),
        Some("1700.00")
    );
    assert_eq!(message.attachments.len(), 1);
}

#[tokio::test]
async fn slack_adapter_send_returns_shared_delivery_receipt() {
    let adapter = SlackAdapter::new(SlackConfig {
        bot_token: Some("token".into()),
        signing_secret: Some("secret".into()),
    });
    let receipt = adapter
        .send(OutboundMessage::new(
            ConversationRef::new("slack", "C123"),
            "outbound",
        ))
        .await
        .unwrap();

    assert_eq!(receipt.message.conversation.adapter, "slack");
    assert_eq!(receipt.message.conversation.conversation_id, "C123");
}
