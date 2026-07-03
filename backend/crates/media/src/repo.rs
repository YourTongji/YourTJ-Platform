//! Database access layer for the media domain.

use shared::AppResult;
use sqlx::PgPool;

use crate::error::MediaError;
use crate::models::UploadRow;

/// Insert a new upload row with status `pending`.
#[allow(clippy::too_many_arguments)]
pub async fn insert_upload(
    pool: &PgPool,
    account_id: i64,
    kind: &str,
    oss_key: &str,
    url: &str,
    bytes: i64,
    mime: &str,
    sha256: &str,
) -> AppResult<UploadRow> {
    let row = sqlx::query_as::<_, UploadRow>(
        "INSERT INTO media.uploads (account_id, kind, oss_key, url, bytes, mime, sha256) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) \
         RETURNING id, account_id, kind, oss_key, url, bytes, mime, sha256, status, created_at",
    )
    .bind(account_id)
    .bind(kind)
    .bind(oss_key)
    .bind(url)
    .bind(bytes)
    .bind(mime)
    .bind(sha256)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

/// Find an upload by its primary key.
pub async fn find_upload(pool: &PgPool, id: i64) -> AppResult<Option<UploadRow>> {
    let row = sqlx::query_as::<_, UploadRow>(
        "SELECT id, account_id, kind, oss_key, url, bytes, mime, sha256, status, created_at \
         FROM media.uploads WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Update the status of an upload.
pub async fn update_status(pool: &PgPool, id: i64, status: &str) -> AppResult<()> {
    let affected = sqlx::query("UPDATE media.uploads SET status = $1 WHERE id = $2")
        .bind(status)
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected();
    if affected == 0 {
        return Err(MediaError::NotFound.into());
    }
    Ok(())
}

/// List pending uploads with cursor-based pagination.
///
/// The cursor is the opaque base64-encoded `(created_at_timestamp, id)` pair.
/// Returns `(rows, next_cursor)`.
pub async fn list_pending(
    pool: &PgPool,
    cursor: Option<&str>,
    limit: i64,
) -> AppResult<(Vec<UploadRow>, Option<String>)> {
    let (created_at_bound, id_bound) = if let Some(c) = cursor {
        let decoded = decode_pending_cursor(c)?;
        (Some(decoded.0), Some(decoded.1))
    } else {
        (None, None)
    };

    let rows = sqlx::query_as::<_, UploadRow>(
        "SELECT id, account_id, kind, oss_key, url, bytes, mime, sha256, status, created_at \
         FROM media.uploads \
         WHERE status = 'pending' \
           AND ($1::timestamptz IS NULL OR created_at < $1::timestamptz \
                OR (created_at = $1::timestamptz AND id < $2::bigint)) \
         ORDER BY created_at DESC, id DESC \
         LIMIT $3",
    )
    .bind(created_at_bound)
    .bind(id_bound)
    .bind(limit + 1)
    .fetch_all(pool)
    .await?;

    let has_more = rows.len() as i64 > limit;
    let items = rows.into_iter().take(limit as usize).collect::<Vec<_>>();

    let next_cursor = if has_more {
        let last = items.last().ok_or(MediaError::BadRequest("empty result set".into()))?;
        Some(encode_pending_cursor(last.created_at, last.id))
    } else {
        None
    };

    Ok((items, next_cursor))
}

/// Encode a pending-list cursor: base64(`created_at_timestamp:id`).
fn encode_pending_cursor(created_at: chrono::DateTime<chrono::Utc>, id: i64) -> String {
    use base64::Engine;
    let raw = format!("{}:{}", created_at.timestamp(), id);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw)
}

/// Decode a pending-list cursor back into `(created_at_timestamp, id)`.
fn decode_pending_cursor(cursor: &str) -> Result<(chrono::DateTime<chrono::Utc>, i64), MediaError> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(cursor)
        .map_err(|e| MediaError::BadRequest(format!("invalid cursor: {e}")))?;
    let s = String::from_utf8(bytes)
        .map_err(|e| MediaError::BadRequest(format!("invalid cursor encoding: {e}")))?;
    let (ts_str, id_str) =
        s.rsplit_once(':').ok_or_else(|| MediaError::BadRequest("invalid cursor format".into()))?;
    let ts: i64 = ts_str
        .parse()
        .map_err(|e| MediaError::BadRequest(format!("invalid cursor timestamp: {e}")))?;
    let id: i64 =
        id_str.parse().map_err(|e| MediaError::BadRequest(format!("invalid cursor id: {e}")))?;
    Ok((
        chrono::DateTime::from_timestamp(ts, 0)
            .ok_or_else(|| MediaError::BadRequest("cursor timestamp out of range".into()))?,
        id,
    ))
}
