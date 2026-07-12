//! Opaque subjects for identity rate-limit keys.

use hmac::{Hmac, Mac as _};
use sha2::Sha256;
use shared::email_crypto::EmailEncryption;
use shared::{AppError, AppResult};

type HmacSha256 = Hmac<Sha256>;

const EMAIL_RATE_LIMIT_DOMAIN: &[u8] = b"yourtj:identity:email-rate-limit:v1\0";
const NETWORK_RATE_LIMIT_DOMAIN: &[u8] = b"yourtj:identity:network-rate-limit:v1\0";

/// Derive a deterministic rate-limit subject without placing an email address in Redis.
pub(crate) fn email_rate_limit_subject(
    encryption: Option<&EmailEncryption>,
    fallback_secret: &str,
    email: &str,
) -> AppResult<String> {
    if let Some(encryption) = encryption {
        return Ok(encryption.blind_index(email));
    }

    let mut mac = HmacSha256::new_from_slice(fallback_secret.as_bytes()).map_err(|_| {
        AppError::Internal(anyhow::anyhow!("identity rate-limit HMAC initialization failed"))
    })?;
    mac.update(EMAIL_RATE_LIMIT_DOMAIN);
    mac.update(email.trim().to_ascii_lowercase().as_bytes());
    Ok(hex::encode(mac.finalize().into_bytes()))
}

/// Derive an opaque subject for a client-network or global abuse-control bucket.
pub(crate) fn network_rate_limit_subject(
    fallback_secret: &str,
    namespace: &str,
    value: &str,
) -> AppResult<String> {
    let mut mac = HmacSha256::new_from_slice(fallback_secret.as_bytes()).map_err(|_| {
        AppError::Internal(anyhow::anyhow!(
            "identity network rate-limit HMAC initialization failed"
        ))
    })?;
    mac.update(NETWORK_RATE_LIMIT_DOMAIN);
    mac.update(namespace.as_bytes());
    mac.update(&[0]);
    mac.update(value.trim().as_bytes());
    Ok(hex::encode(mac.finalize().into_bytes()))
}

#[cfg(test)]
mod tests {
    use super::{email_rate_limit_subject, network_rate_limit_subject};
    use shared::email_crypto::EmailEncryption;

    const EMAIL: &str = "student@tongji.edu.cn";

    #[test]
    fn fallback_subject_is_deterministic_normalized_and_opaque() {
        let first = email_rate_limit_subject(None, "test-jwt-secret", EMAIL)
            .expect("fallback subject is derived");
        let normalized =
            email_rate_limit_subject(None, "test-jwt-secret", "  STUDENT@TONGJI.EDU.CN  ")
                .expect("normalized fallback subject is derived");

        assert_eq!(first, normalized);
        assert_eq!(first.len(), 64);
        assert!(!first.contains("student"));
        assert!(!first.contains("tongji"));
        assert_ne!(
            first,
            email_rate_limit_subject(None, "different-secret", EMAIL)
                .expect("rotated fallback subject is derived")
        );
    }

    #[test]
    fn configured_encryption_reuses_the_active_blind_index() {
        let encryption = EmailEncryption::from_keys(
            1,
            "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
            "101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f",
            &[],
        )
        .expect("test keys are valid")
        .expect("test encryption is configured");

        let subject = email_rate_limit_subject(Some(&encryption), "unused", EMAIL)
            .expect("blind-index subject is derived");

        assert_eq!(subject, encryption.blind_index(EMAIL));
        assert_eq!(subject.len(), 64);
        assert!(!subject.contains("student"));
        assert!(!subject.contains("tongji"));
    }

    #[test]
    fn network_subject_is_namespace_bound_and_opaque() {
        let first = network_rate_limit_subject("test-secret", "password", "203.0.113.5")
            .expect("network subject");
        let second = network_rate_limit_subject("test-secret", "email-code", "203.0.113.5")
            .expect("namespaced network subject");

        assert_eq!(first.len(), 64);
        assert!(!first.contains("203.0.113.5"));
        assert_ne!(first, second);
    }
}
