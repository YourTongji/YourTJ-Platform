//! Cross-domain lock ordering for media mutations that also inspect an account owner.

use shared::{AppError, AppResult};
use sqlx::PgConnection;

/// Lock owner account before upload and return the minimal authorization projection.
pub(crate) async fn lock_upload_owner(
    connection: &mut PgConnection,
    upload_id: i64,
) -> AppResult<(i64, identity::public_accounts::AccountAuthorizationState)> {
    let owner_id: Option<i64> =
        sqlx::query_scalar("SELECT account_id FROM media.uploads WHERE id = $1")
            .bind(upload_id)
            .fetch_optional(&mut *connection)
            .await?;
    let owner_id = owner_id.ok_or(AppError::NotFound)?;
    let owner =
        identity::public_accounts::lock_account_authorization_state_by_id(connection, owner_id)
            .await?
            .ok_or(AppError::NotFound)?;
    let locked_owner_id: Option<i64> =
        sqlx::query_scalar("SELECT account_id FROM media.uploads WHERE id = $1 FOR UPDATE")
            .bind(upload_id)
            .fetch_optional(connection)
            .await?;
    if locked_owner_id != Some(owner_id) {
        return Err(AppError::Internal(anyhow::anyhow!(
            "media upload owner changed while establishing lock order"
        )));
    }
    Ok((owner_id, owner))
}
