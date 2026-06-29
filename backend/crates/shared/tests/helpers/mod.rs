//! Shared helpers for Redis integration tests.

use deadpool_redis::Pool;

/// Get a Redis pool from the REDIS_URL environment variable.
/// Returns None if REDIS_URL is not set or connection fails.
pub async fn try_connect() -> Option<Pool> {
    let url = std::env::var("REDIS_URL").ok()?;
    if url.is_empty() {
        return None;
    }

    let pool = deadpool_redis::Config::from_url(&url)
        .create_pool(Some(deadpool_redis::Runtime::Tokio1))
        .ok()?;

    // Verify we can actually talk to Redis.
    let mut conn = pool.get().await.ok()?;
    let _: String = redis::cmd("PING").query_async(&mut conn).await.ok()?;

    Some(pool)
}
