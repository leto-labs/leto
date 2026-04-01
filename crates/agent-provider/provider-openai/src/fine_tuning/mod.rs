//! Fine-tuning API surface.
//!
//! Official reference:
//! - Overview: <https://platform.openai.com/docs/api-reference/fine-tuning>
//! - List jobs: <https://platform.openai.com/docs/api-reference/fine-tuning/jobs/list>

mod client;
mod types;

pub use client::FineTuningClient;
pub use types::*;
