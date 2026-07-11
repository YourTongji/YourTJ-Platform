//! Campus-email verification code primitives.
//!
//! Each code is a 6-digit random number generated from the system's
//! cryptographic RNG. Codes are stored as a SHA-256 hash and compared
//! in constant time to prevent timing side-channels.

use ring::rand::{SecureRandom, SystemRandom};
use sha2::{Digest, Sha256};

/// Security purpose bound to a verification code at issuance time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodePurpose {
    Login,
    Registration,
    PasswordReset,
    RecentAuth,
    Appeal,
}

impl CodePurpose {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Login => "login",
            Self::Registration => "registration",
            Self::PasswordReset => "password_reset",
            Self::RecentAuth => "recent_auth",
            Self::Appeal => "appeal",
        }
    }

    pub fn from_stored(value: &str) -> Option<Self> {
        match value {
            "login" => Some(Self::Login),
            "registration" => Some(Self::Registration),
            "password_reset" => Some(Self::PasswordReset),
            "recent_auth" => Some(Self::RecentAuth),
            "appeal" => Some(Self::Appeal),
            _ => None,
        }
    }
}

/// Generate a 6-digit verification code (e.g. "482913").
pub fn generate_code() -> String {
    let rng = SystemRandom::new();
    let mut buf = [0u8; 3]; // 3 bytes → up to 16_777_215 — enough bias-free 6 digits
    rng.fill(&mut buf).expect("system CSPRNG must not fail");
    let n = u32::from_be_bytes([0, buf[0], buf[1], buf[2]]) % 1_000_000;
    format!("{n:06}")
}

/// Produce the SHA-256 hex digest of `code`. This is what we store in the DB.
pub fn hash_code(code: &str) -> String {
    hex::encode(Sha256::digest(code.as_bytes()))
}

/// Compare a user's attempted code against the stored hash in constant time.
pub fn verify_code(attempt: &str, hash: &str) -> bool {
    let attempt_hash = Sha256::digest(attempt.as_bytes());
    let stored_bytes = hex::decode(hash).unwrap_or_default();
    // Constant-time comparison: always traverses both slices fully regardless of
    // where (or whether) a mismatch occurs, to avoid timing side-channels.
    if attempt_hash.len() != stored_bytes.len() {
        return false;
    }
    let mut diff = 0u8;
    for (a, b) in attempt_hash.iter().zip(stored_bytes.iter()) {
        diff |= a ^ b;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_has_length_6() {
        for _ in 0..100 {
            let code = generate_code();
            assert_eq!(code.len(), 6, "code '{code}' is not 6 chars");
            assert!(code.chars().all(|c| c.is_ascii_digit()), "code '{code}' has non-digit");
        }
    }

    #[test]
    fn hash_is_deterministic() {
        let h1 = hash_code("123456");
        let h2 = hash_code("123456");
        assert_eq!(h1, h2);
    }

    #[test]
    fn correct_code_verifies() {
        let code = "789012";
        let hash = hash_code(code);
        assert!(verify_code(code, &hash));
    }

    #[test]
    fn wrong_code_rejects() {
        let hash = hash_code("111111");
        assert!(!verify_code("222222", &hash));
    }

    #[test]
    fn wrong_length_code_rejects() {
        let hash = hash_code("123456");
        assert!(!verify_code("12345", &hash));
        assert!(!verify_code("1234567", &hash));
        assert!(!verify_code("", &hash));
    }
}
