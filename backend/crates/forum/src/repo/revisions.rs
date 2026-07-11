//! Post revision history for edit tracking.

use shared::{AppError, AppResult};
use sqlx::{PgConnection, PgPool};

use crate::models::PostRevisionRow;

use super::{base64_decode_i64, base64_encode_i64};

/// Canonical source snapshot stored before an edit.
pub(crate) struct RevisionSource<'a> {
    pub old_title: Option<&'a str>,
    pub old_body: &'a str,
    pub old_content_format: &'a str,
    pub old_content_version: i64,
}

/// Create a revision in the caller's transaction.
///
/// The advisory lock makes sequence allocation safe even if a future caller
/// does not already hold the target content row lock.
pub(crate) async fn create_revision_tx(
    connection: &mut PgConnection,
    post_type: &str,
    post_id: i64,
    editor_id: i64,
    source: RevisionSource<'_>,
) -> AppResult<PostRevisionRow> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(format!("forum:revision:{post_type}:{post_id}"))
        .execute(&mut *connection)
        .await?;

    let current_seq: i32 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(seq), 0) FROM forum.post_revisions WHERE post_type = $1 AND post_id = $2",
    )
    .bind(post_type)
    .bind(post_id)
    .fetch_one(&mut *connection)
    .await?;

    let row = sqlx::query_as::<_, PostRevisionRow>(
        "INSERT INTO forum.post_revisions \
         (post_type, post_id, seq, editor_id, old_title, old_body, old_content_format, old_content_version) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
         RETURNING id, post_type, post_id, seq, editor_id, old_title, old_body, \
                   old_content_format, old_content_version, created_at",
    )
    .bind(post_type)
    .bind(post_id)
    .bind(current_seq + 1)
    .bind(editor_id)
    .bind(source.old_title)
    .bind(source.old_body)
    .bind(source.old_content_format)
    .bind(source.old_content_version)
    .fetch_one(&mut *connection)
    .await?;

    Ok(row)
}

/// List a bounded page of revisions for a post.
pub async fn list_revisions(
    pool: &PgPool,
    post_type: &str,
    post_id: i64,
    cursor: Option<&str>,
    limit: i64,
) -> AppResult<(Vec<PostRevisionRow>, Option<String>)> {
    if !(1..=100).contains(&limit) {
        return Err(AppError::BadRequest("limit must be between 1 and 100".into()));
    }
    let cursor_seq = cursor
        .map(base64_decode_i64)
        .transpose()
        .map_err(|_| AppError::BadRequest("invalid revision cursor".into()))?
        .map(|value| {
            i32::try_from(value)
                .ok()
                .filter(|value| *value > 0)
                .ok_or_else(|| AppError::BadRequest("invalid revision cursor".into()))
        })
        .transpose()?;
    let mut rows = sqlx::query_as::<_, PostRevisionRow>(
        "SELECT id, post_type, post_id, seq, editor_id, old_title, old_body, old_content_format, \
                old_content_version, created_at \
         FROM forum.post_revisions \
         WHERE post_type = $1 AND post_id = $2 \
           AND ($3::int IS NULL OR seq < $3) \
         ORDER BY seq DESC LIMIT $4",
    )
    .bind(post_type)
    .bind(post_id)
    .bind(cursor_seq)
    .bind(limit + 1)
    .fetch_all(pool)
    .await?;
    let has_more = rows.len() > limit as usize;
    if has_more {
        rows.truncate(limit as usize);
    }
    let next_cursor =
        has_more.then(|| rows.last().map(|row| base64_encode_i64(i64::from(row.seq)))).flatten();
    Ok((rows, next_cursor))
}
