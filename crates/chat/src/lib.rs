//! Shared chat SDK primitives for external messaging integrations.
//!
//! This crate defines a chat-provider-neutral contract centered around:
//!
//! - [`ChatAdapter`] for adapter implementations
//! - [`ChatEvent`] and [`ChatEventStream`] for normalized inbound events
//! - [`ChatCapabilities`] for capability discovery
//! - conversation, participant, message, and attachment types for normalized
//!   chat payloads
//!
//! The crate intentionally does not model runtime, session, tool, or agent
//! concepts. Those behaviors belong in higher layers that compose the chat SDK.

pub mod adapter;
pub mod capability;
pub mod error;
pub mod event;
pub mod message;

pub use adapter::*;
pub use capability::*;
pub use error::*;
pub use event::*;
pub use message::*;
