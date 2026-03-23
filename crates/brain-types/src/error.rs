use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InferenceErrorKind {
    Generic,
    ContextLengthExceeded,
    OutputLengthExceeded,
}

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

    pub fn inference_kind(&self) -> Option<InferenceErrorKind> {
        match self {
            Self::Inference(message) => Some(classify_inference_message(message)),
            _ => None,
        }
    }
}

fn classify_inference_message(message: &str) -> InferenceErrorKind {
    let lower = message.to_ascii_lowercase();

    if lower.contains("context length")
        || lower.contains("context window")
        || lower.contains("maximum context")
        || lower.contains("max context")
        || lower.contains("too many tokens")
        || lower.contains("prompt is too long")
    {
        return InferenceErrorKind::ContextLengthExceeded;
    }

    if lower.contains("output length")
        || lower.contains("max_tokens limit")
        || lower.contains("response was truncated")
        || lower.contains("maximum output")
        || lower.contains("max output")
        || lower.contains("completion length")
    {
        return InferenceErrorKind::OutputLengthExceeded;
    }

    InferenceErrorKind::Generic
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

    #[test]
    fn classifies_context_length_inference_errors() {
        let error = BrainError::Inference("maximum context length exceeded".into());
        assert_eq!(
            error.inference_kind(),
            Some(InferenceErrorKind::ContextLengthExceeded)
        );
    }

    #[test]
    fn classifies_output_length_inference_errors() {
        let error = BrainError::Inference("Model hit max_tokens limit".into());
        assert_eq!(
            error.inference_kind(),
            Some(InferenceErrorKind::OutputLengthExceeded)
        );
    }

    #[test]
    fn defaults_unrecognized_inference_errors_to_generic() {
        let error = BrainError::Inference("temporary upstream error".into());
        assert_eq!(error.inference_kind(), Some(InferenceErrorKind::Generic));
    }
}
