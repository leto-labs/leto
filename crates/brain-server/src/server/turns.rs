use std::sync::Arc;

use futures::StreamExt;
use futures::future::BoxFuture;
use tokio_util::sync::CancellationToken;
use ulid::Ulid;

use brain_types::*;

use crate::types::ServerEvent;

use super::{BrainServer, Inner};

impl BrainServer {
    pub(super) fn send_message_api(
        &self,
        session_id: Ulid,
        content: &str,
    ) -> BoxFuture<'_, Result<(), BrainError>> {
        let content = content.to_owned();
        let inner = self.inner.clone();
        Box::pin(async move {
            let cancel = CancellationToken::new();

            {
                let mut turns = inner.active_turns.write().await;
                if turns.contains_key(&session_id) {
                    return Err(BrainError::TurnActive(session_id));
                }
                turns.insert(session_id, cancel.clone());
            }

            let stream = inner.brain.turn(session_id, &content, cancel);

            tokio::spawn(drain_turn(inner, session_id, stream));
            Ok(())
        })
    }

    pub(super) fn send_message_stream_api(
        &self,
        session_id: Ulid,
        content: &str,
    ) -> BoxFuture<'_, Result<EventStream, BrainError>> {
        let content = content.to_owned();
        let inner = self.inner.clone();
        Box::pin(async move {
            let cancel = CancellationToken::new();

            {
                let mut turns = inner.active_turns.write().await;
                if turns.contains_key(&session_id) {
                    return Err(BrainError::TurnActive(session_id));
                }
                turns.insert(session_id, cancel.clone());
            }

            let stream = inner.brain.turn(session_id, &content, cancel);

            let (tx, rx) = tokio::sync::mpsc::channel::<Event>(256);
            let inner_clone = inner.clone();
            tokio::spawn(async move {
                let mut stream = stream;
                while let Some(event) = stream.next().await {
                    inner_clone
                        .event_bus
                        .publish(ServerEvent::new(session_id, event.clone()));
                    if tx.send(event).await.is_err() {
                        break;
                    }
                }
                inner_clone.active_turns.write().await.remove(&session_id);
            });

            Ok(Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx)) as EventStream)
        })
    }

    pub(super) fn cancel_turn_api(
        &self,
        session_id: Ulid,
    ) -> BoxFuture<'_, Result<(), BrainError>> {
        Box::pin(async move {
            let turns = self.inner.active_turns.read().await;
            if let Some(token) = turns.get(&session_id) {
                token.cancel();
            }
            Ok(())
        })
    }
}

async fn drain_turn(inner: Arc<Inner>, session_id: Ulid, mut stream: EventStream) {
    while let Some(event) = stream.next().await {
        inner.event_bus.publish(ServerEvent::new(session_id, event));
    }
    inner.active_turns.write().await.remove(&session_id);
}
