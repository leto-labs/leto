//! Embeddings API surface.
//!
//! Official reference:
//! - Overview: <https://platform.openai.com/docs/api-reference/embeddings>
//! - Create embeddings: <https://platform.openai.com/docs/api-reference/embeddings/create>

mod client;
mod types;

pub use client::EmbeddingsClient;
pub use types::*;
