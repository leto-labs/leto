use std::sync::Arc;

use chat::{
    ChatAdapter, ChatAdapterInfo, ChatCapabilities, ChatEvent, ChatEventStream, DeliveryReceipt,
    Error, MessageRef, OutboundMessage,
};
use futures::{StreamExt, future::BoxFuture};
use tokio::sync::{Mutex, broadcast};
use tokio_stream::wrappers::BroadcastStream;

use crate::{TeamsActivity, TeamsConfig};

/// Microsoft Teams implementation of the shared [`chat::ChatAdapter`] trait.
pub struct TeamsAdapter {
    config: TeamsConfig,
    events: broadcast::Sender<ChatEvent>,
    sent_messages: Arc<Mutex<Vec<OutboundMessage>>>,
}

impl TeamsAdapter {
    /// Creates a new Teams adapter from Teams-specific config.
    pub fn new(config: TeamsConfig) -> Self {
        let (events, _) = broadcast::channel(32);
        Self {
            config,
            events,
            sent_messages: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Injects a normalized provider activity into the adapter stream.
    pub fn push_activity(&self, activity: TeamsActivity) {
        let _ = self.events.send(activity.into());
    }
}

impl ChatAdapter for TeamsAdapter {
    fn info(&self) -> ChatAdapterInfo {
        ChatAdapterInfo {
            id: "teams".into(),
            display_name: "Microsoft Teams".into(),
            capabilities: ChatCapabilities {
                receive_messages: true,
                send_messages: true,
                message_edits: false,
                threaded_replies: true,
                typing_indicators: false,
                reactions: false,
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
                "teams-outbound-1",
            )))
        })
    }

    fn health_check<'a>(&'a self) -> BoxFuture<'a, Result<bool, Error>> {
        Box::pin(async move { Ok(self.config.bot_token.is_some() && self.config.app_id.is_some()) })
    }
}
