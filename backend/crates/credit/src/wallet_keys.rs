//! Application boundary for reading Identity-owned wallet verification keys.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use shared::AppResult;
use sqlx::{PgConnection, PgPool};

/// Owned account-to-key projection used while verifying historical ledger signatures.
pub type VerificationPublicKeys = HashMap<i64, Vec<String>>;

/// Boxed async result returned by object-safe wallet-key owner boundaries.
pub type WalletKeyFuture<'a, T> = Pin<Box<dyn Future<Output = AppResult<T>> + Send + 'a>>;

/// Resolves the sole active key and retained historical verification keys.
pub trait WalletKeyResolver: Send + Sync {
    fn active_public_key<'a>(
        &'a self,
        pool: &'a PgPool,
        account_id: i64,
    ) -> WalletKeyFuture<'a, Option<String>>;

    /// Resolve the active key on the business transaction's connection.
    fn active_public_key_on<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        account_id: i64,
    ) -> WalletKeyFuture<'a, Option<String>>;

    /// Resolve active and revoked keys retained for historical ledger verification.
    fn verification_public_keys_on<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        account_ids: &'a [i64],
    ) -> WalletKeyFuture<'a, VerificationPublicKeys>;
}
