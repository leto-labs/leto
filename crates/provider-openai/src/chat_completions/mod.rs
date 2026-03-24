//! Chat Completions API surface.
//!
//! Official reference:
//! <https://developers.openai.com/api/reference/chat-completions/overview>

mod client;
mod parser;
mod types;

pub use client::{ChatCompletionsClient, ChatCompletionStream};
pub use types::*;
