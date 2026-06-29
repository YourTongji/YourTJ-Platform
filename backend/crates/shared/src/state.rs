//! Shared application state, constructed once at startup and passed to every
//! handler via Axum's `State` extractor. Cheap to clone — all heavy resources
//! are behind `Arc` or connection pools.

use sqlx::PgPool;

/// Application state available to all handlers via `State<AppState>`.
#[derive(Clone)]
pub struct AppState {
    /// Primary database connection pool.
    pub db: PgPool,

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
}
