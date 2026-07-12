//! Database access layer for the media domain.

use chrono::{DateTime, Utc};
use shared::{AppError, AppResult};
use sqlx::{PgConnection, PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::error::MediaError;
use crate::models::{AssetVariantRow, ModerationUploadRow, UploadRow};

const MAX_ACTIVE_UPLOAD_INTENTS: i64 = 10;
const MAX_UPLOAD_CREDENTIALS_PER_DAY: i64 = 100;
const MAX_ACCOUNT_MEDIA_BYTES: i64 = 512 * 1024 * 1024;
const MAX_ACCOUNT_LIVE_MEDIA_OBJECTS: i64 = 500;
const MAX_ACCOUNT_MEDIA_RECORDS: i64 = 2_000;

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
    pub callback_token_hash: Vec<u8>,
    pub expires_at: DateTime<Utc>,
    pub upload_id: Option<i64>,
}

/// Consume the database-authoritative upload credential quota for one account.
pub(crate) async fn consume_upload_credential_quota(
    connection: &mut PgConnection,
    account_id: i64,
    reserved_bytes: i64,
) -> AppResult<()> {
    if !(1..=20 * 1024 * 1024).contains(&reserved_bytes) {
        return Err(AppError::BadRequest("invalid upload byte reservation".into()));
    }
    let daily_attempts: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM media.upload_credential_attempts \
         WHERE account_id = $1 AND created_at > now() - interval '24 hours'",
    )
    .bind(account_id)
    .fetch_one(&mut *connection)
    .await?;
    if daily_attempts >= MAX_UPLOAD_CREDENTIALS_PER_DAY {
        return Err(AppError::RateLimited);
    }

    let (active_intents, media_records, live_media_objects, stored_bytes, reserved_intent_bytes): (
        i64,
        i64,
        i64,
        i64,
        i64,
    ) = sqlx::query_as(
        "SELECT \
           (SELECT count(*)::bigint FROM media.upload_intents intent \
            WHERE intent.account_id = $1 AND intent.upload_id IS NULL \
              AND intent.revoked_at IS NULL AND intent.expires_at > now()), \
           (SELECT count(*)::bigint FROM media.uploads upload \
            WHERE upload.account_id = $1 AND NOT upload.is_cleanup_tombstone), \
           (SELECT count(*)::bigint FROM media.uploads upload \
            WHERE upload.account_id = $1 AND NOT upload.is_cleanup_tombstone \
              AND upload.status IN ('pending', 'clean', 'quarantined')), \
           COALESCE(( \
             SELECT sum(upload.bytes)::bigint FROM media.uploads upload \
             WHERE upload.account_id = $1 AND NOT upload.is_cleanup_tombstone \
               AND upload.status IN ('pending', 'clean', 'quarantined') \
           ), 0)::bigint, \
           COALESCE(( \
             SELECT sum(intent.max_bytes)::bigint FROM media.upload_intents intent \
             LEFT JOIN media.uploads upload ON upload.id = intent.upload_id \
             WHERE intent.account_id = $1 \
               AND (intent.upload_id IS NULL \
                    OR (upload.is_cleanup_tombstone AND upload.status = 'quarantined')) \
           ), 0)::bigint",
    )
    .bind(account_id)
    .fetch_one(&mut *connection)
    .await?;
    if active_intents >= MAX_ACTIVE_UPLOAD_INTENTS
        || media_records.saturating_add(active_intents) >= MAX_ACCOUNT_MEDIA_RECORDS
        || live_media_objects.saturating_add(active_intents) >= MAX_ACCOUNT_LIVE_MEDIA_OBJECTS
    {
        return Err(AppError::RateLimited);
    }
    if stored_bytes.saturating_add(reserved_intent_bytes).saturating_add(reserved_bytes)
        > MAX_ACCOUNT_MEDIA_BYTES
    {
        return Err(AppError::RateLimited);
    }

    sqlx::query(
        "INSERT INTO media.upload_credential_attempts (account_id, reserved_bytes) \
         VALUES ($1, $2)",
    )
    .bind(account_id)
    .bind(reserved_bytes)
    .execute(&mut *connection)
    .await?;
    Ok(())
}

/// Insert a new upload intent bound to one account-scoped object key.
#[allow(clippy::too_many_arguments)] // reason: upload intent creation binds every persisted authorization field explicitly.
pub async fn insert_upload_intent(
    connection: &mut PgConnection,
    intent_id: Uuid,
    account_id: i64,
    kind: &str,
    oss_key: &str,
    content_type: &str,
    usage: Option<&str>,
    max_bytes: i64,
    callback_token_hash: &[u8],
    expires_at: DateTime<Utc>,
) -> AppResult<UploadIntentRow> {
    let row = sqlx::query_as::<_, UploadIntentRow>(
        "INSERT INTO media.upload_intents \
         (id, account_id, kind, oss_key, content_type, usage, max_bytes, \
          callback_token_hash, expires_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
         RETURNING id, account_id, kind, oss_key, content_type, usage, max_bytes, \
                   callback_token_hash, expires_at, upload_id",
    )
    .bind(intent_id)
    .bind(account_id)
    .bind(kind)
    .bind(oss_key)
    .bind(content_type)
    .bind(usage)
    .bind(max_bytes)
    .bind(callback_token_hash)
    .bind(expires_at)
    .fetch_one(connection)
    .await?;
    Ok(row)
}

