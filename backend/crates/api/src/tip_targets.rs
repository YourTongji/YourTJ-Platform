//! Composition adapter for content-owned credit tip target resolvers.

use std::future::Future;
use std::pin::Pin;

use credit::tip_targets::{ResolvedTipTarget, TipTargetResolver};
use shared::AppResult;
use sqlx::PgConnection;

pub(crate) struct ContentTipTargetResolver;

impl TipTargetResolver for ContentTipTargetResolver {
    fn resolve<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        target_type: &'a str,
        target_id: i64,
    ) -> Pin<Box<dyn Future<Output = AppResult<Option<ResolvedTipTarget>>> + Send + 'a>> {
        Box::pin(async move {
            let target =
                match target_type {
                    "review" => reviews::tip_targets::resolve_tip_target(conn, target_id)
                        .await?
                        .map(|target| ResolvedTipTarget {
                            canonical_type: target.canonical_type.to_string(),
                            canonical_id: target.canonical_id,
                            author_id: target.author_id,
                        }),
                    "thread" | "comment" => {
                        forum::tip_targets::resolve_tip_target(conn, target_type, target_id)
                            .await?
                            .map(|target| ResolvedTipTarget {
                                canonical_type: target.canonical_type.to_string(),
                                canonical_id: target.canonical_id,
                                author_id: target.author_id,
                            })
                    }
                    _ => None,
                };
            let Some(target) = target else {
                return Ok(None);
            };
            if !identity::public_accounts::is_credit_recipient_eligible(conn, target.author_id)
                .await?
            {
                return Ok(None);
            }
            Ok(Some(target))
        })
    }
}
