//! Assistants API surface.
//!
//! Official reference:
//! - Overview: <https://platform.openai.com/docs/api-reference/assistants>
//! - Create assistant: <https://platform.openai.com/docs/api-reference/assistants>

mod client;
mod types;

pub use client::AssistantsClient;
pub use types::*;
