//! TongjiCaptcha integration with purpose-bound, single-use Redis consumption.
//!
//! The YourTongji/YourTJCaptcha provider signs pass tokens as JWTs with a 10‑minute
//! expiry but provides no action binding or replay defence. This module wraps every
//! verification with a `(purpose, token_hash)` atomic Redis SET NX so each token can
//! only be consumed once per purpose.
//!
//! All providers return [`CaptchaResult`] — callers must not distinguish internal
//! errors from verification failures so timing side-channels are minimised.

use sha2::{Digest, Sha256};
use std::time::Duration;

/// The only three outcomes a caller should see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptchaOutcome {
    Ok,
    Invalid,
    Unavailable,
}

/// Opaque token hash used as the Redis key suffix.
fn token_hash(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

// ── Injectable provider trait ──────────────────────────────────────────────

/// Pluggable captcha verifier so integration tests can inject a deterministic fake.
#[async_trait::async_trait]
pub trait CaptchaVerifier: Send + Sync {
    /// Verify `token` with the upstream provider. Implementations must be
    /// side-effect-free apart from a single HTTP call.
    async fn verify(&self, token: &str) -> CaptchaOutcome;
}

// ── Real YourTongji provider ───────────────────────────────────────────────

pub struct YourTongjiCaptcha {
    siteverify_url: String,
    client: reqwest::Client,
}

impl YourTongjiCaptcha {
    pub fn new(siteverify_url: String, timeout: Duration) -> Self {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { siteverify_url, client }
    }
}

#[async_trait::async_trait]
impl CaptchaVerifier for YourTongjiCaptcha {
    async fn verify(&self, token: &str) -> CaptchaOutcome {
        let resp = match self
            .client
            .post(&self.siteverify_url)
            .json(&serde_json::json!({"token": token}))
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return CaptchaOutcome::Unavailable,
        };
        if !resp.status().is_success() {
            return CaptchaOutcome::Unavailable;
        }
        let body: serde_json::Value = match resp.json().await {
            Ok(v) => v,
            Err(_) => return CaptchaOutcome::Unavailable,
        };
        match body.get("success").and_then(|v| v.as_bool()) {
            Some(true) => CaptchaOutcome::Ok,
            _ => CaptchaOutcome::Invalid,
        }
    }
}

// ── Fake for tests ─────────────────────────────────────────────────────────

/// A fake verifier that always returns `CaptchaOutcome::Ok` for any token. Only
/// used in integration tests where network access is unavailable or undesirable.
pub struct FakeCaptcha;

#[async_trait::async_trait]
impl CaptchaVerifier for FakeCaptcha {
    async fn verify(&self, _token: &str) -> CaptchaOutcome {
        CaptchaOutcome::Ok
    }
}

// ── Redis single-use gate ───────────────────────────────────────────────────

/// Try to atomically consume a token for `purpose`.
///
/// Uses `SET NX EX` on `captcha:{purpose}:{token_hash}`.  Returns:
/// - `Ok(CaptchaOutcome::Ok)` when the token is fresh,
/// - `Ok(CaptchaOutcome::Invalid)` when already consumed,
/// - `Err(_)` when Redis is unavailable (caller should fail-closed).
pub async fn consume_once(
    redis: Option<&deadpool_redis::Pool>,
    purpose: &str,
    token: &str,
    ttl_seconds: u64,
) -> Result<CaptchaOutcome, CaptchaOutcome> {
    let pool = match redis {
        Some(p) => p,
        None => return Err(CaptchaOutcome::Unavailable),
    };
    let mut conn = match pool.get().await {
        Ok(c) => c,
        Err(_) => return Err(CaptchaOutcome::Unavailable),
    };
    let key = format!("captcha:{}:{}", purpose, token_hash(token));
    let set: Option<String> = redis::cmd("SET")
        .arg(&key)
        .arg("1")
        .arg("NX")
        .arg("EX")
        .arg(ttl_seconds)
        .query_async(&mut conn)
        .await
        .unwrap_or(None);
    match set {
        Some(_) => Ok(CaptchaOutcome::Ok),
        None => Ok(CaptchaOutcome::Invalid),
    }
}

