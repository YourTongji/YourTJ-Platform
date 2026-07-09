//! Cryptographic primitives for the credit ledger.
//!
//! Every ledger entry carries a SHA-256 hash of the canonical payload +
//! previous hash, and an Ed25519 signature over the canonical payload.
//! Verification recomputes both and checks against every entry.

use ring::signature::{Ed25519KeyPair, UnparsedPublicKey, ED25519};
use sha2::Digest as _;

/// Produce a deterministic, sort-order-independent string from a JSON payload.
/// Keys are sorted alphabetically; no whitespace.
pub fn canonicalize(payload: &serde_json::Value) -> String {
    serde_json::to_string(payload).expect("canonicalize: serialisation must not fail")
}

/// Build the canonical payload for a ledger entry. Returns the canonical JSON string
/// ready for hashing or signing.
#[allow(clippy::too_many_arguments)] // reason: canonical form must include all ledger fields for deterministic hashing; collapsing them would risk reorder bugs
pub fn build_ledger_canonical(
    tx_id: &str,
    type_: &str,
    from_account: Option<i64>,
    to_account: Option<i64>,
    amount: i64,
    nonce: &str,
    metadata: Option<&serde_json::Value>,
    signer: &str,
    created_at: i64,
) -> String {
    let payload = serde_json::json!({
        "tx_id": tx_id,
        "type": type_,
        "from_account": from_account.map(|v| v.to_string()),
        "to_account": to_account.map(|v| v.to_string()),
        "amount": amount,
        "nonce": nonce,
        "metadata": metadata,
        "signer": signer,
        "timestamp": created_at,
    });
    canonicalize(&payload)
}

/// Compute the entry hash: SHA-256 hex of `canonical || prev_hash`.
pub fn compute_hash(canonical: &str, prev_hash: &str) -> String {
    let mut hasher = sha2::Sha256::new();
    hasher.update(canonical.as_bytes());
    hasher.update(prev_hash.as_bytes());
    hex::encode(hasher.finalize())
}

/// Sign a payload with an Ed25519 private key. Returns the base64 signature.
pub fn sign_payload(payload: &str, private_key_bytes: &[u8]) -> String {
    let key_pair =
        Ed25519KeyPair::from_seed_unchecked(private_key_bytes).expect("invalid seed length");
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD.encode(key_pair.sign(payload.as_bytes()).as_ref())
}

/// Derive the Ed25519 public key bytes from a 32-byte seed. Returns the raw public key bytes.
pub fn derive_public_key(seed: &[u8]) -> Vec<u8> {
    use ring::signature::KeyPair;
    let key_pair = Ed25519KeyPair::from_seed_unchecked(seed).expect("invalid seed length");
    key_pair.public_key().as_ref().to_vec()
}

/// Sign a payload with an Ed25519 private key seed. Returns the base64 signature.
pub fn sign_with_seed(payload: &str, seed: &[u8]) -> String {
    let key_pair = Ed25519KeyPair::from_seed_unchecked(seed).expect("invalid seed length");
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD.encode(key_pair.sign(payload.as_bytes()).as_ref())
}

/// Verify an Ed25519 signature (base64) over a payload against a public key (base64).
pub fn verify_signature(payload: &str, signature_b64: &str, public_key_b64: &str) -> bool {
    use base64::Engine as _;
    let sig_bytes = match base64::engine::general_purpose::STANDARD.decode(signature_b64) {
        Ok(b) => b,
        Err(_) => return false,
    };
    let pk_bytes = match base64::engine::general_purpose::STANDARD.decode(public_key_b64) {
        Ok(b) => b,
        Err(_) => return false,
    };
    let public_key = UnparsedPublicKey::new(&ED25519, pk_bytes);
    public_key.verify(payload.as_bytes(), &sig_bytes).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn canonicalize_sorts_keys() {
        let a = json!({"b": 1, "a": 2});
        let b = json!({"a": 2, "b": 1});
        assert_eq!(canonicalize(&a), canonicalize(&b));
    }

    #[test]
    fn hash_is_deterministic() {
        let h1 = compute_hash("hello", "prev");
        let h2 = compute_hash("hello", "prev");
        assert_eq!(h1, h2);
    }

    #[test]
    fn hash_changes_with_input() {
        let h1 = compute_hash("hello", "prev");
        let h2 = compute_hash("hello", "other");
        assert_ne!(h1, h2);
    }

    #[test]
    fn sign_and_verify_roundtrip() {
        use ring::rand::SystemRandom;
        use ring::signature::KeyPair;

        let rng = SystemRandom::new();
        let pkcs8 = Ed25519KeyPair::generate_pkcs8(&rng).expect("gen key");
        let key_pair = Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).expect("parse key");

        use base64::Engine as _;
        let pk_b64 =
            base64::engine::general_purpose::STANDARD.encode(key_pair.public_key().as_ref());
        let seed: &[u8] = pkcs8.as_ref();
        // The seed is the first 32 bytes of the pkcs8 key.
        let seed_bytes = &seed[16..48]; // ring pkcs8 v2 has the seed at offset 16

        let payload = r#"{"amount":100,"type":"tip"}"#;
        let sig = sign_payload(payload, seed_bytes);
        assert!(verify_signature(payload, &sig, &pk_b64));
    }

    #[test]
    fn verify_rejects_wrong_key() {
        use ring::rand::SystemRandom;
        use ring::signature::KeyPair;

        let rng = SystemRandom::new();
        let pkcs8_1 = Ed25519KeyPair::generate_pkcs8(&rng).expect("gen key 1");
        let pkcs8_2 = Ed25519KeyPair::generate_pkcs8(&rng).expect("gen key 2");
        let kp2 = Ed25519KeyPair::from_pkcs8(pkcs8_2.as_ref()).expect("parse key 2");

        use base64::Engine as _;
        let pk2_b64 = base64::engine::general_purpose::STANDARD.encode(kp2.public_key().as_ref());
        let seed1: &[u8] = pkcs8_1.as_ref();
        let seed1_bytes = &seed1[16..48];

        let payload = r#"{"amount":100}"#;
        let sig = sign_payload(payload, seed1_bytes);
        assert!(!verify_signature(payload, &sig, &pk2_b64));
    }
}
