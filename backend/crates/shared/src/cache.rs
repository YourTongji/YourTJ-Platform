//! Version-bump cache helpers built on Redis.
//!
//! Each cached object has a version key (`ver:{prefix}:{id}`) that is INCR'd
//! on every invalidation. The cache key itself is `{prefix}:{id}:v{version}`.
//! Old cache keys are not actively deleted — they naturally expire via TTL.
//! When Redis is unavailable every function returns an error and callers
//! degrade gracefully.

use anyhow::Result;
use deadpool_redis::Pool;

/// INCR the version key for a cached object. Old cache keys naturally expire
/// via TTL.
pub async fn bump_version(pool: &Pool, prefix: &str, id: &str) -> Result<i64> {
    let mut conn = pool.get().await?;
    let key = format!("ver:{prefix}:{id}");
    Ok(redis::cmd("INCR").arg(&key).query_async(&mut conn).await?)
}

/// Optional wrapper — no-op when Redis is unavailable.
pub async fn bump_version_opt(pool: Option<&Pool>, prefix: &str, id: &str) -> Result<()> {
    if let Some(p) = pool {
        bump_version(p, prefix, id).await?;
    }
    Ok(())
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
