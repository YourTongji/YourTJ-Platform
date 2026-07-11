use shared::{AppError, AppResult};
use sqlx::{PgConnection, PgPool};

use crate::models::BoardRow;

/// List all boards.
pub async fn list_boards(pool: &PgPool) -> AppResult<Vec<BoardRow>> {
    let rows = sqlx::query_as::<_, BoardRow>(
        "SELECT id, slug, name, parent_id, description, position, is_locked, \
                is_qa, min_trust_to_post, thread_count \
         FROM forum.boards ORDER BY id",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Find a single board by id.
pub async fn find_board(pool: &PgPool, id: i64) -> AppResult<Option<BoardRow>> {
    let row = sqlx::query_as::<_, BoardRow>(
        "SELECT id, slug, name, parent_id, description, position, is_locked, \
                is_qa, min_trust_to_post, thread_count \
         FROM forum.boards WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Lock boards in a stable order before a thread visibility transition updates their counters.
pub(crate) async fn lock_boards_for_thread_count(
    connection: &mut PgConnection,
    board_ids: &[i64],
) -> AppResult<Vec<i64>> {
    let mut unique_ids = board_ids.to_vec();
    unique_ids.sort_unstable();
    unique_ids.dedup();
    if unique_ids.is_empty() {
        return Ok(unique_ids);
    }

    let locked_ids: Vec<i64> =
        sqlx::query_scalar("SELECT id FROM forum.boards WHERE id = ANY($1) ORDER BY id FOR UPDATE")
            .bind(&unique_ids)
            .fetch_all(connection)
            .await?;
    if locked_ids.len() != unique_ids.len() {
        return Err(AppError::NotFound);
    }
    Ok(unique_ids)
}

/// Recalculate visible thread counters for boards already locked by the caller.
pub(crate) async fn refresh_board_thread_counts(
    connection: &mut PgConnection,
    board_ids: &[i64],
) -> AppResult<()> {
    if board_ids.is_empty() {
        return Ok(());
    }
    sqlx::query(
        "UPDATE forum.boards board SET thread_count = ( \
           SELECT COUNT(*)::int FROM forum.threads thread \
           WHERE thread.board_id = board.id AND thread.status = 'visible' \
             AND thread.deleted_at IS NULL AND thread.hidden_at IS NULL \
             AND thread.archived_at IS NULL \
         ) WHERE board.id = ANY($1)",
    )
    .bind(board_ids)
    .execute(connection)
    .await?;
    Ok(())
}