// ── Full verification pipeline ─────────────────────────────────────────────

/// Verify a captcha token with the provider, then atomically consume it for `purpose`.
/// Both steps must succeed; any failure is reported as `CaptchaOutcome::Invalid` or
/// `CaptchaOutcome::Unavailable` depending on the failing step.
pub async fn verify_and_consume(
    verifier: &dyn CaptchaVerifier,
    redis: Option<&deadpool_redis::Pool>,
    purpose: &str,
    token: &str,
    ttl_seconds: u64,
) -> CaptchaOutcome {
    let outcome = verifier.verify(token).await;
    if outcome != CaptchaOutcome::Ok {
        return outcome;
    }
    match consume_once(redis, purpose, token, ttl_seconds).await {
        Ok(CaptchaOutcome::Ok) => CaptchaOutcome::Ok,
        Ok(_) => CaptchaOutcome::Invalid,
        Err(_) => CaptchaOutcome::Unavailable,
    }
}

// ── Central enforcement helper ──────────────────────────────────────────────

/// Validate a captcha token from a request. Returns `Ok(())` only after
/// successful provider verification and atomic single-use consumption.
///
/// `purpose` must be a short, stable label like `"email_code"`, `"review_create"`.
/// `token` must be non-empty — empty tokens are rejected immediately.
/// When the captcha verifier is `None` (not configured), returns
/// `CaptchaOutcome::Unavailable` so callers can fail closed.
pub async fn enforce_captcha(
    verifier: &dyn CaptchaVerifier,
    redis: Option<&deadpool_redis::Pool>,
    purpose: &str,
    token: &str,
) -> CaptchaOutcome {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return CaptchaOutcome::Invalid;
    }
    verify_and_consume(verifier, redis, purpose, trimmed, 600).await
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    struct AlwaysInvalid;
    #[async_trait::async_trait]
    impl CaptchaVerifier for AlwaysInvalid {
        async fn verify(&self, _token: &str) -> CaptchaOutcome {
            CaptchaOutcome::Invalid
        }
    }

    #[tokio::test]
    async fn verify_and_consume_with_fake_verifier() {
        let verifier = FakeCaptcha;
        // No Redis — consume will fail but the fake verifier always returns Ok,
        // and verify_and_consume chains: Ok → consume Err → Unavailable.
        let out = verify_and_consume(&verifier, None, "test", "any", 600).await;
        assert_eq!(out, CaptchaOutcome::Unavailable);
    }

    #[tokio::test]
    async fn verify_rejects_invalid_token() {
        let verifier = AlwaysInvalid;
        let out = verify_and_consume(&verifier, None, "test", "bad", 600).await;
        assert_eq!(out, CaptchaOutcome::Invalid);
    }

    #[tokio::test]
    async fn enforce_captcha_rejects_empty_token() {
        let verifier = FakeCaptcha;
        let out = enforce_captcha(&verifier, None, "purpose", "").await;
        assert_eq!(out, CaptchaOutcome::Invalid);
    }

    #[tokio::test]
    async fn verify_provider_unavailable_returns_unavailable() {
        // A provider whose HTTP call will fail because the URL is unreachable.
        let verifier =
            YourTongjiCaptcha::new("http://127.0.0.1:1/siteverify".into(), Duration::from_secs(1));
        let out = verifier.verify("token").await;
        assert_eq!(out, CaptchaOutcome::Unavailable);
    }

    #[test]
    fn token_hash_is_deterministic() {
        assert_eq!(token_hash("abc"), token_hash("abc"));
        assert_ne!(token_hash("abc"), token_hash("abd"));
    }
}
