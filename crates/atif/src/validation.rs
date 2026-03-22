use thiserror::Error;

use crate::SchemaVersion;

#[derive(Debug, Error, PartialEq)]
pub enum ValidationError {
    #[error("trajectory must contain at least one step")]
    EmptySteps,
    #[error("invalid step_id {0}, expected step IDs to start at 1")]
    InvalidStepId(u32),
    #[error("non-sequential step_id: expected {expected}, got {actual}")]
    NonSequentialStepId { expected: u32, actual: u32 },
    #[error("invalid ISO 8601 timestamp: {timestamp}")]
    InvalidTimestamp { timestamp: String },
    #[error("agent-only fields present on non-agent step {step_id}")]
    AgentOnlyFieldOnNonAgentStep { step_id: u32 },
    #[error("unknown observation source_call_id '{source_call_id}' on step {step_id}")]
    UnknownSourceCallId {
        step_id: u32,
        source_call_id: String,
    },
    #[error("'text' field is required when type='text'")]
    MissingTextContentPartField,
    #[error("'source' field is not allowed when type='text'")]
    UnexpectedImageSourceOnTextPart,
    #[error("'source' field is required when type='image'")]
    MissingImageContentPartField,
    #[error("'text' field is not allowed when type='image'")]
    UnexpectedTextOnImagePart,
    #[error(
        "field '{field}' requires {minimum_version} or newer, but trajectory declares {actual_version}"
    )]
    UnsupportedFieldForSchemaVersion {
        field: &'static str,
        minimum_version: SchemaVersion,
        actual_version: SchemaVersion,
    },
}

pub(crate) fn require_supported(
    supported: bool,
    field: &'static str,
    minimum_version: SchemaVersion,
    actual_version: SchemaVersion,
) -> Result<(), ValidationError> {
    if supported {
        Ok(())
    } else {
        Err(ValidationError::UnsupportedFieldForSchemaVersion {
            field,
            minimum_version,
            actual_version,
        })
    }
}
