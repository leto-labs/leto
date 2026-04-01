//! Messages API surface.
//!
//! Official reference:
//! - Create messages: <https://platform.claude.com/docs/en/api/messages/create>
//! - Working with Messages: <https://platform.claude.com/docs/en/build-with-claude/working-with-messages>
//! - Streaming: <https://platform.claude.com/docs/en/build-with-claude/streaming>

mod client;
mod parser;
mod types;

pub use client::{MessageStream, MessagesClient};
pub use types::*;
