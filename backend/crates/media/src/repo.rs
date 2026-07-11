//! Database access layer for the media domain.

use chrono::{DateTime, Utc};
use shared::AppResult;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::error::MediaError;
use crate::models::UploadRow;

/// Server-issued upload authorization row.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UploadIntentRow {
    pub id: Uuid,
    pub account_id: i64,
    pub kind: String,
    pub oss_key: String,
    pub content_type: String,
    pub usage: Option<String>,
    pub max_bytes: i64,
    pub callback_token: String,
    pub expires_at: DateTime<Utc>,
    pub upload_id: Option<i64>,
}

/// Insert a new upload intent bound to one account-scoped object key.
#[allow(clippy::too_many_arguments)] // reason: upload intent creation binds every persisted authorization field explicitly.
pub async fn insert_upload_intent(
    pool: &PgPool,
    intent_id: Uuid,
    account_id: i64,
    kind: &str,
    oss_key: &str,
    content_type: &str,
    usage: Option<&str>,
    max_bytes: i64,
    callback_token: &str,
    expires_at: DateTime<Utc>,
) -> AppResult<UploadIntentRow> {
    let row = sqlx::query_as::<_, UploadIntentRow>(
        "INSERT INTO media.upload_intents \
         (id, account_id, kind, oss_key, content_type, usage, max_bytes, callback_token, expires_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
         RETURNING id, account_id, kind, oss_key, content_type, usage, max_bytes, callback_token, \
                   expires_at, upload_id",
    )
    .bind(intent_id)
    .bind(account_id)
    .bind(kind)
    .bind(oss_key)
    .bind(content_type)
    .bind(usage)
    .bind(max_bytes)
    .bind(callback_token)
    .bind(expires_at)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

/// Lock an upload intent for callback processing.
pub async fn lock_upload_intent(
    tx: &mut Transaction<'_, Postgres>,
    intent_id: Uuid,
) -> AppResult<Option<UploadIntentRow>> {
    let row = sqlx::query_as::<_, UploadIntentRow>(
        "SELECT id, account_id, kind, oss_key, content_type, usage, max_bytes, callback_token, \
                expires_at, upload_id \
         FROM media.upload_intents WHERE id = $1 FOR UPDATE",
    )
    .bind(intent_id)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(row)
}

/// Insert an upload row inside the callback transaction.
#[allow(clippy::too_many_arguments)] // reason: callback row creation mirrors the persisted upload columns.
pub async fn insert_upload_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    account_id: i64,
    kind: &str,
    oss_key: &str,
    url: &str,
    bytes: i64,
    mime: &str,
    sha256: &str,
    usage: Option<&str>,
) -> AppResult<UploadRow> {
    let row = sqlx::query_as::<_, UploadRow>(
        "INSERT INTO media.uploads (account_id, kind, oss_key, url, bytes, mime, sha256, usage) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
         RETURNING id, account_id, kind, oss_key, url, bytes, mime, sha256, status, usage, created_at",
    )
    .bind(account_id)
    .bind(kind)
    .bind(oss_key)
    .bind(url)
    .bind(bytes)
    .bind(mime)
    .bind(sha256)
    .bind(usage)
    .fetch_one(&mut **tx)
    .await?;
    Ok(row)
}

/// Mark an upload intent consumed by the created upload row.
pub async fn consume_upload_intent(
    tx: &mut Transaction<'_, Postgres>,
    intent_id: Uuid,
    upload_id: i64,
) -> AppResult<()> {
    let affected = sqlx::query(
        "UPDATE media.upload_intents \
         SET consumed_at = now(), upload_id = $2 \
         WHERE id = $1 AND consumed_at IS NULL",
    )
    .bind(intent_id)
    .bind(upload_id)
    .execute(&mut **tx)
    .await?
    .rows_affected();
    if affected != 1 {
        return Err(MediaError::BadRequest("upload intent already consumed".into()).into());
    }
    Ok(())
}

/// Find an upload by its primary key.
pub async fn find_upload(pool: &PgPool, id: i64) -> AppResult<Option<UploadRow>> {
    let row = sqlx::query_as::<_, UploadRow>(
        "SELECT id, account_id, kind, oss_key, url, bytes, mime, sha256, status, usage, created_at \
         FROM media.uploads WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Verify an owned clean image while holding it stable for profile binding.
pub async fn owned_clean_image_exists(
    tx: &mut Transaction<'_, Postgres>,
    account_id: i64,
    upload_id: i64,
) -> AppResult<bool> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS( \
           SELECT 1 FROM media.uploads \
           WHERE id = $1 AND account_id = $2 AND kind = 'image' AND status = 'clean' \
           FOR SHARE \
         )",
    )
    .bind(upload_id)
    .bind(account_id)
    .fetch_one(&mut **tx)
    .await?;
    Ok(exists)
}

/// Resolve only a clean image URL for an already-authorized public projection.
pub async fn find_clean_image_url(pool: &PgPool, upload_id: i64) -> AppResult<Option<String>> {
    let url = sqlx::query_scalar(
        "SELECT url FROM media.uploads WHERE id = $1 AND kind = 'image' AND status = 'clean'",
    )
    .bind(upload_id)
    .fetch_optional(pool)
    .await?;
    Ok(url)
}

