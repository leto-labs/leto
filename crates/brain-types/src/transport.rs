use futures::future::BoxFuture;
use ulid::Ulid;

use crate::{BrainError, Event};

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum InputEvent {
    Message(String),
    ToolApproval {
        id: String,
        approved: bool,
        reason: Option<String>,
    },
    Cancel,
    SwitchSession(Ulid),
}

pub trait Transport: Send + Sync {
    fn name(&self) -> &str;
    fn recv(&self) -> BoxFuture<'_, Result<Option<InputEvent>, BrainError>>;
    fn send(&self, event: Event) -> BoxFuture<'_, Result<(), BrainError>>;
}
