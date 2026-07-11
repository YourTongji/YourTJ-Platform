//! Media-owned account export metadata and purge preparation.

use chrono::{DateTime, Utc};
use serde::Serialize;
use shared::AppResult;
use sqlx::{FromRow, PgPool};

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportUpload {
    id: i64,
    kind: String,
    bytes: i64,
    mime: String,
    status: String,
    usage: Option<String>,
    image_width: Option<i32>,
    image_height: Option<i32>,
    #[serde(with = "chrono::serde::ts_seconds")]
    created_at: DateTime<Utc>,
}

pub async fn snapshot(pool: &PgPool, account_id: i64) -> AppResult<Vec<ExportUpload>> {
    Ok(sqlx::query_as::<_, ExportUpload>(
        "SELECT id, kind, bytes, mime, status, usage, image_width, image_height, created_at \
         FROM media.uploads WHERE account_id = $1 ORDER BY id",
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?)
}

/// Revoke unfinished upload authority before the identity tombstone is finalized.
///
/// Bound clean assets remain subject to content retention and the existing media GC grace; this
/// function does not pretend that deleting database metadata removes an OSS object.
pub async fn prepare_account_purge(pool: &PgPool, account_id: i64) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM media.upload_intents WHERE account_id = $1")
        .bind(account_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE media.uploads SET status = 'blocked' \
         WHERE account_id = $1 AND status = 'pending'",
    )
    .bind(account_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}
