use futures::{StreamExt, future::BoxFuture, stream};

use chat::{
    ChatAdapter, ChatAdapterInfo, ChatCapabilities, ChatEvent, ChatEventStream, ConversationRef,
    DeliveryReceipt, Error, InboundMessage, MessageRef, OutboundEdit, OutboundMessage, Participant,
};

struct DummyAdapter;

impl ChatAdapter for DummyAdapter {
    fn info(&self) -> ChatAdapterInfo {
        ChatAdapterInfo {
            id: "dummy".into(),
            display_name: "Dummy".into(),
            capabilities: ChatCapabilities {
                receive_messages: true,
                send_messages: true,
                health_checks: true,
                ..ChatCapabilities::default()
            },
        }
    }

    fn subscribe(&self) -> Result<ChatEventStream, Error> {
        let conversation = ConversationRef::new("dummy", "conv-1");
        let message = InboundMessage::new(
            MessageRef::new(conversation, "msg-1"),
            Participant::new("user-1"),
            "hello",
        );
        Ok(Box::pin(stream::iter([Ok(ChatEvent::MessageReceived {
            message,
        })])))
    }

    fn send<'a>(
        &'a self,
        message: OutboundMessage,
    ) -> BoxFuture<'a, Result<DeliveryReceipt, Error>> {
        Box::pin(async move {
            Ok(DeliveryReceipt::new(MessageRef::new(
                message.conversation,
                "out-1",
            )))
        })
    }

    fn health_check<'a>(&'a self) -> BoxFuture<'a, Result<bool, Error>> {
        Box::pin(async move { Ok(true) })
    }
}

#[tokio::test]
async fn default_edit_returns_explicit_unsupported_error() {
    let adapter = DummyAdapter;
    let edit = OutboundEdit::new(
        MessageRef::new(ConversationRef::new("dummy", "conv-1"), "msg-1"),
        "updated",
    );

    let error = adapter.edit(edit).await.unwrap_err();
    assert_eq!(
        error,
        Error::UnsupportedCapability {
            adapter: "dummy".into(),
            capability: "message_edits",
        }
    );
}

#[tokio::test]
async fn adapter_subscribe_returns_normalized_chat_event() {
    let adapter = DummyAdapter;
    let mut stream = adapter.subscribe().unwrap();

    let event = stream.next().await.unwrap().unwrap();
    let ChatEvent::MessageReceived { message } = event else {
        panic!("expected message-received event");
    };

    assert_eq!(message.message.message_id, "msg-1");
    assert_eq!(message.sender.id, "user-1");
    assert_eq!(message.text, "hello");
}
