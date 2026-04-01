//! Error types for the OpenAI wire client.

/// Errors returned by the OpenAI wire client and parser layers.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Authentication or OAuth lifecycle failure.
    #[error("auth: {0}")]
    Auth(String),
    /// Upstream inference or transport failure.
    #[error("inference: {0}")]
    Inference(String),
    /// Internal client construction or request-building failure.
    #[error("internal: {0}")]
    Internal(String),
    /// JSON serialization or deserialization failure.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}
