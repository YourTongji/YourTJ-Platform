//! Read-only review target resolution for controlled credit tips.

use shared::AppResult;
use sqlx::PgConnection;

/// A visible review target and its author.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TipTarget {
    pub canonical_type: &'static str,
    pub canonical_id: i64,
    pub author_id: i64,
}

/// Resolve a visible review while holding share locks until the caller's
/// transaction commits.
pub async fn resolve_tip_target(
    conn: &mut PgConnection,
    target_id: i64,
) -> AppResult<Option<TipTarget>> {
    let author_id = sqlx::query_scalar::<_, i64>(
        "SELECT review.account_id \
         FROM reviews.reviews review \
         WHERE review.id = $1 AND review.status = 'visible' \
           AND review.account_id IS NOT NULL \
         FOR SHARE OF review",
    )
    .bind(target_id)
    .fetch_optional(&mut *conn)
    .await?;

    Ok(author_id.map(|author_id| TipTarget {
        canonical_type: "review",
        canonical_id: target_id,
        author_id,
    }))
}
