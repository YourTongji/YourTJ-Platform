//! Privacy-safe semantic mention notification delivery.

use serde_json::{json, Value};
use shared::AppResult;
use sqlx::types::Json;
use sqlx::{PgPool, Postgres, QueryBuilder};

pub(crate) struct MentionContext {
    pub thread_id: i64,
    pub comment_id: Option<i64>,
    pub author_handle: String,
    pub body_excerpt: String,
}

struct MentionCandidate {
    account_id: i64,
    mention_policy: String,
    payload: Value,
}

/// Resolve mention handles through Identity and insert every eligible notification in one query.
pub(crate) async fn create_mention_notifications(
    pool: &PgPool,
    actor_id: i64,
    handles: &[String],
    context: MentionContext,
) -> AppResult<usize> {
    let targets = identity::public_accounts::find_mention_targets_by_handles(pool, handles).await?;
    let candidates: Vec<MentionCandidate> = targets
        .into_iter()
        .filter(|target| target.id != actor_id)
        .map(|target| {
            let mut payload = json!({
                "threadId": context.thread_id.to_string(),
                "authorHandle": context.author_handle.clone(),
                "handle": target.handle,
                "bodyExcerpt": context.body_excerpt.clone(),
            });
            if let Some(comment_id) = context.comment_id {
                payload["commentId"] = Value::String(comment_id.to_string());
            }
            MentionCandidate {
                account_id: target.id,
                mention_policy: target.mention_policy,
                payload,
            }
        })
        .collect();
    if candidates.is_empty() {
        return Ok(0);
    }

    let candidate_ids: Vec<i64> = candidates.iter().map(|candidate| candidate.account_id).collect();
    let mut tx = pool.begin().await?;
    crate::repo::relationships::lock_pairs(&mut tx, actor_id, &candidate_ids).await?;

    let mut query = QueryBuilder::<Postgres>::new(
        "WITH mention_targets (account_id, mention_policy, payload) AS (",
    );
    query.push_values(candidates.iter(), |mut values, candidate| {
        values
            .push_bind(candidate.account_id)
            .push_bind(&candidate.mention_policy)
            .push_bind(Json(&candidate.payload));
    });
    query.push(
        ") INSERT INTO forum.notifications (account_id, type, payload) \
         SELECT target.account_id, 'mention', target.payload \
         FROM mention_targets AS target \
         LEFT JOIN forum.notification_prefs AS preference \
           ON preference.account_id = target.account_id \
         WHERE target.account_id <> ",
    );
    query.push_bind(actor_id);
    query.push(" AND NOT forum.user_pair_blocked(target.account_id, ");
    query.push_bind(actor_id);
    query.push(
        ") AND NOT EXISTS ( \
             SELECT 1 FROM forum.user_mutes AS mute \
             WHERE mute.account_id = target.account_id AND mute.muted_account_id = ",
    );
    query.push_bind(actor_id);
    query.push(
        ") AND ( \
             target.mention_policy = 'everyone' \
             OR (target.mention_policy = 'following' AND EXISTS ( \
               SELECT 1 FROM forum.user_follows AS follow \
               WHERE follow.follower_id = target.account_id AND follow.followed_id = ",
    );
    query.push_bind(actor_id);
    query.push(
        ")) \
           ) \
           AND COALESCE( \
             CASE WHEN jsonb_typeof(preference.prefs #> '{inApp,mentions}') = 'boolean' \
                  THEN (preference.prefs #>> '{inApp,mentions}')::boolean END, \
             CASE WHEN jsonb_typeof(preference.prefs -> 'mention') = 'boolean' \
                  THEN (preference.prefs ->> 'mention')::boolean END, \
             TRUE \
           ) \
         RETURNING account_id, payload",
    );
    let inserted: Vec<(i64, Json<Value>)> = query.build_query_as().fetch_all(&mut *tx).await?;
    tx.commit().await?;
    for (account_id, Json(payload)) in &inserted {
        crate::sse::publish_event(*account_id, "mention", payload.clone());
    }
    Ok(inserted.len())
}
