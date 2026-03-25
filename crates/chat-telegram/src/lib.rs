//! Telegram adapter for the shared [`chat`] SDK.

mod adapter;
mod config;
mod event;

pub use adapter::TelegramAdapter;
pub use config::TelegramConfig;
pub use event::{TelegramAttachment, TelegramUpdate};