/// Insert or re-resolve one derived asset variant for a source upload.
#[allow(dead_code)] // reason: phase 2 exposes the repository API before any caller is wired up.
pub async fn insert_variant(
    connection: &mut PgConnection,
    asset_id: i64,
    variant: &str,
    object_key: &str,
    content_hash: &str,
    mime: &str,
    bytes: i64,
    width: Option<i32>,
    height: Option<i32>,
    status: &str,
    processing_attempts: i32,
) -> AppResult<AssetVariantRow> {
    if !matches!(
        variant,
        "original" | "thumbnail" | "small" | "medium" | "large" | "avif" | "webp"
    ) {
        return Err(AppError::BadRequest("invalid media variant".into()));
    }
    if !matches!(status, "processing" | "published" | "quarantined" | "deleted") {
        return Err(AppError::BadRequest("invalid media variant status".into()));
    }
    if bytes <= 0 {
        return Err(AppError::BadRequest("invalid media variant size".into()));
    }
    if processing_attempts < 0 {
        return Err(AppError::BadRequest("invalid media variant attempts".into()));
    }

    let inserted = sqlx::query_as::<_, AssetVariantRow>(
        "INSERT INTO media.asset_variants \
         (asset_id, variant, object_key, content_hash, mime, bytes, width, height, status, \
          processing_attempts) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) \
         ON CONFLICT (asset_id, variant, content_hash) DO NOTHING \
         RETURNING id, asset_id, variant, object_key, content_hash, mime, bytes, width, height, \
                   status, processing_attempts, created_at, published_at, quarantined_at, deleted_at",
    )
    .bind(asset_id)
    .bind(variant)
    .bind(object_key)
    .bind(content_hash)
    .bind(mime)
    .bind(bytes)
    .bind(width)
    .bind(height)
    .bind(status)
    .bind(processing_attempts)
    .fetch_optional(&mut *connection)
    .await?;
    if let Some(row) = inserted {
        return Ok(row);
    }

    let row = sqlx::query_as::<_, AssetVariantRow>(
        "SELECT id, asset_id, variant, object_key, content_hash, mime, bytes, width, height, \
                status, processing_attempts, created_at, published_at, quarantined_at, deleted_at \
         FROM media.asset_variants \
         WHERE asset_id = $1 AND variant = $2 AND content_hash = $3",
    )
    .bind(asset_id)
    .bind(variant)
    .bind(content_hash)
    .fetch_one(&mut *connection)
    .await?;
    Ok(row)
}

