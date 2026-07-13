//! Database access layer for the media domain.

use chrono::{DateTime, Utc};
use shared::{AppError, AppResult};
use sqlx::{PgConnection, PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::error::MediaError;
use crate::models::{ModerationUploadRow, UploadRow};

const MAX_ACTIVE_UPLOAD_INTENTS: i64 = 10;
const MAX_UPLOAD_CREDENTIALS_PER_DAY: i64 = 100;
const MAX_ACCOUNT_MEDIA_BYTES: i64 = 512 * 1024 * 1024;
const MAX_ACCOUNT_LIVE_MEDIA_OBJECTS: i64 = 500;
const MAX_ACCOUNT_MEDIA_RECORDS: i64 = 2_000;

fn moderation_scan_budget(limit: i64) -> i64 {
    limit.saturating_mul(20).clamp(100, 1_000)
}

#[derive(Debug, sqlx::FromRow)]
struct CleanImageDeliveryRow {
    asset_id: i64,
    object_key: String,
    mime: String,
    width: i32,
    height: i32,
    variant_kind: String,
}

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
    bytes: i64,
    mime: &str,
    sha256: &str,
    usage: Option<&str>,
) -> AppResult<UploadRow> {
    let row = sqlx::query_as::<_, UploadRow>(
        "INSERT INTO media.uploads (account_id, kind, oss_key, url, bytes, mime, sha256, usage) \
         VALUES ($1, $2, $3, '', $4, $5, $6, $7) \
         RETURNING id, kind, bytes, mime, status, usage, \
                   image_width, image_height, created_at, \
                   'unpublished'::text AS delivery_state",
    )
    .bind(account_id)
    .bind(kind)
    .bind(oss_key)
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

/// Batch-resolve typed clean image delivery without exposing storage identifiers.
pub async fn find_clean_image_deliveries(
    pool: &PgPool,
    upload_ids: &[i64],
    variant: crate::delivery::ImageVariant,
) -> AppResult<Vec<(i64, crate::delivery::ImageDeliveryProjection)>> {
    if upload_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows = sqlx::query_as::<_, CleanImageDeliveryRow>(
        "SELECT upload.id AS asset_id, variant.object_key, variant.mime, \
                variant.width, variant.height, variant.variant_kind \
         FROM media.uploads upload \
         JOIN media.asset_publications publication ON publication.asset_id = upload.id \
         JOIN media.asset_variants variant \
           ON variant.asset_id = upload.id \
          AND variant.policy_version = publication.policy_version \
          AND variant.variant_kind = $2 \
         WHERE upload.id = ANY($1) AND upload.kind = 'image' AND upload.status = 'clean' \
           AND publication.status = 'published' AND variant.status = 'published'",
    )
    .bind(upload_ids)
    .bind(variant.as_database())
    .fetch_all(pool)
    .await?;
    if rows.is_empty() {
        return Ok(Vec::new());
    }
    let delivery = crate::delivery::require_delivery_config_from_env()?;
    rows.into_iter()
        .map(|row| {
            let signed = delivery.sign_object(&row.object_key)?;
            Ok((
                row.asset_id,
                crate::delivery::ImageDeliveryProjection {
                    asset_id: row.asset_id.to_string(),
                    url: signed.url,
                    expires_at: signed.expires_at.timestamp(),
                    mime: row.mime,
                    width: row.width,
                    height: row.height,
                    variant: crate::delivery::ImageVariant::from_database(&row.variant_kind)?,
                },
            ))
        })
        .collect()
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

    let actor =
        shared::AuthAccount { id: actor_id, role: actor_role.to_owned(), status: "active".into() };
    let mut scan_created_at = created_at_bound;
    let mut scan_id = id_bound;
    let mut eligible_rows = Vec::with_capacity(limit as usize + 1);
    let mut scanned_candidates = 0_i64;
    let scan_budget = moderation_scan_budget(limit);
    let mut candidates_exhausted = false;
    const CANDIDATE_BATCH_SIZE: i64 = 200;

    loop {
        if scanned_candidates >= scan_budget {
            break;
        }
        let batch_limit = CANDIDATE_BATCH_SIZE.min(scan_budget - scanned_candidates);
        let candidates = sqlx::query_as::<_, ModerationUploadRow>(
            "SELECT upload.id, upload.account_id, upload.kind, upload.bytes, upload.mime, \
                upload.status, publication.status AS delivery_state, \
                publication.last_error_code AS delivery_error_code, upload.usage, \
                upload.image_width, upload.image_height, \
                upload.created_at, upload.account_id = $1 AS is_self_review, \
                EXISTS ( \
                  SELECT 1 FROM media.moderation_evidence evidence \
                  WHERE evidence.upload_id = upload.id \
                    AND evidence.evidence_kind = 'trusted_image_preview' \
                    AND evidence.actor_account_id = $1 \
                    AND evidence.self_review = (upload.account_id = $1) \
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
         JOIN media.asset_publications publication ON publication.asset_id = upload.id \
         LEFT JOIN media.object_deletion_jobs deletion ON deletion.upload_id = upload.id \
         WHERE upload.status = $2 \
           AND ($2 IN ('pending', 'clean') OR deletion.request_source = 'moderation') \
           AND ($3::timestamptz IS NULL OR upload.created_at < $3::timestamptz \
                OR (upload.created_at = $3::timestamptz AND upload.id < $4::bigint)) \
         ORDER BY upload.created_at DESC, upload.id DESC \
         LIMIT $5",
        )
        .bind(actor_id)
        .bind(status)
        .bind(scan_created_at)
        .bind(scan_id)
        .bind(batch_limit)
        .fetch_all(pool)
        .await?;
        if candidates.is_empty() {
            candidates_exhausted = true;
            break;
        }
        scanned_candidates += candidates.len() as i64;
        let batch_is_exhausted = candidates.len() < batch_limit as usize;
        let last_candidate = candidates
            .last()
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("media candidate page vanished")))?;
        scan_created_at = Some(last_candidate.created_at);
        scan_id = Some(last_candidate.id);
        let owner_ids = candidates.iter().map(|row| row.account_id).collect::<Vec<_>>();
        let owner_states =
            identity::public_accounts::find_staff_target_authorization_states_by_ids(
                pool, &owner_ids,
            )
            .await?;
        for row in candidates {
            let Some(owner) = owner_states.get(&row.account_id) else {
                continue;
            };
            if crate::moderation::authorize_moderation(&actor, row.account_id, &owner.role, true)
                .is_ok()
            {
                eligible_rows.push(row);
                if eligible_rows.len() as i64 > limit {
                    break;
                }
            }
        }
        if eligible_rows.len() as i64 > limit || batch_is_exhausted {
            candidates_exhausted = batch_is_exhausted;
            break;
        }
    }

    let has_more = eligible_rows.len() as i64 > limit;
    let items = eligible_rows.into_iter().take(limit as usize).collect::<Vec<_>>();

    let next_cursor = if has_more {
        let last = items.last().ok_or(MediaError::BadRequest("empty result set".into()))?;
        Some(encode_upload_cursor(last.created_at, last.id))
    } else if !candidates_exhausted && scanned_candidates >= scan_budget {
        Some(encode_upload_cursor(
            scan_created_at.ok_or_else(|| {
                AppError::Internal(anyhow::anyhow!("media scan cursor timestamp is missing"))
            })?,
            scan_id.ok_or_else(|| {
                AppError::Internal(anyhow::anyhow!("media scan cursor id is missing"))
            })?,
        ))
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
        "SELECT upload.id, upload.kind, upload.bytes, upload.mime, upload.status, upload.usage, \
                upload.image_width, upload.image_height, upload.created_at, \
                publication.status AS delivery_state \
         FROM media.uploads upload \
         JOIN media.asset_publications publication ON publication.asset_id = upload.id \
         WHERE upload.account_id = $1 AND NOT upload.is_cleanup_tombstone \
           AND ($2::text IS NULL OR usage = $2) \
           AND ($3::timestamptz IS NULL OR upload.created_at < $3::timestamptz \
                OR (upload.created_at = $3::timestamptz AND upload.id < $4::bigint)) \
         ORDER BY upload.created_at DESC, upload.id DESC \
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
        "SELECT upload.id, upload.kind, upload.bytes, upload.mime, upload.status, upload.usage, \
                upload.image_width, upload.image_height, upload.created_at, \
                publication.status AS delivery_state \
         FROM media.uploads upload \
         JOIN media.asset_publications publication ON publication.asset_id = upload.id \
         WHERE upload.id = $1 AND upload.account_id = $2 AND NOT upload.is_cleanup_tombstone",
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

    use super::{decode_upload_cursor, encode_upload_cursor, moderation_scan_budget};

    #[test]
    fn moderation_scan_budget_is_proportional_and_bounded() {
        assert_eq!(moderation_scan_budget(1), 100);
        assert_eq!(moderation_scan_budget(20), 400);
        assert_eq!(moderation_scan_budget(100), 1_000);
        assert_eq!(moderation_scan_budget(i64::MAX), 1_000);
    }

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
