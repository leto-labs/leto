use thiserror::Error;

/// Shared error type for chat adapter operations.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Error {
    /// The requested capability is not implemented by the adapter.
    #[error("adapter `{adapter}` does not support capability `{capability}`")]
    UnsupportedCapability {
        /// Adapter identifier that rejected the request.
        adapter: String,
        /// Capability name associated with the operation.
        capability: &'static str,
    },
    /// The adapter could not normalize or validate a provider payload.
    #[error("invalid chat payload: {0}")]
    InvalidPayload(String),
    /// The adapter encountered a transport or delivery failure.
    #[error("chat transport error: {0}")]
    Transport(String),
    /// The adapter is missing required authentication or configuration.
    #[error("chat authentication/configuration error: {0}")]
    Authentication(String),
}
