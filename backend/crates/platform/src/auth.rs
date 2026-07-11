use axum::http::{header, HeaderMap};
use shared::auth::Capability;
use shared::{AppError, AppResult, AppState, AuthAccount};

pub(crate) async fn required_account(
    headers: &HeaderMap,
    state: &AppState,
) -> AppResult<AuthAccount> {
    identity::auth_middleware::authenticate(
        headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)
}

pub(crate) async fn optional_account(
    headers: &HeaderMap,
    state: &AppState,
) -> AppResult<Option<AuthAccount>> {
    if !headers.contains_key(header::AUTHORIZATION) {
        return Ok(None);
    }
    required_account(headers, state).await.map(Some)
}

pub(crate) async fn staff_account(
    headers: &HeaderMap,
    state: &AppState,
    capability: Capability,
) -> AppResult<AuthAccount> {
    let account = required_account(headers, state).await?;
    account.require_capability(capability).map_err(|_| AppError::Forbidden)?;
    Ok(account)
}

pub(crate) fn is_staff(account: Option<&AuthAccount>) -> bool {
    account.is_some_and(|account| account.role == "mod" || account.role == "admin")
}
