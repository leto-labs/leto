use std::sync::Arc;

use chat::{
    ChatAdapter, ChatAdapterInfo, ChatCapabilities, ChatEvent, ChatEventStream, DeliveryReceipt,
    Error, MessageRef, OutboundEdit, OutboundMessage,
};
use futures::{StreamExt, future::BoxFuture};
use tokio::sync::{Mutex, broadcast};
use tokio_stream::wrappers::BroadcastStream;

use crate::{SlackConfig, SlackEvent};

/// Slack implementation of the shared [`chat::ChatAdapter`] trait.
pub struct SlackAdapter {
    config: SlackConfig,
    events: broadcast::Sender<ChatEvent>,
    sent_messages: Arc<Mutex<Vec<OutboundMessage>>>,
}

impl SlackAdapter {
    /// Creates a new Slack adapter from Slack-specific config.
    pub fn new(config: SlackConfig) -> Self {
        let (events, _) = broadcast::channel(32);
        Self {
            config,
            events,
            sent_messages: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Injects a normalized provider event into the adapter stream.
    pub fn push_event(&self, event: SlackEvent) {
        let _ = self.events.send(event.into());
    }
}

impl ChatAdapter for SlackAdapter {
    fn info(&self) -> ChatAdapterInfo {
        ChatAdapterInfo {
            id: "slack".into(),
            display_name: "Slack".into(),
            capabilities: ChatCapabilities {
                receive_messages: true,
                send_messages: true,
                message_edits: true,
                threaded_replies: true,
                typing_indicators: false,
                reactions: true,
                attachments: true,
                webhook_mode: true,
                polling_mode: false,
                health_checks: true,
            },
        }
    }

    fn subscribe(&self) -> Result<ChatEventStream, Error> {
        let stream = BroadcastStream::new(self.events.subscribe())
            .map(|result| result.map_err(|error| Error::Transport(error.to_string())));
        Ok(Box::pin(stream))
    }

    fn send<'a>(
        &'a self,
        message: OutboundMessage,
    ) -> BoxFuture<'a, Result<DeliveryReceipt, Error>> {
        let sent_messages = Arc::clone(&self.sent_messages);
        Box::pin(async move {
            let conversation = message.conversation.clone();
            sent_messages.lock().await.push(message);
            Ok(DeliveryReceipt::new(MessageRef::new(
                conversation,
                "slack-outbound-1",
            )))
        })
    }

    fn edit<'a>(&'a self, edit: OutboundEdit) -> BoxFuture<'a, Result<DeliveryReceipt, Error>> {
        Box::pin(async move { Ok(DeliveryReceipt::new(edit.message)) })
    }

    fn health_check<'a>(&'a self) -> BoxFuture<'a, Result<bool, Error>> {
        Box::pin(async move {
            Ok(self.config.bot_token.is_some() && self.config.signing_secret.is_some())
        })
    }
}
