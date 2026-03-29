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

#[tokio::test]
async fn slack_edited_event_normalizes_into_shared_edit() {
    let adapter = SlackAdapter::new(SlackConfig {
        bot_token: Some("token".into()),
        signing_secret: Some("secret".into()),
    });
    let mut stream = adapter.subscribe().unwrap();

    adapter.push_event(SlackEvent {
        channel_id: "C123".into(),
        ts: "1700.11".into(),
        user_id: "U123".into(),
        text: "edited slack".into(),
        username: Some("alice".into()),
        thread_ts: Some("1700.00".into()),
        edited: true,
        files: vec![SlackFile {
            id: "F124".into(),
            filename: Some("notes.txt".into()),
            content_type: Some("text/plain".into()),
            url: Some("https://example.test/notes.txt".into()),
            size_bytes: Some(128),
        }],
        sent_at: Utc::now(),
    });

    let event = stream.next().await.unwrap().unwrap();
    let ChatEvent::MessageEdited { message } = event else {
        panic!("expected message-edited event");
    };

    assert_eq!(message.message.conversation.adapter, "slack");
    assert_eq!(message.message.conversation.conversation_id, "C123");
    assert_eq!(
        message.message.conversation.thread_id.as_deref(),
        Some("1700.00")
    );
    assert_eq!(message.message.message_id, "1700.11");
    assert_eq!(message.sender.display_name.as_deref(), Some("alice"));
    assert_eq!(message.attachments.len(), 1);
    assert_eq!(message.attachments[0].id.as_deref(), Some("F124"));
    assert_eq!(
        message.attachments[0].filename.as_deref(),
        Some("notes.txt")
    );
    assert_eq!(
        message.attachments[0].content_type.as_deref(),
        Some("text/plain")
    );
    assert_eq!(
        message.attachments[0].url.as_deref(),
        Some("https://example.test/notes.txt")
    );
    assert_eq!(message.attachments[0].size_bytes, Some(128));
}
