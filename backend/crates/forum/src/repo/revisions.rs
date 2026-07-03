//! Post revision history for edit tracking.

use shared::AppResult;
use sqlx::PgPool;

use crate::models::PostRevisionRow;

/// Create a revision record for a post before editing.
pub async fn create_revision(
    pool: &PgPool,
    post_type: &str,
    post_id: i64,
    editor_id: i64,
    old_title: Option<&str>,
    old_body: &str,
) -> AppResult<PostRevisionRow> {
    // Find next seq for this post
    let current_seq: i32 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(seq), 0) FROM forum.post_revisions WHERE post_type = $1 AND post_id = $2",
    )
    .bind(post_type)
    .bind(post_id)
    .fetch_one(pool)
    .await?;

    let row = sqlx::query_as::<_, PostRevisionRow>(
        "INSERT INTO forum.post_revisions (post_type, post_id, seq, editor_id, old_title, old_body) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         RETURNING id, post_type, post_id, seq, editor_id, old_title, old_body, created_at",
    )
    .bind(post_type)
    .bind(post_id)
    .bind(current_seq + 1)
    .bind(editor_id)
    .bind(old_title)
    .bind(old_body)
    .fetch_one(pool)
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
        "SELECT id, post_type, post_id, seq, editor_id, old_title, old_body, created_at \
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
