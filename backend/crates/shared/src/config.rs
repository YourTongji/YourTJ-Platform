//! Runtime configuration, loaded once at startup from the environment.

const DEFAULT_CAPTCHA_SITEVERIFY_URL: &str = "https://captcha.07211024.xyz/api/siteverify";

/// Application configuration. Construct with [`Config::from_env`].
#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub database_url: String,
    pub database_replica_url: Option<String>,
    pub redis_url: String,
    pub meili_url: String,
    pub meili_master_key: String,
    pub jwt_secret: String,
    pub jwt_ttl: u64,
    pub refresh_ttl: u64,
    pub credit_system_private_key: String,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub smtp_from: String,
    pub oss_region: String,
    pub oss_bucket: String,
    pub oss_access_key_id: String,
    pub oss_access_key_secret: String,
    pub oss_role_arn: String,
    pub oss_callback_base_url: String,
    pub email_encryption_active_version: u8,
    pub email_encryption_active_aead_hex: String,
    pub email_encryption_active_blind_hex: String,
    pub email_encryption_strict: bool,
    pub captcha_siteverify_url: String,
}

impl Config {
    /// Load configuration from environment variables, applying sane defaults for
    /// local development. Returns an error only for malformed values.
    pub fn from_env() -> anyhow::Result<Self> {
        let port = match std::env::var("PORT") {
            Ok(v) => v.parse().map_err(|_| anyhow::anyhow!("invalid PORT: {v}"))?,
            Err(_) => 8080,
        };

        Ok(Self {
            port,
            database_url: env_or_default("DATABASE_URL", ""),
            database_replica_url: non_empty(std::env::var("DATABASE_REPLICA_URL").ok()),
            redis_url: env_or_default("REDIS_URL", "redis://localhost:6379"),
            meili_url: env_or_default("MEILI_URL", "http://localhost:7700"),
            meili_master_key: env_or_default("MEILI_MASTER_KEY", ""),
            jwt_secret: env_or_default("JWT_SECRET", ""),
            jwt_ttl: env_or_default_u64("JWT_TTL", 900),
            refresh_ttl: env_or_default_u64("REFRESH_TTL", 604800),
            credit_system_private_key: env_or_default("CREDIT_SYSTEM_PRIVATE_KEY", ""),
            smtp_host: env_or_default("SMTP_HOST", ""),
            smtp_port: env_or_default_u64("SMTP_PORT", 587) as u16,
            smtp_username: env_or_default("SMTP_USERNAME", ""),
            smtp_password: env_or_default("SMTP_PASSWORD", ""),
            smtp_from: env_or_default("SMTP_FROM", ""),
            oss_region: env_or_default("OSS_REGION", ""),
            oss_bucket: env_or_default("OSS_BUCKET", ""),
            oss_access_key_id: env_or_default("OSS_ACCESS_KEY_ID", ""),
            oss_access_key_secret: env_or_default("OSS_ACCESS_KEY_SECRET", ""),
            oss_role_arn: env_or_default("OSS_ROLE_ARN", ""),
            oss_callback_base_url: env_or_default("OSS_CALLBACK_BASE_URL", ""),
            email_encryption_active_version: env_or_default_u64(
                "EMAIL_ENCRYPTION_ACTIVE_VERSION",
                0,
            ) as u8,
            email_encryption_active_aead_hex: env_or_default("EMAIL_ENCRYPTION_ACTIVE_AEAD", ""),
            email_encryption_active_blind_hex: env_or_default("EMAIL_ENCRYPTION_ACTIVE_BLIND", ""),
            email_encryption_strict: std::env::var("EMAIL_ENCRYPTION_STRICT")
                .map(|v| v == "1" || v == "true")
                .unwrap_or(false),
            captcha_siteverify_url: env_or_default(
                "CAPTCHA_SITEVERIFY_URL",
                DEFAULT_CAPTCHA_SITEVERIFY_URL,
            ),
        })
    }

    /// Returns the read-replica DSN when configured, otherwise the primary.
    pub fn read_url(&self) -> &str {
        self.database_replica_url.as_deref().unwrap_or(&self.database_url)
    }
}

fn env_or_default(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_or_default_u64(key: &str, default: u64) -> u64 {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|v| !v.trim().is_empty())
}
