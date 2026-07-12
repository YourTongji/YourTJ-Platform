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

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use super::snapshot;

    #[tokio::test]
    async fn owner_export_excludes_internal_cleanup_tombstones() {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://yourtj:yourtj@localhost:5432/yourtj_test".to_string());
        let pool = PgPool::connect(&database_url).await.expect("connect media export database");
        sqlx::migrate!("../../migrations").run(&pool).await.expect("apply media export migrations");
        let suffix = uuid::Uuid::new_v4().simple().to_string();
        let account_id: i64 = sqlx::query_scalar(
            "INSERT INTO identity.accounts (email, email_verified_at, handle) \
             VALUES ($1, now(), $2) RETURNING id",
        )
        .bind(format!("media-export-{suffix}@tongji.edu.cn"))
        .bind(format!("media-export-{suffix}"))
        .fetch_one(&pool)
        .await
        .expect("insert media export account");
        let owner_upload_id: i64 = sqlx::query_scalar(
            "INSERT INTO media.uploads \
             (account_id, kind, oss_key, url, bytes, mime, sha256, status) \
             VALUES ($1, 'image', $2, '', 17, 'image/png', repeat('a', 64), 'pending') \
             RETURNING id",
        )
        .bind(account_id)
        .bind(format!("uploads/{account_id}/image/{suffix}-owner.png"))
        .fetch_one(&pool)
        .await
        .expect("insert owner-visible upload");
        let cleanup_upload_id: i64 = sqlx::query_scalar(
            "INSERT INTO media.uploads \
             (account_id, kind, oss_key, url, bytes, mime, sha256, status, \
              is_cleanup_tombstone) \
             VALUES ($1, 'image', $2, '', 0, 'image/png', '', 'quarantined', TRUE) \
             RETURNING id",
        )
        .bind(account_id)
        .bind(format!("uploads/{account_id}/image/{suffix}-cleanup.png"))
        .fetch_one(&pool)
        .await
        .expect("insert internal cleanup tombstone");

        let uploads = snapshot(&pool, account_id).await.expect("export owner media");
        assert_eq!(uploads.len(), 1);
        assert_eq!(uploads[0].id, owner_upload_id);

        sqlx::query("DELETE FROM media.uploads WHERE id = ANY($1)")
            .bind(vec![owner_upload_id, cleanup_upload_id])
            .execute(&pool)
            .await
            .expect("delete media export uploads");
        sqlx::query("DELETE FROM identity.accounts WHERE id = $1")
            .bind(account_id)
            .execute(&pool)
            .await
            .expect("delete media export account");
    }
}
