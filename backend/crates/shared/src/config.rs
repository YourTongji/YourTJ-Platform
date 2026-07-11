//! Runtime configuration, loaded once at startup from the environment.

const DEFAULT_CAPTCHA_SITEVERIFY_URL: &str = "https://captcha.07211024.xyz/api/siteverify";
const DEFAULT_CLOUDFLARE_EMAIL_API_BASE_URL: &str = "https://api.cloudflare.com/client/v4";

/// Configured outbound email transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmailProvider {
    /// Development sink that records only redacted delivery metadata.
    Log,
    /// Cloudflare Email Sending REST API.
    Cloudflare,
    /// Legacy SMTP transport.
    Smtp,
}

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
    pub email_provider: EmailProvider,
    pub email_from: String,
    pub cloudflare_email_account_id: String,
    pub cloudflare_email_api_token: String,
    pub cloudflare_email_api_base_url: String,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub oss_region: String,
    pub oss_bucket: String,
    pub oss_access_key_id: String,
    pub oss_access_key_secret: String,
    pub oss_role_arn: String,
    pub oss_callback_base_url: String,
    pub media_retention_gc_enabled: bool,
    pub media_operations_history_purge_enabled: bool,
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
        let cloudflare_email_account_id = env_or_default("CLOUDFLARE_EMAIL_ACCOUNT_ID", "");
        let cloudflare_email_api_token = env_or_default("CLOUDFLARE_EMAIL_API_TOKEN", "");
        let smtp_host = env_or_default("SMTP_HOST", "");
        let email_provider = email_provider_from_env(
            &cloudflare_email_account_id,
            &cloudflare_email_api_token,
            &smtp_host,
        )?;
        let email_from = non_empty(std::env::var("EMAIL_FROM").ok())
            .or_else(|| non_empty(std::env::var("SMTP_FROM").ok()))
            .unwrap_or_default();

        let config = Self {
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
            email_provider,
            email_from,
            cloudflare_email_account_id,
            cloudflare_email_api_token,
            cloudflare_email_api_base_url: env_or_default(
                "CLOUDFLARE_EMAIL_API_BASE_URL",
                DEFAULT_CLOUDFLARE_EMAIL_API_BASE_URL,
            ),
            smtp_host,
            smtp_port: env_or_default_u64("SMTP_PORT", 587) as u16,
            smtp_username: env_or_default("SMTP_USERNAME", ""),
            smtp_password: env_or_default("SMTP_PASSWORD", ""),
            oss_region: env_or_default("OSS_REGION", ""),
            oss_bucket: env_or_default("OSS_BUCKET", ""),
            oss_access_key_id: env_or_default("OSS_ACCESS_KEY_ID", ""),
            oss_access_key_secret: env_or_default("OSS_ACCESS_KEY_SECRET", ""),
            oss_role_arn: env_or_default("OSS_ROLE_ARN", ""),
            oss_callback_base_url: env_or_default("OSS_CALLBACK_BASE_URL", ""),
            media_retention_gc_enabled: env_or_default_bool("MEDIA_RETENTION_GC_ENABLED", false)?,
            media_operations_history_purge_enabled: env_or_default_bool(
                "MEDIA_OPERATIONS_HISTORY_PURGE_ENABLED",
                false,
            )?,
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
        };
        config.validate_email_delivery()?;
        Ok(config)
    }

    /// Returns the read-replica DSN when configured, otherwise the primary.
    pub fn read_url(&self) -> &str {
        self.database_replica_url.as_deref().unwrap_or(&self.database_url)
    }

    fn validate_email_delivery(&self) -> anyhow::Result<()> {
        if self.email_provider == EmailProvider::Log {
            return Ok(());
        }
        if self.email_from.parse::<lettre::message::Mailbox>().is_err() {
            anyhow::bail!("EMAIL_FROM must be a valid mailbox for outbound email");
        }
        match self.email_provider {
            EmailProvider::Cloudflare => {
                if self.cloudflare_email_account_id.len() != 32
                    || !self
                        .cloudflare_email_account_id
                        .chars()
                        .all(|character| character.is_ascii_hexdigit())
                {
                    anyhow::bail!(
                        "CLOUDFLARE_EMAIL_ACCOUNT_ID must be a 32-character hexadecimal account id"
                    );
                }
                if self.cloudflare_email_api_token.len() < 16 {
                    anyhow::bail!("CLOUDFLARE_EMAIL_API_TOKEN is missing or malformed");
                }
                if !self.cloudflare_email_api_base_url.starts_with("https://")
                    && !self.cloudflare_email_api_base_url.starts_with("http://127.0.0.1")
                    && !self.cloudflare_email_api_base_url.starts_with("http://localhost")
                {
                    anyhow::bail!(
                        "CLOUDFLARE_EMAIL_API_BASE_URL must use HTTPS outside loopback tests"
                    );
                }
            }
            EmailProvider::Smtp => {
                if self.smtp_host.trim().is_empty() {
                    anyhow::bail!("SMTP_HOST is required when EMAIL_PROVIDER=smtp");
                }
                if self.smtp_username.is_empty() != self.smtp_password.is_empty() {
                    anyhow::bail!("SMTP_USERNAME and SMTP_PASSWORD must be configured together");
                }
            }
            EmailProvider::Log => {}
        }
        Ok(())
    }
}

fn email_provider_from_env(
    cloudflare_account_id: &str,
    cloudflare_api_token: &str,
    smtp_host: &str,
) -> anyhow::Result<EmailProvider> {
    let configured = std::env::var("EMAIL_PROVIDER").ok().map(|value| value.to_lowercase());
    match configured.as_deref() {
        Some("cloudflare") => Ok(EmailProvider::Cloudflare),
        Some("smtp") => Ok(EmailProvider::Smtp),
        Some("log" | "disabled") => Ok(EmailProvider::Log),
        Some(_) => anyhow::bail!("EMAIL_PROVIDER must be cloudflare, smtp, or log"),
        None if !cloudflare_account_id.is_empty() || !cloudflare_api_token.is_empty() => {
            Ok(EmailProvider::Cloudflare)
        }
        None if !smtp_host.is_empty() => Ok(EmailProvider::Smtp),
        None => Ok(EmailProvider::Log),
    }
}

fn env_or_default(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_or_default_u64(key: &str, default: u64) -> u64 {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn env_or_default_bool(key: &str, default: bool) -> anyhow::Result<bool> {
    match std::env::var(key) {
        Ok(value) if matches!(value.to_ascii_lowercase().as_str(), "1" | "true") => Ok(true),
        Ok(value) if matches!(value.to_ascii_lowercase().as_str(), "0" | "false") => Ok(false),
        Ok(value) => anyhow::bail!("invalid {key}: {value}"),
        Err(_) => Ok(default),
    }
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|v| !v.trim().is_empty())
}
