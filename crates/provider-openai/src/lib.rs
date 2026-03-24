//! Standalone OpenAI-compatible wire client with explicit API surfaces.
//!
//! Official references:
//! - Responses API: <https://developers.openai.com/api/reference/responses/overview>
//! - Chat Completions API: <https://developers.openai.com/api/reference/chat-completions/overview>
//!
//! This crate intentionally models the modern surfaces separately:
//! - [`responses`]
//! - [`chat_completions`]
//!
//! It does not currently model legacy prompt-based `/completions`.

pub mod chat_completions;
pub mod client;
pub mod config;
pub mod error;
pub mod responses;
mod shared;

pub use chat_completions::*;
pub use client::Client;
pub use config::Config;
pub use error::Error;
pub use responses::*;
pub use shared::TokenUsage;
