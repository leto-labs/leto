use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrainErrorCode {
    InferenceFailed,
    AuthFailed,
    ToolNotFound,
    ToolFailed,
    MaxIterations,
    Cancelled,
    TurnActive,
    StorageFailed,
    Internal,
}

#[derive(Debug, thiserror::Error)]
pub enum BrainError {
    #[error("inference: {0}")]
    Inference(String),

    #[error("auth: {0}")]
    Auth(String),

    #[error("tool not found: {0}")]
    ToolNotFound(String),

    #[error("tool failed: {tool} — {reason}")]
    ToolFailed { tool: String, reason: String },

    #[error("max iterations reached: {0}")]
    MaxIterations(u32),

    #[error("cancelled")]
    Cancelled,

    #[error("session {0} already has an active turn")]
    TurnActive(ulid::Ulid),

    #[error("storage: {0}")]
    Storage(String),

    #[error("json: {0}")]
    Json(#[from] serde_json::Error),

    #[error("internal: {0}")]
    Internal(String),
}

impl BrainError {
    pub fn code(&self) -> BrainErrorCode {
        match self {
            Self::Inference(_) => BrainErrorCode::InferenceFailed,
            Self::Auth(_) => BrainErrorCode::AuthFailed,
            Self::ToolNotFound(_) => BrainErrorCode::ToolNotFound,
            Self::ToolFailed { .. } => BrainErrorCode::ToolFailed,
            Self::MaxIterations(_) => BrainErrorCode::MaxIterations,
            Self::Cancelled => BrainErrorCode::Cancelled,
            Self::TurnActive(_) => BrainErrorCode::TurnActive,
            Self::Storage(_) => BrainErrorCode::StorageFailed,
            _ => BrainErrorCode::Internal,
        }
    }

    pub fn recoverable(&self) -> bool {
        matches!(self, Self::ToolFailed { .. } | Self::Cancelled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes() {
        assert_eq!(
            BrainError::Inference("x".into()).code(),
            BrainErrorCode::InferenceFailed
        );
        assert_eq!(
            BrainError::Auth("x".into()).code(),
            BrainErrorCode::AuthFailed
        );
        assert_eq!(
            BrainError::ToolNotFound("x".into()).code(),
            BrainErrorCode::ToolNotFound
        );
        assert_eq!(
            BrainError::ToolFailed {
                tool: "t".into(),
                reason: "r".into()
            }
            .code(),
            BrainErrorCode::ToolFailed
        );
        assert_eq!(
            BrainError::MaxIterations(5).code(),
            BrainErrorCode::MaxIterations
        );
        assert_eq!(BrainError::Cancelled.code(), BrainErrorCode::Cancelled);
        assert_eq!(
            BrainError::Storage("x".into()).code(),
            BrainErrorCode::StorageFailed
        );
        assert_eq!(
            BrainError::Internal("x".into()).code(),
            BrainErrorCode::Internal
        );
    }

    #[test]
    fn recoverable_errors() {
        assert!(
            BrainError::ToolFailed {
                tool: "t".into(),
                reason: "r".into()
            }
            .recoverable()
        );
        assert!(BrainError::Cancelled.recoverable());

        assert!(!BrainError::Inference("x".into()).recoverable());
        assert!(!BrainError::Auth("x".into()).recoverable());
        assert!(!BrainError::Storage("x".into()).recoverable());
        assert!(!BrainError::MaxIterations(5).recoverable());
        assert!(!BrainError::Internal("x".into()).recoverable());
    }

    #[test]
    fn display_messages() {
        assert_eq!(
            BrainError::Inference("fail".into()).to_string(),
            "inference: fail"
        );
        assert_eq!(BrainError::Cancelled.to_string(), "cancelled");
        assert_eq!(
            BrainError::ToolFailed {
                tool: "echo".into(),
                reason: "boom".into()
            }
            .to_string(),
            "tool failed: echo — boom"
        );
        assert_eq!(
            BrainError::MaxIterations(10).to_string(),
            "max iterations reached: 10"
        );
    }

    #[test]
    fn json_from_serde_error() {
        let err: Result<serde_json::Value, _> = serde_json::from_str("not json");
        let brain_err = BrainError::from(err.unwrap_err());
        assert_eq!(brain_err.code(), BrainErrorCode::Internal);
    }

    #[test]
    fn error_code_serde_roundtrip() {
        let code = BrainErrorCode::ToolFailed;
        let json = serde_json::to_string(&code).unwrap();
        assert_eq!(json, r#""tool_failed""#);
        let roundtrip: BrainErrorCode = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip, code);
    }
}
