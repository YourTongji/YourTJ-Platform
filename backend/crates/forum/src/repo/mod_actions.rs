//! Staff action log: record and list mod/admin actions.

use crate::models::ModActionRow;
use shared::AppResult;
use sqlx::PgPool;

/// Insert a mod action row. Must be called within the same transaction as the operation.
pub async fn insert_mod_action(
    executor: impl sqlx::PgExecutor<'_>,
    actor_id: i64,
    action: &str,
    target_type: &str,
    target_id: i64,
    reason: Option<&str>,
    metadata: Option<&serde_json::Value>,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO forum.mod_actions (actor_id, action, target_type, target_id, reason, metadata) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(actor_id)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(reason)
    .bind(metadata)
    .execute(executor)
    .await?;
    Ok(())
}

/// List mod actions, cursor-paginated by id DESC (newest first).
pub async fn list_mod_actions(
    pool: &PgPool,
    cursor: Option<i64>,
    limit: i64,
) -> AppResult<(Vec<ModActionRow>, Option<i64>)> {
    let since_id = cursor.unwrap_or(i64::MAX);

    let rows: Vec<ModActionRow> = sqlx::query_as(
        "SELECT id, actor_id, action, target_type, target_id, reason, metadata, created_at \
         FROM forum.mod_actions \
         WHERE id < $1 \
         ORDER BY id DESC \
         LIMIT $2",
    )
    .bind(since_id)
    .bind(limit + 1)
    .fetch_all(pool)
    .await?;

    let has_more = rows.len() > limit as usize;
    let items = if has_more { rows[..limit as usize].to_vec() } else { rows };
    let next_cursor = items.last().map(|r| r.id);

    Ok((items, next_cursor))
}
