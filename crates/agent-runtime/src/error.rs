/// Errors raised by the reusable agent runtime surface.
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("provider: {0}")]
    Provider(#[from] provider::Error),
    #[error("tool: {0}")]
    Tool(String),
    #[error("worktree: {0}")]
    Worktree(String),
    #[error("session closed")]
    Closed,
    #[error("cancelled")]
    Cancelled,
    #[error("max iterations reached: {0}")]
    MaxIterations(u32),
    #[error("internal: {0}")]
    Internal(String),
}

impl RuntimeError {
    pub fn recoverable(&self) -> bool {
        matches!(
            self,
            Self::Provider(provider::Error::Inference(_) | provider::Error::Remote(_))
        )
    }
}
