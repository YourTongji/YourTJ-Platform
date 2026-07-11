//! Thread state machine operations: pin, close, archive, hide, delete, restore,
//! and solved-answer management for Q&A boards.

use shared::AppResult;
use sqlx::PgPool;

/// Pin a thread to the top of its board.
pub async fn pin_thread(
    executor: impl sqlx::PgExecutor<'_>,
    thread_id: i64,
    globally: bool,
) -> AppResult<()> {
    sqlx::query("UPDATE forum.threads SET pinned_at = now(), pinned_globally = $1 WHERE id = $2")
        .bind(globally)
        .bind(thread_id)
        .execute(executor)
        .await?;
    Ok(())
}

/// Unpin a thread.
pub async fn unpin_thread(executor: impl sqlx::PgExecutor<'_>, thread_id: i64) -> AppResult<()> {
    sqlx::query("UPDATE forum.threads SET pinned_at = NULL, pinned_globally = FALSE WHERE id = $1")
        .bind(thread_id)
        .execute(executor)
        .await?;
    Ok(())
}

/// Close a thread (read-only, no new comments).
pub async fn close_thread(executor: impl sqlx::PgExecutor<'_>, thread_id: i64) -> AppResult<()> {
    sqlx::query("UPDATE forum.threads SET closed_at = now() WHERE id = $1 AND closed_at IS NULL")
        .bind(thread_id)
        .execute(executor)
        .await?;
    Ok(())
}

/// Reopen a closed thread.
pub async fn reopen_thread(executor: impl sqlx::PgExecutor<'_>, thread_id: i64) -> AppResult<()> {
    sqlx::query("UPDATE forum.threads SET closed_at = NULL WHERE id = $1")
        .bind(thread_id)
        .execute(executor)
        .await?;
    Ok(())
}

/// Archive a thread (removed from feeds, no new comments).
pub async fn archive_thread(executor: impl sqlx::PgExecutor<'_>, thread_id: i64) -> AppResult<()> {
    sqlx::query(
        "UPDATE forum.threads SET archived_at = now() WHERE id = $1 AND archived_at IS NULL",
    )
    .bind(thread_id)
    .execute(executor)
    .await?;
    Ok(())
}

/// Return an archived thread to regular feeds.
pub async fn unarchive_thread(
    executor: impl sqlx::PgExecutor<'_>,
    thread_id: i64,
) -> AppResult<()> {
    sqlx::query("UPDATE forum.threads SET archived_at = NULL WHERE id = $1")
        .bind(thread_id)
        .execute(executor)
        .await?;
    Ok(())
}

/// Hide a thread (flag-queue hide).
pub async fn hide_thread(executor: impl sqlx::PgExecutor<'_>, thread_id: i64) -> AppResult<()> {
    sqlx::query("UPDATE forum.threads SET hidden_at = now() WHERE id = $1")
        .bind(thread_id)
        .execute(executor)
        .await?;
    Ok(())
}

/// Unhide a thread.
pub async fn unhide_thread(executor: impl sqlx::PgExecutor<'_>, thread_id: i64) -> AppResult<()> {
    sqlx::query("UPDATE forum.threads SET hidden_at = NULL WHERE id = $1")
        .bind(thread_id)
        .execute(executor)
        .await?;
    Ok(())
}

/// Restore a soft-deleted thread.
pub async fn restore_thread(executor: impl sqlx::PgExecutor<'_>, thread_id: i64) -> AppResult<()> {
    sqlx::query("UPDATE forum.threads SET deleted_at = NULL, deleted_by = NULL WHERE id = $1")
        .bind(thread_id)
        .execute(executor)
        .await?;
    Ok(())
}

/// Move a thread to a different board.
pub async fn move_thread(
    executor: impl sqlx::PgExecutor<'_>,
    thread_id: i64,
    new_board_id: i64,
) -> AppResult<()> {
    sqlx::query("UPDATE forum.threads SET board_id = $1 WHERE id = $2")
        .bind(new_board_id)
        .bind(thread_id)
        .execute(executor)
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
    match auto_archive_stale_inner(pool).await {
        Ok(count) => {
            if count > 0 {
                tracing::info!(count, "auto-archived stale threads");
            }
            count
        }
        Err(error) => {
            tracing::warn!(?error, "failed to auto-archive stale threads");
            0
        }
    }
}

async fn auto_archive_stale_inner(pool: &PgPool) -> AppResult<i32> {
    let mut tx = pool.begin().await?;
    let candidates: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT id, board_id FROM forum.threads \
         WHERE archived_at IS NULL AND deleted_at IS NULL \
           AND last_activity_at < now() - INTERVAL '90 days' \
         ORDER BY id FOR UPDATE",
    )
    .fetch_all(&mut *tx)
    .await?;
    if candidates.is_empty() {
        tx.commit().await?;
        return Ok(0);
    }
    let candidate_ids: Vec<i64> = candidates.iter().map(|(thread_id, _)| *thread_id).collect();
    let candidate_board_ids: Vec<i64> = candidates.iter().map(|(_, board_id)| *board_id).collect();
    let affected_board_ids =
        super::boards::lock_boards_for_thread_count(&mut tx, &candidate_board_ids).await?;
    let thread_ids: Vec<i64> = sqlx::query_scalar(
        "UPDATE forum.threads SET archived_at = now() \
         WHERE id = ANY($1) AND archived_at IS NULL AND deleted_at IS NULL \
           AND last_activity_at < now() - INTERVAL '90 days' \
         RETURNING id",
    )
    .bind(&candidate_ids)
    .fetch_all(&mut *tx)
    .await?;
    for thread_id in &thread_ids {
        activity::contributions::deactivate_contribution(
            &mut tx,
            &format!("forum_thread:{thread_id}"),
            chrono::Utc::now(),
        )
        .await?;
        super::votes::deactivate_target_vote_contributions(
            &mut tx,
            "thread",
            *thread_id,
            chrono::Utc::now(),
        )
        .await?;
    }
    super::boards::refresh_board_thread_counts(&mut tx, &affected_board_ids).await?;
    tx.commit().await?;
    Ok(thread_ids.len() as i32)
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
