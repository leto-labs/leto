//! Standalone OpenAI-compatible wire client with explicit API surfaces.
//!
//! Official references:
//! - Responses API overview: <https://developers.openai.com/api/reference/responses/overview>
//! - Create a response: <https://developers.openai.com/api/reference/resources/responses/methods/create>
//! - Chat Completions overview: <https://developers.openai.com/api/reference/chat-completions/overview>
//! - Create a chat completion: <https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create>
//!
//! This crate intentionally models the modern surfaces separately:
//! - [`responses`]
//! - [`chat_completions`]
//!
//! It does not currently model legacy prompt-based `/completions`.
//!
//! The protocol-native [`Client`] is the primary wire abstraction.
//! [`OpenAiProvider`] is an optional adapter that exposes the shared
//! [`provider::Provider`] trait on top of the Responses API.

pub mod chat_completions;
pub mod client;
pub mod config;
pub mod error;
pub mod provider_impl;
pub mod responses;
mod shared;

pub use chat_completions::*;
pub use client::Client;
pub use config::Config;
pub use error::Error;
pub use provider_impl::OpenAiProvider;
pub use responses::*;
pub use shared::TokenUsage;
