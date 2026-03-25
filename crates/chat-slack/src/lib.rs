//! Slack adapter for the shared [`chat`] SDK.

mod adapter;
mod config;
mod event;

pub use adapter::SlackAdapter;
pub use config::SlackConfig;
pub use event::{SlackEvent, SlackFile};
