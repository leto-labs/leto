use std::fmt::Display;

use agent_client_protocol as acp;
use brain_core::{BrainError, BrainErrorCode};

pub fn internal_error(message: impl Display) -> acp::Error {
    tracing::error!("{message}");
    acp::Error::internal_error()
}

pub fn map_brain_error(error: BrainError) -> acp::Error {
    match error.code() {
        BrainErrorCode::StorageFailed | BrainErrorCode::TurnActive => acp::Error::invalid_params(),
        BrainErrorCode::Cancelled => acp::Error::invalid_request(),
        _ => internal_error(error),
    }
}
