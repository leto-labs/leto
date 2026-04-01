//! Vector stores API surface.
//!
//! Official reference:
//! - Overview: <https://platform.openai.com/docs/api-reference/vector-stores>
//! - Create vector store: <https://platform.openai.com/docs/api-reference/vector-stores/create>

mod client;
mod types;

pub use client::VectorStoresClient;
pub use types::*;
