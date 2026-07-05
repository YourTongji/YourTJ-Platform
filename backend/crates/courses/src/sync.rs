//! Selection data sync pipeline — materialize pk_* → selection.*, reindex
//! Meilisearch, and bump cache versions.
//!
//! Called from the admin sync endpoint as a fire-and-forget background task.

use anyhow::Context;
use deadpool_redis::Pool as RedisPool;
use sqlx::PgPool;

/// Execute the full selection sync pipeline.
///
/// Steps:
/// 1. Materialize selection.* tables from pk_* data (idempotent)
/// 2. Reindex Meilisearch selection_courses index
/// 3. Bump Redis cache versions for selection endpoints
pub async fn run_selection_sync(
    pool: &PgPool,
    meili_url: &str,
    meili_key: &str,
    redis: Option<&RedisPool>,
) -> anyhow::Result<()> {
    // Step 1: Materialize selection tables from pk_* data
    tracing::info!("selection sync: starting materialize step");
    let sql = include_str!("../../../ops/materialize_selection.sql");
    sqlx::raw_sql(sql).execute(pool).await.context("materialize selection SQL failed")?;
    tracing::info!("selection sync: materialize complete");

    // Step 2: Reindex Meilisearch
    tracing::info!("selection sync: reindexing Meilisearch");
    crate::meili::sync_selection_courses_to_meili(meili_url, meili_key, pool).await;
    tracing::info!("selection sync: Meilisearch reindex complete");

    // Step 3: Bump cache versions
    if let Some(redis_pool) = redis {
        for (prefix, id) in &[("calendars", "all"), ("faculties", "all"), ("natures", "all")] {
            if let Err(e) = shared::cache::bump_version(redis_pool, prefix, id).await {
                tracing::warn!(error = %e, prefix, id, "selection sync: cache version bump failed");
            }
        }
    }
    tracing::info!("selection sync: pipeline complete");

    Ok(())
}
