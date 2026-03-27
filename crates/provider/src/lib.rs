//! Shared provider SDK primitives for inference-oriented AI integrations.
//!
//! This crate defines a provider-neutral contract centered around:
//!
//! - [`Provider`] for stream-first inference
//! - [`Request`] and transcript/content types for shared inputs
//! - [`Event`] and related block types for shared streamed outputs
//! - [`ProviderCapabilities`] and [`ProviderInfo`] for capability discovery
//! - [`ModelInfo`] and related catalog types for static model metadata
//! - [`MockProvider`] for deterministic tests and local integration work
//!
//! The crate intentionally does not model agent-runtime concerns such as tool
//! execution policy, steering, approvals, or session orchestration. Those
//! behaviors belong in higher layers built on top of this SDK surface.
//!
//! # Example
//!
//! ```rust
//! use provider::{MockProvider, Provider, Request};
//!
//! # futures::executor::block_on(async {
//! let provider = MockProvider::new();
//! let request = Request::user_text("hello");
//! let _stream = provider.stream(&request).await.unwrap();
//! # });
//! ```

pub mod capability;
pub mod credential;
pub mod error;
pub mod event;
pub mod mock;
pub mod model;
pub mod provider_trait;
pub mod request;
pub mod tool;
pub mod usage;

pub use capability::*;
pub use credential::*;
pub use error::*;
pub use event::*;
pub use mock::*;
pub use model::*;
pub use provider_trait::*;
pub use request::*;
pub use tool::*;
pub use usage::*;
