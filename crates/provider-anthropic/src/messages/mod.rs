//! Messages API surface.
//!
//! Official reference:
//! <https://platform.claude.com/docs/en/build-with-claude/working-with-messages>

mod client;
mod parser;
mod types;

pub use client::{MessageStream, MessagesClient};
pub use types::*;
