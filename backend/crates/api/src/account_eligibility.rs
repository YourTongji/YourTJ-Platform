//! Wires Credit's counterparty eligibility boundary to the Identity owner API.

use credit::account_eligibility::{AccountEligibilityFuture, AccountEligibilityResolver};
use sqlx::PgConnection;

pub(crate) struct IdentityAccountEligibilityResolver;

impl AccountEligibilityResolver for IdentityAccountEligibilityResolver {
    fn is_eligible_on<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        account_id: i64,
    ) -> AccountEligibilityFuture<'a> {
        Box::pin(identity::public_accounts::is_credit_recipient_eligible(conn, account_id))
    }

    fn are_eligible_on<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        account_ids: &'a [i64],
    ) -> AccountEligibilityFuture<'a> {
        Box::pin(identity::public_accounts::lock_active_interaction_accounts(conn, account_ids))
    }
}
