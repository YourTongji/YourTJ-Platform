//! Stable cache scopes for forum list and detail surfaces.

use deadpool_redis::Pool;
use sqlx::PgPool;

const BOARD_SCOPE_PREFIX: &str = "forum-board-scope";
const GLOBAL_FEED_SCOPE_PREFIX: &str = "forum-feed-scope";

pub async fn scope_generation(redis: Option<&Pool>, prefix: &str, id: &str) -> i64 {
    let Some(pool) = redis else {
        return 0;
    };
    let Ok(mut connection) = pool.get().await else {
        return 0;
    };
    redis::cmd("GET")
        .arg(format!("ver:{prefix}:{id}"))
        .query_async::<Option<i64>>(&mut connection)
        .await
        .ok()
        .flatten()
        .unwrap_or(0)
}

pub async fn board_generation(redis: Option<&Pool>, board_id: i64) -> i64 {
    scope_generation(redis, BOARD_SCOPE_PREFIX, &board_id.to_string()).await
}

pub async fn global_feed_generation(redis: Option<&Pool>) -> i64 {
    scope_generation(redis, GLOBAL_FEED_SCOPE_PREFIX, "global").await
}

pub async fn invalidate_thread_surfaces(redis: Option<&Pool>, thread_id: i64, board_id: i64) {
    shared::cache::bump_version_silent(redis, "thread", &thread_id.to_string()).await;
    shared::cache::bump_version_silent(redis, BOARD_SCOPE_PREFIX, &board_id.to_string()).await;
    shared::cache::bump_version_silent(redis, GLOBAL_FEED_SCOPE_PREFIX, "global").await;
}

pub async fn invalidate_thread_by_id(redis: Option<&Pool>, pool: &PgPool, thread_id: i64) {
    let board_id: Option<i64> =
        sqlx::query_scalar("SELECT board_id FROM forum.threads WHERE id = $1")
            .bind(thread_id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();
    if let Some(board_id) = board_id {
        invalidate_thread_surfaces(redis, thread_id, board_id).await;
    } else {
        shared::cache::bump_version_silent(redis, "thread", &thread_id.to_string()).await;
        shared::cache::bump_version_silent(redis, GLOBAL_FEED_SCOPE_PREFIX, "global").await;
    }
}
