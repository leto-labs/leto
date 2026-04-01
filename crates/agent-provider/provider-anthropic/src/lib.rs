//! Standalone Anthropic wire client with an explicit Messages API surface.
//!
//! Official references:
//! - API Overview: <https://platform.claude.com/docs/en/api/overview>
//! - Messages create: <https://platform.claude.com/docs/en/api/messages/create>
//! - Messages guide: <https://platform.claude.com/docs/en/build-with-claude/working-with-messages>
//! - Streaming: <https://platform.claude.com/docs/en/build-with-claude/streaming>
//!
//! The protocol-native [`Client`] is the primary wire abstraction.
//! [`AnthropicProvider`] is an optional adapter that exposes the shared
//! [`provider::Provider`] trait on top of Anthropic Messages.

pub mod client;
pub mod config;
pub mod error;
pub mod messages;
pub mod provider_impl;
mod shared;

pub use client::Client;
pub use config::Config;
pub use error::Error;
pub use messages::*;
pub use provider_impl::AnthropicProvider;
