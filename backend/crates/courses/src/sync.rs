//! Selection data sync pipeline — materialize pk_* → selection.*, reindex
//! Meilisearch, and bump cache versions.
//!
//! Called from the admin sync endpoint as an in-process background task.

use anyhow::Context;
use deadpool_redis::Pool as RedisPool;
use sqlx::PgPool;

/// Execute the full selection sync pipeline.
///
/// Steps:
/// 1. Materialize selection.* tables from pk_* data (idempotent)
/// 2. Apply the Meilisearch selection index settings
/// 3. Clear and rebuild the selection index, waiting for both tasks
/// 4. Bump Redis cache versions for selection endpoints
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

    tracing::info!("selection sync: applying Meilisearch index settings");
    crate::meili::setup_selection_index(meili_url, meili_key)
        .await
        .map_err(anyhow::Error::msg)
        .context("selection Meilisearch index setup failed")?;

    tracing::info!("selection sync: rebuilding Meilisearch index");
    let document_count = crate::meili::sync_selection_courses_to_meili(meili_url, meili_key, pool)
        .await
        .context("selection Meilisearch rebuild failed")?;
    tracing::info!(document_count, "selection sync: Meilisearch rebuild complete");

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
