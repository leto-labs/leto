use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};

use crate::{
    ChatCapabilities, ChatEventStream, DeliveryReceipt, Error, MessageRef, OutboundEdit,
    OutboundMessage,
};

/// Shared metadata published by a concrete chat adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatAdapterInfo {
    /// Stable adapter identifier such as `telegram`.
    pub id: String,
    /// Human-readable display name.
    pub display_name: String,
    /// Capability metadata for the concrete adapter.
    pub capabilities: ChatCapabilities,
}

/// Shared trait implemented by concrete chat providers.
pub trait ChatAdapter: Send + Sync {
    /// Returns static adapter metadata and capability information.
    fn info(&self) -> ChatAdapterInfo;

    /// Subscribes to normalized inbound chat events.
    fn subscribe(&self) -> Result<ChatEventStream, Error>;

    /// Sends a normalized outbound message.
    fn send<'a>(
        &'a self,
        message: OutboundMessage,
    ) -> BoxFuture<'a, Result<DeliveryReceipt, Error>>;

    /// Edits a previously sent message when supported.
    fn edit<'a>(&'a self, _edit: OutboundEdit) -> BoxFuture<'a, Result<DeliveryReceipt, Error>> {
        let adapter = self.info().id;
        Box::pin(async move {
            Err(Error::UnsupportedCapability {
                adapter,
                capability: "message_edits",
            })
        })
    }

    /// Deletes a previously sent message when supported.
    fn delete<'a>(&'a self, _message: MessageRef) -> BoxFuture<'a, Result<(), Error>> {
        let adapter = self.info().id;
        Box::pin(async move {
            Err(Error::UnsupportedCapability {
                adapter,
                capability: "message_deletes",
            })
        })
    }

    /// Checks whether the adapter is configured and operational enough to serve traffic.
    fn health_check<'a>(&'a self) -> BoxFuture<'a, Result<bool, Error>>;
}
