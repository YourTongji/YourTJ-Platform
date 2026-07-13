//! Public author media projections for Forum content attribution.

use std::collections::HashMap;

use media::ImageDeliveryProjection;
use shared::AppResult;
use sqlx::PgPool;

/// Resolve active authors' current clean avatars without reading Identity or Media owner tables.
pub(crate) async fn resolve_author_avatars(
    pool: &PgPool,
    author_ids: &[i64],
) -> AppResult<HashMap<i64, ImageDeliveryProjection>> {
    let accounts = identity::public_accounts::find_public_accounts_by_ids(pool, author_ids).await?;
    let avatar_asset_ids =
        accounts.iter().filter_map(|account| account.avatar_asset_id).collect::<Vec<_>>();
    let deliveries = media::resolve_clean_image_deliveries(
        pool,
        &avatar_asset_ids,
        media::ImageVariant::Thumb256,
    )
    .await?;

    Ok(accounts
        .into_iter()
        .filter_map(|account| {
            let asset_id = account.avatar_asset_id?;
            let delivery = deliveries.get(&asset_id)?.clone();
            Some((account.id, delivery))
        })
        .collect())
}
