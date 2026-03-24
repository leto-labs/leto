#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("inference: {0}")]
    Inference(String),
    #[error("internal: {0}")]
    Internal(String),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}
