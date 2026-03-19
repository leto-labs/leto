use tokio::sync::mpsc;

use super::agent::{MockAgent, NotificationEnvelope};

pub(super) fn build_agent(
    session_update_tx: mpsc::UnboundedSender<NotificationEnvelope>,
) -> MockAgent {
    MockAgent::new(session_update_tx)
}
