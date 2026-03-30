use chrono::Utc;
use futures::StreamExt;

use chat::{
    ChatAdapter, ChatEvent, ConversationRef, Error, MessageRef, OutboundEdit, OutboundMessage,
};
use chat_teams::{TeamsActivity, TeamsAdapter, TeamsAttachment, TeamsConfig};

#[tokio::test]
async fn teams_activity_normalizes_reply_metadata() {
    let adapter = TeamsAdapter::new(TeamsConfig {
        bot_token: Some("token".into()),
        app_id: Some("app-id".into()),
    });
    let mut stream = adapter.subscribe().unwrap();

    adapter.push_activity(TeamsActivity {
        conversation_id: "conversation-1".into(),
        activity_id: "activity-1".into(),
        from_id: "user-1".into(),
        from_name: Some("Alice".into()),
        text: "hello teams".into(),
        reply_to_id: Some("root-1".into()),
        edited: false,
        attachments: vec![TeamsAttachment {
            id: Some("att-1".into()),
            content_type: Some("image/png".into()),
            content_url: None,
            name: Some("diagram.png".into()),
            size_bytes: Some(1024),
        }],
        sent_at: Utc::now(),
    });

    let event = stream.next().await.unwrap().unwrap();
    let ChatEvent::MessageReceived { message } = event else {
        panic!("expected message-received event");
    };

    assert_eq!(message.message.conversation.adapter, "teams");
    assert_eq!(message.reply_to.unwrap().message_id, "root-1");
    assert_eq!(message.attachments.len(), 1);
}

#[tokio::test]
async fn teams_edit_uses_shared_unsupported_default() {
    let adapter = TeamsAdapter::new(TeamsConfig {
        bot_token: Some("token".into()),
        app_id: Some("app-id".into()),
    });
    let error = adapter
        .edit(OutboundEdit::new(
            MessageRef::new(
                ConversationRef::new("teams", "conversation-1"),
                "activity-1",
            ),
            "updated",
        ))
        .await
        .unwrap_err();

    assert_eq!(
        error,
        Error::UnsupportedCapability {
            adapter: "teams".into(),
            capability: "message_edits",
        }
    );
}

#[tokio::test]
async fn teams_adapter_send_returns_shared_delivery_receipt() {
    let adapter = TeamsAdapter::new(TeamsConfig {
        bot_token: Some("token".into()),
        app_id: Some("app-id".into()),
    });
    let receipt = adapter
        .send(OutboundMessage::new(
            ConversationRef::new("teams", "conversation-1"),
            "outbound",
        ))
        .await
        .unwrap();

    assert_eq!(receipt.message.conversation.adapter, "teams");
    assert_eq!(
        receipt.message.conversation.conversation_id,
        "conversation-1"
    );
}

#[tokio::test]
async fn teams_edited_activity_normalizes_into_shared_edit() {
    let adapter = TeamsAdapter::new(TeamsConfig {
        bot_token: Some("token".into()),
        app_id: Some("app-id".into()),
    });
    let mut stream = adapter.subscribe().unwrap();

    adapter.push_activity(TeamsActivity {
        conversation_id: "conversation-1".into(),
        activity_id: "activity-2".into(),
        from_id: "user-1".into(),
        from_name: Some("Alice".into()),
        text: "updated teams".into(),
        reply_to_id: Some("root-1".into()),
        edited: true,
        attachments: vec![TeamsAttachment {
            id: Some("att-2".into()),
            content_type: Some("text/plain".into()),
            content_url: Some("https://example.test/notes.txt".into()),
            name: Some("notes.txt".into()),
            size_bytes: Some(128),
        }],
        sent_at: Utc::now(),
    });

    let event = stream.next().await.unwrap().unwrap();
    let ChatEvent::MessageEdited { message } = event else {
        panic!("expected message-edited event");
    };

    assert_eq!(message.message.conversation.adapter, "teams");
    assert_eq!(
        message.message.conversation.conversation_id,
        "conversation-1"
    );
    assert_eq!(message.message.message_id, "activity-2");
    assert_eq!(message.sender.display_name.as_deref(), Some("Alice"));
    assert_eq!(message.reply_to.unwrap().message_id, "root-1");
    assert_eq!(message.attachments.len(), 1);
    assert_eq!(message.attachments[0].id.as_deref(), Some("att-2"));
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
