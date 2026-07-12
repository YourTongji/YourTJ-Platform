//! Privacy and relationship enforcement for ranked user search candidates.

use std::collections::{HashMap, HashSet};

use serde::Serialize;
use shared::AppResult;
use sqlx::PgPool;

/// One account result after current privacy and relationship checks.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSearchHit {
    pub id: String,
    pub handle: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub role: String,
    pub follower_count: i32,
    pub following: bool,
}

/// Reconstruct ranked user candidates under current profile and relationship policy.
pub async fn load_user_hits(
    pool: &PgPool,
    candidate_ids: &[i64],
    viewer_id: Option<i64>,
    limit: usize,
) -> AppResult<Vec<UserSearchHit>> {
    if candidate_ids.is_empty() || limit == 0 {
        return Ok(Vec::new());
    }
    let accounts = identity::public_accounts::find_public_accounts_by_ids(pool, candidate_ids)
        .await?
        .into_iter()
        .filter(|account| {
            account.is_campus_verified
                && account.discoverable
                && match account.profile_visibility.as_str() {
                    "public" => true,
                    "campus" => viewer_id.is_some(),
                    "only_me" => viewer_id == Some(account.id),
                    _ => false,
                }
        })
        .map(|account| (account.id, account))
        .collect::<HashMap<_, _>>();
    let hidden_ids: HashSet<i64> = match viewer_id {
        Some(viewer_id) => sqlx::query_scalar(
            "SELECT candidate_id FROM unnest($1::bigint[]) candidate_id \
             WHERE candidate_id <> $2 AND forum.user_content_hidden($2, candidate_id)",
        )
        .bind(candidate_ids)
        .bind(viewer_id)
        .fetch_all(pool)
        .await?
        .into_iter()
        .collect(),
        None => HashSet::new(),
    };
    let social_counts: HashMap<i64, i32> = sqlx::query_as::<_, (i64, i32)>(
        "SELECT account_id, follower_count FROM forum.user_social_stats \
         WHERE account_id = ANY($1)",
    )
    .bind(candidate_ids)
    .fetch_all(pool)
    .await?
    .into_iter()
    .collect();
    let following_ids: HashSet<i64> = match viewer_id {
        Some(viewer_id) => sqlx::query_scalar(
            "SELECT followed_id FROM forum.user_follows \
             WHERE follower_id = $1 AND followed_id = ANY($2)",
        )
        .bind(viewer_id)
        .bind(candidate_ids)
        .fetch_all(pool)
        .await?
        .into_iter()
        .collect(),
        None => HashSet::new(),
    };
    let asset_ids =
        accounts.values().filter_map(|account| account.avatar_asset_id).collect::<Vec<_>>();
    let avatar_urls = media::resolve_clean_profile_images(pool, &asset_ids).await?;
    let mut items = Vec::new();
    for account_id in candidate_ids {
        if hidden_ids.contains(account_id) {
            continue;
        }
        let Some(account) = accounts.get(account_id) else {
            continue;
        };
        items.push(UserSearchHit {
            id: account.id.to_string(),
            handle: account.handle.clone(),
            display_name: account.display_name.clone(),
            avatar_url: account.avatar_asset_id.and_then(|id| avatar_urls.get(&id).cloned()),
            role: account.role.clone(),
            follower_count: social_counts.get(account_id).copied().unwrap_or_default(),
            following: following_ids.contains(account_id),
        });
        if items.len() == limit {
            break;
        }
    }
    Ok(items)
}
