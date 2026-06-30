//! Minimal cryptographic helpers for the identity domain.
//!
//! Contains Ed25519 signature verification used by the wallet claim flow.

use ring::signature::{UnparsedPublicKey, ED25519};

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
