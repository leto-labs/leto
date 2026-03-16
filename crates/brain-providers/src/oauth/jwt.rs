use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

/// Extract a provider-specific account ID from a JWT access token.
///
/// Decodes the JWT payload without signature verification (the token is
/// already trusted from the OAuth token endpoint) and looks for:
/// 1. `chatgpt_account_id` claim (preferred)
/// 2. `organizations[0].id` (fallback)
///
/// Returns `None` if neither claim exists or the token is malformed.
pub fn extract_account_id(token: &str) -> Option<String> {
    let payload_b64 = token.split('.').nth(1)?;
    let payload_bytes = URL_SAFE_NO_PAD.decode(payload_b64).ok()?;
    let claims: serde_json::Value = serde_json::from_slice(&payload_bytes).ok()?;

    if let Some(id) = claims.get("chatgpt_account_id").and_then(|v| v.as_str()) {
        return Some(id.to_owned());
    }

    claims
        .get("organizations")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|org| org.get("id"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;

    fn make_jwt(payload_json: &str) -> String {
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"RS256","typ":"JWT"}"#);
        let payload = URL_SAFE_NO_PAD.encode(payload_json);
        let sig = URL_SAFE_NO_PAD.encode("fake_signature");
        format!("{header}.{payload}.{sig}")
    }

    #[test]
    fn extracts_chatgpt_account_id() {
        let jwt = make_jwt(r#"{"chatgpt_account_id": "acct_abc123"}"#);
        assert_eq!(extract_account_id(&jwt), Some("acct_abc123".into()));
    }

    #[test]
    fn falls_back_to_organization_id() {
        let jwt = make_jwt(r#"{"organizations": [{"id": "org_xyz789"}]}"#);
        assert_eq!(extract_account_id(&jwt), Some("org_xyz789".into()));
    }

    #[test]
    fn chatgpt_account_id_takes_precedence() {
        let jwt = make_jwt(
            r#"{"chatgpt_account_id": "acct_1", "organizations": [{"id": "org_2"}]}"#,
        );
        assert_eq!(extract_account_id(&jwt), Some("acct_1".into()));
    }

    #[test]
    fn returns_none_for_no_claims() {
        let jwt = make_jwt(r#"{"sub": "user_123"}"#);
        assert_eq!(extract_account_id(&jwt), None);
    }

    #[test]
    fn returns_none_for_malformed_token() {
        assert_eq!(extract_account_id("not.a.jwt.at.all"), None);
        assert_eq!(extract_account_id(""), None);
        assert_eq!(extract_account_id("only_one_part"), None);
    }
}
