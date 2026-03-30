//! Videos API surface.
//!
//! Official reference:
//! - Overview: <https://platform.openai.com/docs/api-reference/videos>
//! - Create video: <https://platform.openai.com/docs/api-reference/videos/create>

mod client;
mod types;

pub use client::VideosClient;
pub use types::*;
