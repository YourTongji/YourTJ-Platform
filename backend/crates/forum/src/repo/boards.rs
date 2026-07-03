use shared::AppResult;
use sqlx::PgPool;

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
