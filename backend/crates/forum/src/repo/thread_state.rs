//! Thread state machine operations: pin, close, archive, hide, delete, restore,
//! and solved-answer management for Q&A boards.

use shared::AppResult;
use sqlx::PgPool;

/// Pin a thread to the top of its board.
pub async fn pin_thread(pool: &PgPool, thread_id: i64, globally: bool) -> AppResult<()> {
    sqlx::query("UPDATE forum.threads SET pinned_at = now(), pinned_globally = $1 WHERE id = $2")
        .bind(globally)
        .bind(thread_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Unpin a thread.
pub async fn unpin_thread(pool: &PgPool, thread_id: i64) -> AppResult<()> {
    sqlx::query("UPDATE forum.threads SET pinned_at = NULL, pinned_globally = FALSE WHERE id = $1")
        .bind(thread_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Close a thread (read-only, no new comments).
pub async fn close_thread(pool: &PgPool, thread_id: i64) -> AppResult<()> {
    sqlx::query("UPDATE forum.threads SET closed_at = now() WHERE id = $1 AND closed_at IS NULL")
        .bind(thread_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Reopen a closed thread.
pub async fn reopen_thread(pool: &PgPool, thread_id: i64) -> AppResult<()> {
    sqlx::query("UPDATE forum.threads SET closed_at = NULL WHERE id = $1")
        .bind(thread_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Archive a thread (removed from feeds, no new comments).
pub async fn archive_thread(pool: &PgPool, thread_id: i64) -> AppResult<()> {
    sqlx::query(
        "UPDATE forum.threads SET archived_at = now() WHERE id = $1 AND archived_at IS NULL",
    )
    .bind(thread_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Hide a thread (flag-queue hide).
pub async fn hide_thread(pool: &PgPool, thread_id: i64) -> AppResult<()> {
    sqlx::query("UPDATE forum.threads SET hidden_at = now() WHERE id = $1")
        .bind(thread_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Unhide a thread.
pub async fn unhide_thread(pool: &PgPool, thread_id: i64) -> AppResult<()> {
    sqlx::query("UPDATE forum.threads SET hidden_at = NULL WHERE id = $1")
        .bind(thread_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Restore a soft-deleted thread.
pub async fn restore_thread(pool: &PgPool, thread_id: i64) -> AppResult<()> {
    sqlx::query("UPDATE forum.threads SET deleted_at = NULL, deleted_by = NULL WHERE id = $1")
        .bind(thread_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Move a thread to a different board.
pub async fn move_thread(pool: &PgPool, thread_id: i64, new_board_id: i64) -> AppResult<()> {
    sqlx::query("UPDATE forum.threads SET board_id = $1 WHERE id = $2")
        .bind(new_board_id)
        .bind(thread_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Auto-archive threads that have had no new comments for 90+ days.
///
/// Threads that are already archived, soft-deleted, or had recent activity
/// are skipped. Returns the number of threads archived.
///
/// This is intended to be called periodically from a scheduled task
/// (e.g. every hour via `tokio::time::interval`).
pub async fn auto_archive_stale(pool: &PgPool) -> i32 {
    let result = sqlx::query_scalar::<_, i64>(
        "WITH archived AS (
            UPDATE forum.threads
            SET archived_at = now()
            WHERE archived_at IS NULL
              AND deleted_at IS NULL
              AND last_activity_at < now() - INTERVAL '90 days'
            RETURNING id
        )
        SELECT count(*) FROM archived",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let count = result as i32;
    if count > 0 {
        tracing::info!(count, "auto-archived stale threads");
    }
    count
}

/// Mark a comment as the solved answer for a thread.
pub async fn set_solved_answer(pool: &PgPool, thread_id: i64, comment_id: i64) -> AppResult<()> {
    sqlx::query("UPDATE forum.threads SET solved_answer_id = $1 WHERE id = $2")
        .bind(comment_id)
        .bind(thread_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Unmark the solved answer for a thread.
pub async fn clear_solved_answer(pool: &PgPool, thread_id: i64) -> AppResult<()> {
    sqlx::query("UPDATE forum.threads SET solved_answer_id = NULL WHERE id = $1")
        .bind(thread_id)
        .execute(pool)
        .await?;
    Ok(())
}