/// List all derived variants for one source upload.
#[allow(dead_code)] // reason: phase 2 exposes the repository API before any caller is wired up.
pub async fn find_variants_by_asset(
    pool: &PgPool,
    asset_id: i64,
) -> AppResult<Vec<AssetVariantRow>> {
    let rows = sqlx::query_as::<_, AssetVariantRow>(
        "SELECT id, asset_id, variant, object_key, content_hash, mime, bytes, width, height, \
                status, processing_attempts, created_at, published_at, quarantined_at, deleted_at \
         FROM media.asset_variants \
         WHERE asset_id = $1 \
         ORDER BY variant, id",
    )
    .bind(asset_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Update the lifecycle status for one derived asset variant.
#[allow(dead_code)] // reason: phase 2 exposes the repository API before any caller is wired up.
pub async fn update_variant_status(
    connection: &mut PgConnection,
    id: i64,
    status: &str,
) -> AppResult<()> {
    if !matches!(status, "processing" | "published" | "quarantined" | "deleted") {
        return Err(AppError::BadRequest("invalid media variant status".into()));
    }

    let updated = sqlx::query(
        "UPDATE media.asset_variants \
         SET status = $2, \
             processing_attempts = CASE WHEN $2 = 'processing' \
                                         THEN processing_attempts + 1 \
                                         ELSE processing_attempts END, \
             published_at = CASE WHEN $2 = 'published' AND published_at IS NULL \
                                 THEN now() ELSE published_at END, \
             quarantined_at = CASE WHEN $2 = 'quarantined' AND quarantined_at IS NULL \
                                   THEN now() ELSE quarantined_at END, \
             deleted_at = CASE WHEN $2 = 'deleted' AND deleted_at IS NULL \
                               THEN now() ELSE deleted_at END \
         WHERE id = $1",
    )
    .bind(id)
    .bind(status)
    .execute(&mut *connection)
    .await?;
    if updated.rows_affected() != 1 {
        return Err(MediaError::NotFound.into());
    }
    Ok(())
}

/// Lock an upload intent for callback processing.
pub async fn lock_upload_intent(
    tx: &mut Transaction<'_, Postgres>,
    intent_id: Uuid,
) -> AppResult<Option<UploadIntentRow>> {
    let row = sqlx::query_as::<_, UploadIntentRow>(
        "SELECT id, account_id, kind, oss_key, content_type, usage, max_bytes, \
                callback_token_hash, expires_at, upload_id \
         FROM media.upload_intents WHERE id = $1 AND revoked_at IS NULL FOR UPDATE",
    )
    .bind(intent_id)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(row)
}

/// Revoke an intent whose STS response could not be delivered to the caller.
pub async fn revoke_upload_intent_after_provider_failure(
    pool: &PgPool,
    intent_id: Uuid,
) -> AppResult<()> {
    sqlx::query(
        "UPDATE media.upload_intents SET revoked_at = COALESCE(revoked_at, now()) \
         WHERE id = $1 AND upload_id IS NULL",
    )
    .bind(intent_id)
    .execute(pool)
    .await?;
    Ok(())
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
         RETURNING id, account_id, kind, oss_key, bytes, mime, status, usage, \
                   image_width, image_height, created_at",
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
        "SELECT id, account_id, kind, oss_key, bytes, mime, status, usage, \
                image_width, image_height, created_at \
         FROM media.uploads WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
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

/// List uploads one actor may moderate with cursor-based pagination.
///
/// The cursor is the opaque base64-encoded `(created_at_timestamp, id)` pair.
/// Returns `(rows, next_cursor)`.
pub async fn list_moderatable(
    pool: &PgPool,
    actor_id: i64,
    actor_role: &str,
    status: &str,
    cursor: Option<&str>,
    limit: i64,
) -> AppResult<(Vec<ModerationUploadRow>, Option<String>)> {
    let (created_at_bound, id_bound) = if let Some(cursor) = cursor {
        let decoded = decode_upload_cursor(cursor)?;
        (Some(decoded.0), Some(decoded.1))
    } else {
        (None, None)
    };

    let rows = sqlx::query_as::<_, ModerationUploadRow>(
        "SELECT upload.id, upload.account_id, upload.kind, upload.bytes, upload.mime, \
                upload.status, upload.usage, upload.image_width, upload.image_height, \
                upload.created_at, \
                EXISTS ( \
                  SELECT 1 FROM media.moderation_evidence evidence \
                  WHERE evidence.upload_id = upload.id \
                    AND evidence.evidence_kind = 'trusted_image_preview' \
                    AND evidence.actor_account_id = $1 \
                ) AS has_reviewer_evidence, \
                deletion.status AS deletion_state, \
                EXISTS ( \
                  SELECT 1 FROM media.asset_retention_holds hold \
                  WHERE hold.asset_id = upload.id AND hold.released_at IS NULL \
                    AND hold.expires_at > now() \
                ) AS retention_held, \
                CASE \
                  WHEN EXISTS (SELECT 1 FROM media.asset_retention_holds hold \
                               WHERE hold.asset_id = upload.id AND hold.released_at IS NULL \
                                 AND hold.expires_at > now()) THEN 'active' \
                  WHEN EXISTS (SELECT 1 FROM media.asset_retention_holds hold \
                               WHERE hold.asset_id = upload.id AND hold.released_at IS NULL) \
                    THEN 'expired' \
                  ELSE 'none' \
                END AS retention_state, \
                (SELECT hold.expires_at FROM media.asset_retention_holds hold \
                 WHERE hold.asset_id = upload.id AND hold.released_at IS NULL \
                 LIMIT 1) AS retention_expires_at \
         FROM media.uploads upload \
         JOIN identity.accounts owner ON owner.id = upload.account_id \
         LEFT JOIN media.object_deletion_jobs deletion ON deletion.upload_id = upload.id \
         WHERE upload.status = $3 \
           AND upload.account_id <> $1 \
           AND ($3 IN ('pending', 'clean') OR deletion.request_source = 'moderation') \
           AND (($2 = 'mod' AND owner.role = 'user') \
                OR ($2 = 'admin' AND owner.role IN ('user', 'mod'))) \
           AND ($4::timestamptz IS NULL OR upload.created_at < $4::timestamptz \
                OR (upload.created_at = $4::timestamptz AND upload.id < $5::bigint)) \
         ORDER BY upload.created_at DESC, upload.id DESC \
         LIMIT $6",
    )
    .bind(actor_id)
    .bind(actor_role)
    .bind(status)
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
        "SELECT id, account_id, kind, oss_key, bytes, mime, status, usage, \
                image_width, image_height, created_at \
         FROM media.uploads \
         WHERE account_id = $1 AND NOT is_cleanup_tombstone \
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
        "SELECT id, account_id, kind, oss_key, bytes, mime, status, usage, \
                image_width, image_height, created_at \
         FROM media.uploads WHERE id = $1 AND account_id = $2 AND NOT is_cleanup_tombstone",
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
