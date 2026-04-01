/// Tool-scoped errors returned by shared tool executors and implementations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct ToolError {
    message: String,
}

impl ToolError {
    /// Creates a new tool error from a human-readable message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// Returns the underlying error message.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl From<String> for ToolError {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for ToolError {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}
