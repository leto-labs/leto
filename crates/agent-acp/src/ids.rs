//! ID conversion helpers for ACP.

use agent_client_protocol as acp;
use agent_store::SessionId;

/// Parses an ACP session ID into a SessionId.
pub fn parse_session_id(id: &acp::SessionId) -> Result<SessionId, acp::Error> {
    let ulid = id.0.parse::<ulid::Ulid>().map_err(|_| acp::Error::invalid_params())?;
    Ok(SessionId::from(ulid))
}

/// Converts a SessionId to an ACP session ID.
pub fn session_id_from_ulid(id: SessionId) -> acp::SessionId {
    acp::SessionId::new(id.to_string())
}