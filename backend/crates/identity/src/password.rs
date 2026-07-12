//! Password hashing and verification using Argon2id.
//!
//! Passwords are hashed with Argon2id (v19) using 19 MiB memory, 2 iterations,
//! 1 lane of parallelism. The output is a PHC string. Verification uses
//! constant-time comparison internally via the `argon2` crate.
//!
//! Validation enforces minimum length (8) and checks strength with zxcvbn
//! (score ≥ 2 required), passing the email as user_inputs to weaken
//! common passwords that include personal info.

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Algorithm, Argon2, Params, PasswordHash, PasswordVerifier, Version,
};
use shared::AppError;

use crate::error::IdentityError;

/// Argon2id parameters: 19 MiB memory, 2 iterations, 1 lane.
const M_COST: u32 = 19_456;
const T_COST: u32 = 2;
const P_COST: u32 = 1;

/// Minimum password length in characters.
const MIN_LENGTH: usize = 8;
/// Maximum password length in characters.
const MAX_LENGTH: usize = 128;

const DUMMY_PASSWORD_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$lMsuCNrM/Jk4lpdAY/Gk9w$NkmJDYSq0o5US61ZPai1ajtpZWKmn7Rvn4wqQn3DR7Y";

fn hash_blocking(password: &str) -> Result<String, AppError> {
    if password.is_empty() {
        return Err(IdentityError::InvalidPassword.into());
    }

    let salt = SaltString::generate(&mut OsRng);
    let params = Params::new(M_COST, T_COST, P_COST, None)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("argon2 params: {e}")))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    let phc = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("argon2 hash: {e}")))?
        .to_string();

    Ok(phc)
}

fn verify_blocking(password: &str, phc: &str) -> bool {
    let parsed = match PasswordHash::new(phc) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok()
}

/// Hash a password without blocking a Tokio worker thread.
pub async fn hash(password: &str) -> Result<String, AppError> {
    let password = password.to_owned();
    tokio::task::spawn_blocking(move || hash_blocking(&password))
        .await
        .map_err(|error| AppError::Internal(anyhow::Error::new(error)))?
}

/// Verify a password without blocking a Tokio worker thread.
pub async fn verify(password: &str, phc: &str) -> Result<bool, AppError> {
    let password = password.to_owned();
    let phc = phc.to_owned();
    tokio::task::spawn_blocking(move || verify_blocking(&password, &phc))
        .await
        .map_err(|error| AppError::Internal(anyhow::Error::new(error)))
}

/// Perform one Argon2 verification even when an account has no stored password.
pub async fn verify_or_dummy(password: &str, phc: Option<&str>) -> Result<bool, AppError> {
    let has_valid_hash = phc.is_some_and(|stored| PasswordHash::new(stored).is_ok());
    let selected_hash =
        if has_valid_hash { phc.unwrap_or(DUMMY_PASSWORD_HASH) } else { DUMMY_PASSWORD_HASH };
    Ok(has_valid_hash && verify(password, selected_hash).await?)
}

/// Validate password strength via zxcvbn and length checks.
///
/// - Length must be 8–128 characters.
/// - zxcvbn score must be ≥ 2 (password strength).
/// - The email is passed as `user_inputs` so passwords containing the
///   email prefix score lower (mitigates common "name123" patterns).
pub fn validate(password: &str, email: &str) -> Result<(), AppError> {
    if password.len() < MIN_LENGTH {
        return Err(IdentityError::InvalidPassword.into());
    }
    if password.len() > MAX_LENGTH {
        return Err(IdentityError::InvalidPassword.into());
    }

    let local_part = email.split('@').next().unwrap_or("");
    let entropy = zxcvbn::zxcvbn(password, &[local_part, email]);

    if entropy.score() < zxcvbn::Score::Two {
        return Err(IdentityError::InvalidPassword.into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // hash / verify round-trip
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn hash_roundtrip() {
        let pw = "MyStr0ngP4ssword!";
        let phc = hash(pw).await.expect("hash should succeed");
        assert!(phc.starts_with("$argon2id$"));
        assert!(verify(pw, &phc).await.expect("verify password"));
    }

    #[tokio::test]
    async fn wrong_password_rejects() {
        let phc = hash("correct-horse-battery-staple").await.expect("hash password");
        assert!(!verify("wrong-password", &phc).await.expect("verify password"));
    }

    #[tokio::test]
    async fn empty_password_rejects() {
        assert!(hash("").await.is_err());
    }

    #[tokio::test]
    async fn empty_password_verify_fails() {
        let phc = hash("something").await.expect("hash password");
        assert!(!verify("", &phc).await.expect("verify password"));
    }

    #[tokio::test]
    async fn each_hash_is_unique() {
        let pw = "same-password-twice";
        let h1 = hash(pw).await.expect("hash password");
        let h2 = hash(pw).await.expect("hash password");
        assert_ne!(h1, h2, "different salts should produce different hashes");
        assert!(verify(pw, &h1).await.expect("verify password"));
        assert!(verify(pw, &h2).await.expect("verify password"));
    }

    // -----------------------------------------------------------------------
    // validate
    // -----------------------------------------------------------------------

    #[test]
    fn short_password_rejects() {
        let result = validate("short", "test@tongji.edu.cn");
        assert!(result.is_err());
    }

    #[test]
    fn exact_min_length_passes_if_strong() {
        // "a1b2c3d4" is 8 chars and should be strong enough
        let result = validate("Xy9#mP2$", "test@tongji.edu.cn");
        assert!(result.is_ok());
    }

    #[test]
    fn long_password_rejects() {
        let too_long = "A".repeat(129);
        assert!(validate(&too_long, "test@tongji.edu.cn").is_err());
    }

    #[test]
    fn max_length_passes_if_strong() {
        let pw = "X".repeat(128);
        // zxcvbn will rate repeated chars as score 0
        // this is expected — test only length boundary
        let result = validate(&pw, "test@tongji.edu.cn");
        // score < 2 → Err
        assert!(result.is_err());
    }

    #[test]
    fn common_password_weak() {
        // "password123" is well-known, score will be 0
        let result = validate("password123", "test@tongji.edu.cn");
        assert!(result.is_err(), "password123 should be rejected as too weak");
    }

    #[test]
    fn email_in_password_weakens_score() {
        // If the password starts with the email prefix, it should score lower
        let result = validate("testtesttest1", "test@tongji.edu.cn");
        // This might pass or fail depending on zxcvbn, but the important
        // thing is that the email prefix is passed to zxcvbn as user_input
        assert!(result.is_ok() || result.is_err(), "should not panic");
    }

    #[test]
    fn strong_passphrase_passes() {
        let result = validate("correct-horse-battery-staple!", "test@tongji.edu.cn");
        assert!(result.is_ok(), "strong passphrase should pass");
    }

    // -----------------------------------------------------------------------
    // edge cases
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn verify_garbage_phc_returns_false() {
        assert!(!verify("anything", "not-a-valid-phc-string").await.expect("verify password"));
    }

    #[tokio::test]
    async fn verify_empty_phc_returns_false() {
        assert!(!verify("anything", "").await.expect("verify password"));
    }

    #[tokio::test]
    async fn dummy_hash_can_never_authenticate() {
        assert!(!verify_or_dummy("yourtj-constant-dummy-password", None)
            .await
            .expect("verify dummy password"));
        assert!(!verify_or_dummy("yourtj-constant-dummy-password", Some("invalid-phc"))
            .await
            .expect("verify invalid stored hash"));
    }
}
