use credit::wallet_keys::{VerificationPublicKeys, WalletKeyFuture, WalletKeyResolver};
use sqlx::{PgConnection, PgPool};

pub(crate) struct AccountWalletKeyResolver;

impl WalletKeyResolver for AccountWalletKeyResolver {
    fn active_public_key<'a>(
        &'a self,
        pool: &'a PgPool,
        account_id: i64,
    ) -> WalletKeyFuture<'a, Option<String>> {
        Box::pin(identity::wallet_keys::active_public_key(pool, account_id))
    }

    fn active_public_key_on<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        account_id: i64,
    ) -> WalletKeyFuture<'a, Option<String>> {
        Box::pin(identity::wallet_keys::active_public_key_on(conn, account_id))
    }

    fn verification_public_keys_on<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        account_ids: &'a [i64],
    ) -> WalletKeyFuture<'a, VerificationPublicKeys> {
        Box::pin(identity::wallet_keys::verification_public_keys_on(conn, account_ids))
    }
}
