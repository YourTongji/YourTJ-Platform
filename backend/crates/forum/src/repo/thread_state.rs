//! Thread state machine operations: pin, close, archive, hide, delete, restore.

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
