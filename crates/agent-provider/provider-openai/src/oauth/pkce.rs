use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::Rng;
use sha2::{Digest, Sha256};

const VERIFIER_LEN: usize = 43;
const UNRESERVED: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";

pub fn pkce_verifier() -> (String, String) {
    let mut rng = rand::rng();
    let verifier: String = (0..VERIFIER_LEN)
        .map(|_| {
            let idx = rng.random_range(0..UNRESERVED.len());
            UNRESERVED[idx] as char
        })
        .collect();
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    (verifier, challenge)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifier_has_expected_length() {
        let (verifier, challenge) = pkce_verifier();
        assert_eq!(verifier.len(), VERIFIER_LEN);
        assert!(!challenge.is_empty());
    }
}
