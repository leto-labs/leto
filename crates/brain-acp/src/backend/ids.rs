use agent_client_protocol as acp;
use ulid::Ulid;

pub fn session_id_from_ulid(session_id: Ulid) -> acp::SessionId {
    acp::SessionId::new(session_id.to_string())
}

pub fn parse_session_id(session_id: &acp::SessionId) -> Result<Ulid, acp::Error> {
    session_id
        .0
        .parse::<Ulid>()
        .map_err(|_| acp::Error::invalid_params())
}
