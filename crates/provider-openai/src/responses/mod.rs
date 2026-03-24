//! Responses API surface.
//!
//! Official reference:
//! <https://developers.openai.com/api/reference/responses/overview>

mod client;
mod parser;
mod types;

pub use client::{ResponseStream, ResponseStreamTransport, ResponsesClient};
pub use types::*;
