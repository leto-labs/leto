pub(super) fn compat_session_id(session_id: SessionId) -> String {
    format!("ses{session_id}")
}

pub(super) fn parse_compat_session_id(value: &str) -> Result<SessionId, CompatRequestError> {
    value
        .strip_prefix("ses")
        .unwrap_or(value)
        .parse()
        .map_err(|_| CompatRequestError::InvalidRequest("invalid_session_id"))
}

pub(super) fn compat_message_id(message_id: Ulid) -> String {
    format!("msg{message_id}")
}

pub(super) fn parse_compat_message_id(value: &str) -> Result<Ulid, CompatRequestError> {
    value
        .strip_prefix("msg")
        .unwrap_or(value)
        .parse()
        .map_err(|_| CompatRequestError::InvalidRequest("invalid_message_id"))
}

pub(super) fn compat_part_id(message_id: Ulid, index: usize) -> String {
    format!("prt{}_{}", compat_message_id(message_id), index)
}

pub(super) fn parse_compat_part_index(
    message_id: Ulid,
    value: &str,
) -> Result<usize, CompatRequestError> {
    let stripped = value.strip_prefix("prt").unwrap_or(value);
    let Some((message, index)) = stripped.rsplit_once('_') else {
        return Err(CompatRequestError::InvalidRequest("invalid_part_id"));
    };
    let parsed_message = parse_compat_message_id(message)?;
    if parsed_message != message_id {
        return Err(CompatRequestError::InvalidRequest("invalid_part_id"));
    }
    index
        .parse::<usize>()
        .map_err(|_| CompatRequestError::InvalidRequest("invalid_part_id"))
}
