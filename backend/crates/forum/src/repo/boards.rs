use shared::AppResult;
use sqlx::PgPool;

use crate::models::BoardRow;

/// List all boards.
pub async fn list_boards(pool: &PgPool) -> AppResult<Vec<BoardRow>> {
    let rows = sqlx::query_as::<_, BoardRow>("SELECT id, slug, name FROM forum.boards ORDER BY id")
        .fetch_all(pool)
        .await?;
    Ok(rows)
}
