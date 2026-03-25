use std::sync::Arc;

use chat::{
    ChatAdapter, ChatAdapterInfo, ChatCapabilities, ChatEvent, ChatEventStream, DeliveryReceipt,
    Error, MessageRef, OutboundEdit, OutboundMessage,
};
use futures::{StreamExt, future::BoxFuture};
use tokio::sync::{Mutex, broadcast};
use tokio_stream::wrappers::BroadcastStream;

use crate::{TelegramConfig, TelegramUpdate};

/// Telegram implementation of the shared [`chat::ChatAdapter`] trait.
pub struct TelegramAdapter {
    config: TelegramConfig,
    events: broadcast::Sender<ChatEvent>,
    sent_messages: Arc<Mutex<Vec<OutboundMessage>>>,
}

impl TelegramAdapter {
    /// Creates a new Telegram adapter from Telegram-specific config.
    pub fn new(config: TelegramConfig) -> Self {
        let (events, _) = broadcast::channel(32);
        Self {
            config,
            events,
            sent_messages: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Injects a normalized provider update into the adapter stream.
    pub fn push_update(&self, update: TelegramUpdate) {
        let _ = self.events.send(update.into());
    }
}

impl ChatAdapter for TelegramAdapter {
    fn info(&self) -> ChatAdapterInfo {
        ChatAdapterInfo {
            id: "telegram".into(),
            display_name: "Telegram".into(),
            capabilities: ChatCapabilities {
                receive_messages: true,
                send_messages: true,
                message_edits: true,
                threaded_replies: true,
                typing_indicators: true,
                reactions: false,
                attachments: true,
                webhook_mode: true,
                polling_mode: true,
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
                "telegram-outbound-1",
            )))
        })
    }

    fn edit<'a>(&'a self, edit: OutboundEdit) -> BoxFuture<'a, Result<DeliveryReceipt, Error>> {
        Box::pin(async move { Ok(DeliveryReceipt::new(edit.message)) })
    }

    fn health_check<'a>(&'a self) -> BoxFuture<'a, Result<bool, Error>> {
        Box::pin(async move { Ok(self.config.bot_token.is_some()) })
    }
}
