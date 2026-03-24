//! Standalone Anthropic wire client with an explicit Messages API surface.
//!
//! Official references:
//! - API Overview: <https://platform.claude.com/docs/en/api/overview>
//! - Messages API: <https://platform.claude.com/docs/en/build-with-claude/working-with-messages>
//! - Streaming: <https://platform.claude.com/docs/en/build-with-claude/streaming>

pub mod client;
pub mod config;
pub mod error;
pub mod messages;
mod shared;

pub use client::Client;
pub use config::Config;
pub use error::Error;
pub use messages::*;
