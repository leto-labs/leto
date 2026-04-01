//! Shared error types for the provider SDK.

/// Errors returned by shared-provider implementations and adapters.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The caller supplied invalid configuration or the adapter could not build
    /// a valid provider client.
    #[error("configuration: {0}")]
    Configuration(String),
    /// The requested behavior cannot be represented by the target provider.
    #[error("unsupported: {0}")]
    Unsupported(String),
    /// The provider failed while performing inference or streaming a response.
    #[error("inference: {0}")]
    Inference(String),
    /// The upstream service returned an explicit error payload.
    #[error("remote: {0}")]
    Remote(String),
    /// JSON serialization or deserialization failed.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}
