//! Post revision history for edit tracking.

use shared::AppResult;
use sqlx::{PgConnection, PgPool};

use crate::models::PostRevisionRow;

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

/// List revisions for a post.
pub async fn list_revisions(
    pool: &PgPool,
    post_type: &str,
    post_id: i64,
) -> AppResult<Vec<PostRevisionRow>> {
    let rows = sqlx::query_as::<_, PostRevisionRow>(
        "SELECT id, post_type, post_id, seq, editor_id, old_title, old_body, old_content_format, \
                old_content_version, created_at \
         FROM forum.post_revisions \
         WHERE post_type = $1 AND post_id = $2 \
         ORDER BY seq DESC",
    )
    .bind(post_type)
    .bind(post_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
