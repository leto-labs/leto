use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::Rng;
use sha2::{Digest, Sha256};

const VERIFIER_LEN: usize = 43;
const UNRESERVED: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";

/// Generate a PKCE code verifier and its S256 challenge.
///
/// Returns `(verifier, challenge)` where:
/// - `verifier` is a 43-character cryptographically random string of unreserved URI chars
/// - `challenge` is `base64url(SHA256(verifier))` with no padding
pub fn pkce_verifier() -> (String, String) {
    let mut rng = rand::rng();
    let verifier: String = (0..VERIFIER_LEN)
        .map(|_| {
            let idx = rng.random_range(0..UNRESERVED.len());
            UNRESERVED[idx] as char
        })
        .collect();

    let challenge = s256_challenge(&verifier);
    (verifier, challenge)
}

/// Compute the S256 challenge for a given verifier string.
pub fn s256_challenge(verifier: &str) -> String {
    let hash = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifier_length_and_charset() {
        let (verifier, _) = pkce_verifier();
        assert_eq!(verifier.len(), VERIFIER_LEN);
        for c in verifier.chars() {
            assert!(
                UNRESERVED.contains(&(c as u8)),
                "unexpected char in verifier: {c}"
            );
        }
    }

    #[test]
    fn challenge_is_deterministic_for_same_verifier() {
        let c1 = s256_challenge("test_verifier_1234567890_abcdefghijklmnop");
        let c2 = s256_challenge("test_verifier_1234567890_abcdefghijklmnop");
        assert_eq!(c1, c2);
    }

    #[test]
    fn challenge_is_valid_base64url() {
        let (_, challenge) = pkce_verifier();
        assert!(URL_SAFE_NO_PAD.decode(&challenge).is_ok());
        assert!(!challenge.contains('='));
        assert!(!challenge.contains('+'));
        assert!(!challenge.contains('/'));
    }

    #[test]
    fn different_verifiers_produce_different_challenges() {
        let (v1, c1) = pkce_verifier();
        let (v2, c2) = pkce_verifier();
        assert_ne!(v1, v2);
        assert_ne!(c1, c2);
    }
}
