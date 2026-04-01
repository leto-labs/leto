use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

pub fn extract_account_id(token: &str) -> Option<String> {
    let payload_b64 = token.split('.').nth(1)?;
    let payload_bytes = URL_SAFE_NO_PAD.decode(payload_b64).ok()?;
    let claims: serde_json::Value = serde_json::from_slice(&payload_bytes).ok()?;
    if let Some(id) = claims
        .get("chatgpt_account_id")
        .and_then(|value| value.as_str())
    {
        return Some(id.to_owned());
    }
    claims
        .get("organizations")
        .and_then(|value| value.as_array())
        .and_then(|items| items.first())
        .and_then(|item| item.get("id"))
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;

    fn make_jwt(payload_json: &str) -> String {
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"RS256","typ":"JWT"}"#);
        let payload = URL_SAFE_NO_PAD.encode(payload_json);
        let sig = URL_SAFE_NO_PAD.encode("sig");
        format!("{header}.{payload}.{sig}")
    }

    #[test]
    fn extracts_account_id() {
        let jwt = make_jwt(r#"{"chatgpt_account_id":"acct_123"}"#);
        assert_eq!(extract_account_id(&jwt).as_deref(), Some("acct_123"));
    }
}
