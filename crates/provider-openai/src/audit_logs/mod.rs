//! Audit Logs API surface.
//!
//! Official reference:
//! - Overview: <https://platform.openai.com/docs/api-reference/audit-logs>
//! - List audit logs: <https://platform.openai.com/docs/api-reference/audit-logs>

mod client;
mod types;

pub use client::AuditLogsClient;
pub use types::*;
