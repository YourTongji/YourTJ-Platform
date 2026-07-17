//! Application boundary for locking Identity-owned account eligibility in Credit transactions.

use std::future::Future;
use std::pin::Pin;

use shared::AppResult;
use sqlx::PgConnection;

/// Boxed async result returned by the object-safe account eligibility boundary.
pub type AccountEligibilityFuture<'a> = Pin<Box<dyn Future<Output = AppResult<bool>> + Send + 'a>>;

/// Resolves Identity-owned eligibility while holding account lifecycle barriers.
pub trait AccountEligibilityResolver: Send + Sync {
    /// Return whether one account may participate on the business transaction's connection.
    fn is_eligible_on<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        account_id: i64,
    ) -> AccountEligibilityFuture<'a>;

    /// Lock a sorted account set and return whether every account may participate.
    fn are_eligible_on<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        account_ids: &'a [i64],
    ) -> AccountEligibilityFuture<'a>;
}
