use std::pin::Pin;
use std::sync::Arc;

use futures::Stream;
use tokio_util::sync::CancellationToken;
use ulid::Ulid;

use crate::provider::Provider;
use crate::tool::Tool;
use crate::{AgentConfig, Event, Message};

pub type EventStream = Pin<Box<dyn Stream<Item = Event> + Send>>;

pub trait AgentLoop: Send + Sync {
    fn run(
        &self,
        provider: Arc<dyn Provider>,
        tools: Vec<Arc<dyn Tool>>,
        messages: Vec<Message>,
        config: AgentConfig,
        cancel: CancellationToken,
        session_id: Option<Ulid>,
    ) -> EventStream;
}
