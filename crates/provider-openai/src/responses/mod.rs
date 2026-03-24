//! Responses API surface.
//!
//! Official reference:
//! - Overview: <https://developers.openai.com/api/reference/responses/overview>
//! - Create a response: <https://developers.openai.com/api/reference/resources/responses/methods/create>
//! - Retrieve a response: <https://developers.openai.com/api/reference/resources/responses/methods/retrieve>
//! - List input items: <https://developers.openai.com/api/reference/resources/responses/subresources/input_items/methods/list>
//! - Count input tokens: <https://developers.openai.com/api/reference/resources/responses/subresources/input_tokens/methods/count>
//! - Cancel a response: <https://developers.openai.com/api/reference/resources/responses/methods/cancel>
//! - Compact a response: <https://developers.openai.com/api/reference/resources/responses/methods/compact>

mod client;
mod parser;
mod types;

pub use client::{ResponseStream, ResponseStreamTransport, ResponsesClient};
pub use types::*;
