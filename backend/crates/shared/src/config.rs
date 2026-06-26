//! Runtime configuration, loaded once at startup from the environment.

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
