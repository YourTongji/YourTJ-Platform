//! JWT access-token creation/verification and refresh-token generation.
//!
//! Access tokens are HS256 JWTs containing `{ sub, exp, iat }`.
//! Refresh tokens are 256 random bits, stored as hex; their SHA-256 hash
//! is persisted in `identity.sessions.refresh_hash`.

use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Claims inside an access token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ver: Option<i64>,
}

/// Create a HS256-signed access token valid for `ttl_secs`.
pub fn create_access_token(account_id: i64, secret: &str, ttl_secs: u64) -> Result<String, String> {
    let now = Utc::now().timestamp() as usize;
    let claims = JwtClaims {
        sub: account_id.to_string(),
        exp: now + ttl_secs as usize,
        iat: now,
        sid: None,
        ver: None,
    };
    let key = EncodingKey::from_secret(secret.as_bytes());
    encode(&Header::default(), &claims, &key).map_err(|e| format!("JWT encode: {e}"))
}

/// Create an access token bound to one revocable server-side session.
pub fn create_session_access_token(
    account_id: i64,
    session_id: i64,
    auth_version: i64,
    secret: &str,
    ttl_secs: u64,
) -> Result<String, String> {
    let now = Utc::now().timestamp() as usize;
    let claims = JwtClaims {
        sub: account_id.to_string(),
        exp: now + ttl_secs as usize,
        iat: now,
        sid: Some(session_id.to_string()),
        ver: Some(auth_version),
    };
    let key = EncodingKey::from_secret(secret.as_bytes());
    encode(&Header::default(), &claims, &key).map_err(|error| format!("JWT encode: {error}"))
}

/// Verify an access token and return the account id (from `sub`).
pub fn verify_access_token(token: &str, secret: &str) -> Result<i64, String> {
    let mut v = Validation::new(jsonwebtoken::Algorithm::HS256);
    v.validate_exp = true;
    let key = DecodingKey::from_secret(secret.as_bytes());
    let data = decode::<JwtClaims>(token, &key, &v).map_err(|e| format!("JWT decode: {e}"))?;
    data.claims.sub.parse::<i64>().map_err(|e| format!("invalid sub claim: {e}"))
}

/// Generate a secure random refresh token returning (plaintext, SHA-256 hash).
pub fn generate_refresh_token() -> (String, String) {
    let rng = SystemRandom::new();
    let mut buf = [0u8; 32]; // 256 random bits
    rng.fill(&mut buf).expect("system CSPRNG must not fail");
    let plain = hex::encode(buf);
    let hash = hex::encode(Sha256::digest(plain.as_bytes()));
    (plain, hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    const SECRET: &str = "test-jwt-secret-for-unit-tests";

    #[test]
    fn jwt_round_trip() {
        let token = create_access_token(42, SECRET, 3600).expect("create token");
        let id = verify_access_token(&token, SECRET).expect("verify token");
        assert_eq!(id, 42);
    }

    #[test]
    fn session_access_token_carries_revocation_binding() {
        let token = create_session_access_token(42, 7, 3, SECRET, 3600)
            .expect("create session access token");
        let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
        validation.validate_exp = true;
        let key = DecodingKey::from_secret(SECRET.as_bytes());
        let claims = decode::<JwtClaims>(&token, &key, &validation)
            .expect("decode session access token")
            .claims;
        assert_eq!(claims.sub, "42");
        assert_eq!(claims.sid.as_deref(), Some("7"));
        assert_eq!(claims.ver, Some(3));
    }

    #[test]
    fn expired_token_rejected() {
        // Create a token that expired 2 hours ago to defeat the default 60s leeway.
        let past = Utc::now().timestamp() as usize - 7200;
        let claims = JwtClaims {
            sub: 42.to_string(),
            exp: past + 1, // expired 1 second after `past`
            iat: past,
            sid: None,
            ver: None,
        };
        let key = EncodingKey::from_secret(SECRET.as_bytes());
        let token = jsonwebtoken::encode(&Header::default(), &claims, &key).expect("encode");
        let result = verify_access_token(&token, SECRET);
        assert!(result.is_err(), "expired token should be rejected");
    }

    #[test]
    fn malformed_token_rejected() {
        let result = verify_access_token("not.a.jwt", SECRET);
        assert!(result.is_err());
    }

    #[test]
    fn refresh_tokens_are_unique() {
        let mut seen = HashSet::new();
        for _ in 0..1000 {
            let (plain, _hash) = generate_refresh_token();
            assert_eq!(plain.len(), 64, "refresh token should be 64 hex chars (256 bits)");
            assert!(seen.insert(plain), "unexpected collision in 1000 tokens");
        }
    }

    #[test]
    fn refresh_hash_is_deterministic() {
        let token = "deadbeef".repeat(8); // 64 hex chars
        let h1 = hex::encode(Sha256::digest(token.as_bytes()));
        let h2 = hex::encode(Sha256::digest(token.as_bytes()));
        assert_eq!(h1, h2);
    }
}
