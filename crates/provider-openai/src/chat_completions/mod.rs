//! Chat Completions API surface.
//!
//! Official reference:
//! - Overview: <https://developers.openai.com/api/reference/chat-completions/overview>
//! - Create a chat completion: <https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create>
//! - Streaming events: <https://developers.openai.com/api/reference/resources/chat/subresources/completions/streaming-events>

mod client;
mod parser;
mod types;

pub use client::{ChatCompletionStream, ChatCompletionsClient};
pub use types::*;
