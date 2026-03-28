//! Error types for the ACP adapter.

use agent_client_protocol as acp;

/// Maps a store error to ACP error.
pub fn map_store_error(error: impl std::error::Error) -> acp::Error {
    acp::Error::internal_error(error.to_string())
}

/// Creates an internal ACP error.
pub fn internal_error(message: impl Into<String>) -> acp::Error {
    acp::Error::internal_error(message.into())
}