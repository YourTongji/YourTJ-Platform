//! Version-bump cache helpers built on Redis.
//!
//! Each cached object has a version key (`ver:{prefix}:{id}`) that is INCR'd
//! on every invalidation. The cache key itself is `{prefix}:{id}:v{version}`.
//! Old cache keys are not actively deleted — they naturally expire via TTL.
//! When Redis is unavailable every function returns an error and callers
//! degrade gracefully.

use anyhow::Result;
use deadpool_redis::Pool;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::AppError;

/// INCR the version key for a cached object. Old cache keys naturally expire
/// via TTL.
pub async fn bump_version(pool: &Pool, prefix: &str, id: &str) -> Result<i64> {
    let mut conn = pool.get().await?;
    let key = format!("ver:{prefix}:{id}");
    Ok(redis::cmd("INCR").arg(&key).query_async(&mut conn).await?)
}

/// Bump version and ignore errors entirely — no return value.
pub async fn bump_version_silent(pool: Option<&Pool>, prefix: &str, id: &str) {
    if let Some(p) = pool {
        let _ = bump_version_opt(Some(p), prefix, id).await;
    }
}

/// Optional wrapper — no-op when Redis is unavailable.
pub async fn bump_version_opt(pool: Option<&Pool>, prefix: &str, id: &str) -> Result<()> {
    if let Some(p) = pool {
        bump_version(p, prefix, id).await?;
    }
    Ok(())
}

/// Read the current namespace version, returning zero when Redis is absent or unavailable.
pub async fn current_version_opt(pool: Option<&Pool>, prefix: &str, id: &str) -> i64 {
    let Some(pool) = pool else {
        return 0;
    };
    let Ok(mut conn) = pool.get().await else {
        return 0;
    };
    let key = format!("ver:{prefix}:{id}");
    redis::cmd("GET")
        .arg(key)
        .query_async::<Option<i64>>(&mut conn)
        .await
        .ok()
        .flatten()
        .unwrap_or(0)
}

/// Read a cached value by version. Returns `None` on miss.
pub async fn get_cached(pool: &Pool, prefix: &str, id: &str) -> Result<Option<String>> {
    let mut conn = pool.get().await?;
    let ver_key = format!("ver:{prefix}:{id}");
    let version: Option<i64> = redis::cmd("GET").arg(&ver_key).query_async(&mut conn).await?;
    let v = version.unwrap_or(0);
    let cache_key = format!("{prefix}:{id}:v{v}");
    let result: Option<String> = redis::cmd("GET").arg(&cache_key).query_async(&mut conn).await?;
    Ok(result)
}

/// Write a value with version-bumped TTL.
pub async fn set_cached(
    pool: &Pool,
    prefix: &str,
    id: &str,
    value: &str,
    ttl_secs: u64,
) -> Result<()> {
    let mut conn = pool.get().await?;
    let ver_key = format!("ver:{prefix}:{id}");
    let version: i64 = redis::cmd("INCR").arg(&ver_key).query_async(&mut conn).await?;
    let cache_key = format!("{prefix}:{id}:v{version}");
    redis::cmd("SETEX")
        .arg(&cache_key)
        .arg(ttl_secs)
        .arg(value)
        .query_async::<()>(&mut conn)
        .await?;
    Ok(())
}

/// Generic cached JSON wrapper: read from Redis cache, or call `fetch` to
/// populate the cache on miss.
pub async fn cached_json<T, E>(
    redis: Option<&Pool>,
    prefix: &str,
    id: &str,
    ttl_secs: u64,
    fetch: impl std::future::Future<Output = Result<T, E>>,
) -> Result<T, AppError>
where
    T: Serialize + DeserializeOwned,
    E: Into<AppError>,
{
    if let Some(pool) = redis {
        if let Ok(Some(cached)) = get_cached(pool, prefix, id).await {
            if let Ok(val) = serde_json::from_str::<T>(&cached) {
                return Ok(val);
            }
        }
    }
    let val = fetch.await.map_err(Into::into)?;
    if let Some(pool) = redis {
        if let Ok(json) = serde_json::to_string(&val) {
            let _ = set_cached(pool, prefix, id, &json, ttl_secs).await;
        }
    }
    Ok(val)
}
