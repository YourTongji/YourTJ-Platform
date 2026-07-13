//! Minimal durable facts for high-value credential events.

use shared::AppResult;
use sqlx::{PgConnection, PgPool};

#[derive(Clone, Copy)]
pub(crate) enum SecurityEventKind {
    PasswordSet,
    PasswordChanged,
    PasswordReset,
    RefreshReplayDetected,
}

impl SecurityEventKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::PasswordSet => "password_set",
            Self::PasswordChanged => "password_changed",
            Self::PasswordReset => "password_reset",
            Self::RefreshReplayDetected => "refresh_replay_detected",
        }
    }
}

pub(crate) async fn record_tx(
    connection: &mut PgConnection,
    account_id: i64,
    event: SecurityEventKind,
    subject_session_id: Option<i64>,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO identity.security_events \
         (account_id, event_type, subject_session_id) VALUES ($1, $2, $3) \
         ON CONFLICT DO NOTHING",
    )
    .bind(account_id)
    .bind(event.as_str())
    .bind(subject_session_id)
    .execute(connection)
    .await?;
    Ok(())
}

pub(crate) async fn purge_expired(pool: &PgPool) -> AppResult<u64> {
    let removed = sqlx::query("DELETE FROM identity.security_events WHERE expires_at <= now()")
        .execute(pool)
        .await?
        .rows_affected();
    Ok(removed)
}
