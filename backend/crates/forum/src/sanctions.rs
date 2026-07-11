//! Forum-side wrappers around identity-owned sanction enforcement.

use sqlx::PgPool;

use shared::AppResult;

/// Check if an account is currently silenced (can't write).
/// Delegates to identity crate's check.
pub async fn require_can_post(
    redis: Option<&deadpool_redis::Pool>,
    pool: &PgPool,
    account_id: i64,
) -> AppResult<()> {
    if identity::sanctions::is_silenced(redis, pool, account_id).await? {
        return Err(shared::AppError::Forbidden);
    }
    Ok(())
}