/// Batch-resolve clean image URLs without exposing pending or blocked objects.
pub async fn find_clean_image_urls(
    pool: &PgPool,
    upload_ids: &[i64],
) -> AppResult<Vec<(i64, String)>> {
    if upload_ids.is_empty() {
        return Ok(Vec::new());
    }
    let urls = sqlx::query_as(
        "SELECT id, url FROM media.uploads \
         WHERE id = ANY($1) AND kind = 'image' AND status = 'clean'",
    )
    .bind(upload_ids)
    .fetch_all(pool)
    .await?;
    Ok(urls)
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
    let (created_at_bound, id_bound) = if let Some(cursor) = cursor {
        let decoded = decode_upload_cursor(cursor)?;
        (Some(decoded.0), Some(decoded.1))
    } else {
        (None, None)
    };

    let rows = sqlx::query_as::<_, UploadRow>(
        "SELECT id, account_id, kind, oss_key, url, bytes, mime, sha256, status, usage, created_at \
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
        Some(encode_upload_cursor(last.created_at, last.id))
    } else {
        None
    };

    Ok((items, next_cursor))
}

/// List recent uploads owned by one account, optionally scoped to an intended profile slot.
pub async fn list_owned(
    pool: &PgPool,
    account_id: i64,
    usage: Option<&str>,
    cursor: Option<&str>,
    limit: i64,
) -> AppResult<(Vec<UploadRow>, Option<String>)> {
    let (created_at_bound, id_bound) = if let Some(cursor) = cursor {
        let decoded = decode_upload_cursor(cursor)?;
        (Some(decoded.0), Some(decoded.1))
    } else {
        (None, None)
    };
    let rows = sqlx::query_as::<_, UploadRow>(
        "SELECT id, account_id, kind, oss_key, url, bytes, mime, sha256, status, usage, created_at \
         FROM media.uploads \
         WHERE account_id = $1 \
           AND ($2::text IS NULL OR usage = $2) \
           AND ($3::timestamptz IS NULL OR created_at < $3::timestamptz \
                OR (created_at = $3::timestamptz AND id < $4::bigint)) \
         ORDER BY created_at DESC, id DESC \
         LIMIT $5",
    )
    .bind(account_id)
    .bind(usage)
    .bind(created_at_bound)
    .bind(id_bound)
    .bind(limit + 1)
    .fetch_all(pool)
    .await?;

    let has_more = rows.len() as i64 > limit;
    let items = rows.into_iter().take(limit as usize).collect::<Vec<_>>();
    let next_cursor = if has_more {
        let last = items.last().ok_or(MediaError::BadRequest("empty result set".into()))?;
        Some(encode_upload_cursor(last.created_at, last.id))
    } else {
        None
    };
    Ok((items, next_cursor))
}

/// Find one upload only when it belongs to the requesting account.
pub async fn find_owned_upload(
    pool: &PgPool,
    account_id: i64,
    upload_id: i64,
) -> AppResult<Option<UploadRow>> {
    let row = sqlx::query_as::<_, UploadRow>(
        "SELECT id, account_id, kind, oss_key, url, bytes, mime, sha256, status, usage, created_at \
         FROM media.uploads WHERE id = $1 AND account_id = $2",
    )
    .bind(upload_id)
    .bind(account_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Encode an upload-list cursor: base64(`created_at_timestamp:id`).
fn encode_upload_cursor(created_at: chrono::DateTime<chrono::Utc>, id: i64) -> String {
    use base64::Engine;
    let raw = format!("{}:{}", created_at.timestamp_micros(), id);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw)
}

/// Decode an upload-list cursor back into `(created_at_timestamp, id)`.
fn decode_upload_cursor(cursor: &str) -> Result<(chrono::DateTime<chrono::Utc>, i64), MediaError> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(cursor)
        .map_err(|error| MediaError::BadRequest(format!("invalid cursor: {error}")))?;
    let decoded = String::from_utf8(bytes)
        .map_err(|error| MediaError::BadRequest(format!("invalid cursor encoding: {error}")))?;
    let (timestamp_text, id_text) = decoded
        .rsplit_once(':')
        .ok_or_else(|| MediaError::BadRequest("invalid cursor format".into()))?;
    let timestamp: i64 = timestamp_text
        .parse()
        .map_err(|error| MediaError::BadRequest(format!("invalid cursor timestamp: {error}")))?;
    let id: i64 = id_text
        .parse()
        .map_err(|error| MediaError::BadRequest(format!("invalid cursor id: {error}")))?;
    let created_at = if (-100_000_000_000..100_000_000_000).contains(&timestamp) {
        chrono::DateTime::from_timestamp(timestamp, 0)
    } else {
        chrono::DateTime::from_timestamp_micros(timestamp)
    }
    .ok_or_else(|| MediaError::BadRequest("cursor timestamp out of range".into()))?;
    Ok((created_at, id))
}

#[cfg(test)]
mod tests {
    use base64::Engine;
    use chrono::TimeZone;

    use super::{decode_upload_cursor, encode_upload_cursor};

    #[test]
    fn upload_cursor_preserves_subsecond_order_and_accepts_legacy_seconds() {
        let created_at = chrono::Utc
            .timestamp_opt(1_720_000_000, 123_456_000)
            .single()
            .expect("valid cursor timestamp");
        let encoded = encode_upload_cursor(created_at, 42);
        assert_eq!(decode_upload_cursor(&encoded).expect("current cursor"), (created_at, 42));

        let legacy = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode("1720000000:41");
        let (legacy_created_at, legacy_id) = decode_upload_cursor(&legacy).expect("legacy cursor");
        assert_eq!(legacy_created_at.timestamp(), 1_720_000_000);
        assert_eq!(legacy_id, 41);
    }
}
