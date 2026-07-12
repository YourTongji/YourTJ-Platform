//! Media-owned account export metadata.

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
         FROM media.uploads \
         WHERE account_id = $1 AND NOT is_cleanup_tombstone ORDER BY id",
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?)
}
