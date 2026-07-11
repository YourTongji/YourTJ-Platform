//! Shared application state, constructed once at startup and passed to every
//! handler via Axum's `State` extractor. Cheap to clone — all heavy resources
//! are behind `Arc` or connection pools.

use sqlx::PgPool;
use tokio::sync::broadcast;

use crate::email_crypto::EmailEncryption;
use crate::sse::SsePayload;
use crate::Config;

/// Application state available to all handlers via `State<AppState>`.
#[derive(Clone)]
pub struct AppState {
    /// Primary database connection pool.
    pub db: PgPool,

    /// Full application config (SMTP, etc.).
    pub config: Config,

    /// HS256 JWT secret loaded from the environment.
    pub jwt_secret: String,

    /// Access token lifetime in seconds (default: 900 = 15 minutes).
    pub jwt_ttl: u64,

    /// Refresh token lifetime in seconds (default: 604800 = 7 days).
    pub refresh_ttl: u64,

    /// Meilisearch server URL (e.g. http://localhost:7700).
    pub meili_url: String,

    /// Meilisearch master key for index management and search.
    pub meili_master_key: String,

    /// Redis connection pool. `None` when Redis is unavailable — caching
    /// and rate limiting gracefully degrade without it.
    pub redis: Option<deadpool_redis::Pool>,

    /// System Ed25519 private key seed (32 bytes, hex-decoded from env).
    pub system_private_key: Vec<u8>,

    /// System Ed25519 public key (base64-encoded).
    pub system_public_key_b64: String,

    /// Email encryption for PII-at-rest (None when not configured).
    pub email_encryption: Option<EmailEncryption>,

    /// SSE broadcast sender for real-time notification delivery.
    /// `None` when SSE is not configured / disabled.
    pub sse_tx: Option<broadcast::Sender<SsePayload>>,
}
