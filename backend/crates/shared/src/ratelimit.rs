//! Redis-backed token-bucket rate limiter for the platform.
//!
//! Uses INCR + EXPIRE to implement a simple sliding window. A configured Redis
//! outage fails closed with a stable 503 so distributed abuse controls cannot
//! be bypassed by taking down the shared limiter.

use deadpool_redis::Pool;

use crate::AppError;

/// Check a simple INCR+EXPIRE token bucket.
pub async fn check_token_bucket(
    pool: Option<&Pool>,
    bucket: &str,
    id: &str,
    max_tokens: u64,
    window_secs: u64,
) -> Result<(), AppError> {
    let p = match pool {
        Some(p) => p,
        None => return Ok(()),
    };
    let mut conn = p.get().await.map_err(|error| {
        tracing::warn!(bucket, ?error, "rate-limit Redis connection unavailable");
        AppError::ServiceUnavailable
    })?;
    let key = format!("rl:{bucket}:{id}");
    let current: u64 =
        redis::cmd("INCR").arg(&key).query_async(&mut conn).await.map_err(|error| {
            tracing::warn!(bucket, ?error, "rate-limit Redis increment failed");
            AppError::ServiceUnavailable
        })?;
    if current == 1 {
        let _: () =
            redis::cmd("EXPIRE").arg(&key).arg(window_secs).query_async(&mut conn).await.map_err(
                |error| {
                    tracing::warn!(bucket, ?error, "rate-limit Redis expiry failed");
                    AppError::ServiceUnavailable
                },
            )?;
    }
    if current > max_tokens {
        return Err(AppError::RateLimited);
    }
    Ok(())
}
