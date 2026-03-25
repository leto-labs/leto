//! Microsoft Teams adapter for the shared [`chat`] SDK.

mod adapter;
mod config;
mod event;

pub use adapter::TeamsAdapter;
pub use config::TeamsConfig;
pub use event::{TeamsActivity, TeamsAttachment};
