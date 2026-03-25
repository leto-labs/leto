use chrono::Utc;
use futures::StreamExt;

use chat::{ChatAdapter, ChatEvent, ConversationRef, OutboundMessage};
use chat_telegram::{TelegramAdapter, TelegramConfig, TelegramUpdate};

#[tokio::test]
async fn telegram_update_normalizes_into_shared_chat_event() {
    let adapter = TelegramAdapter::new(TelegramConfig {
        bot_token: Some("token".into()),
        webhook_secret: None,
    });
    let mut stream = adapter.subscribe().unwrap();

    adapter.push_update(TelegramUpdate {
        chat_id: 42,
        message_id: 7,
        sender_id: 99,
        sender_username: Some("alice".into()),
        text: "hello telegram".into(),
        thread_id: Some(3),
        edited: false,
        attachments: Vec::new(),
        sent_at: Utc::now(),
        reply_to_message_id: None,
    });

    let event = stream.next().await.unwrap().unwrap();
    let ChatEvent::MessageReceived { message } = event else {
        panic!("expected message-received event");
    };

    assert_eq!(message.message.conversation.adapter, "telegram");
    assert_eq!(message.message.conversation.conversation_id, "42");
    assert_eq!(message.message.conversation.thread_id.as_deref(), Some("3"));
    assert_eq!(message.message.message_id, "7");
    assert_eq!(message.sender.display_name.as_deref(), Some("alice"));
}

#[tokio::test]
async fn telegram_adapter_send_returns_shared_delivery_receipt() {
    let adapter = TelegramAdapter::new(TelegramConfig {
        bot_token: Some("token".into()),
        webhook_secret: None,
    });
    let receipt = adapter
        .send(OutboundMessage::new(
            ConversationRef::new("telegram", "42"),
            "outbound",
        ))
        .await
        .unwrap();

    assert_eq!(receipt.message.conversation.adapter, "telegram");
    assert_eq!(receipt.message.conversation.conversation_id, "42");
}
