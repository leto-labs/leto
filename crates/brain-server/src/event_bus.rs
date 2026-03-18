use tokio::sync::broadcast;

use crate::types::ServerEvent;

const DEFAULT_CAPACITY: usize = 1024;

pub struct EventBus {
    tx: broadcast::Sender<ServerEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    pub fn publish(&self, event: ServerEvent) {
        if let Err(e) = self.tx.send(event) {
            tracing::warn!("EventBus: no active subscribers, event dropped ({})", e);
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ServerEvent> {
        self.tx.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(DEFAULT_CAPACITY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brain_types::Event;
    use ulid::Ulid;

    fn test_event(session_id: Ulid) -> ServerEvent {
        ServerEvent::new(
            session_id,
            Event::TurnDone {
                iterations: 1,
                total_tokens: 10,
            },
        )
    }

    #[tokio::test]
    async fn publish_and_receive() {
        let bus = EventBus::default();
        let mut rx = bus.subscribe();

        let sid = Ulid::new();
        bus.publish(test_event(sid));

        let got = rx.recv().await.unwrap();
        assert_eq!(got.session_id, sid);
    }

    #[tokio::test]
    async fn multiple_subscribers() {
        let bus = EventBus::default();
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();

        let sid = Ulid::new();
        bus.publish(test_event(sid));

        let e1 = rx1.recv().await.unwrap();
        let e2 = rx2.recv().await.unwrap();
        assert_eq!(e1.session_id, sid);
        assert_eq!(e2.session_id, sid);
    }

    #[tokio::test]
    async fn dropped_subscriber_does_not_break_bus() {
        let bus = EventBus::default();
        let rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();
        drop(rx1);

        let sid = Ulid::new();
        bus.publish(test_event(sid));

        let got = rx2.recv().await.unwrap();
        assert_eq!(got.session_id, sid);
    }

    #[test]
    fn publish_with_no_subscribers_does_not_panic() {
        let bus = EventBus::default();
        bus.publish(test_event(Ulid::new()));
    }
}
