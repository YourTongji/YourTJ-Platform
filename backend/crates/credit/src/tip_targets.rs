//! Application boundary for resolving content targets before a controlled tip.

use std::future::Future;
use std::pin::Pin;

use shared::AppResult;
use sqlx::PgConnection;

/// Canonical content identity and recipient for a valid tip target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTipTarget {
    pub canonical_type: String,
    pub canonical_id: i64,
    pub author_id: i64,
}

/// Cross-domain resolver supplied by the API composition root.
pub trait TipTargetResolver: Send + Sync {
    /// Resolve only targets that exist, are publicly visible, and have an active
    /// recipient account. The returned future may hold row share locks on
    /// `conn` until the surrounding credit transaction commits.
    fn resolve<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        target_type: &'a str,
        target_id: i64,
    ) -> Pin<Box<dyn Future<Output = AppResult<Option<ResolvedTipTarget>>> + Send + 'a>>;
}
