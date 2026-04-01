//! Rate Limits API surface.
//!
//! Official reference:
//! - Overview: <https://platform.openai.com/docs/api-reference/project-rate-limits>
//! - List project rate limits: <https://platform.openai.com/docs/api-reference/project-rate-limits/list>

mod client;
mod types;

pub use client::RateLimitsClient;
pub use types::*;
