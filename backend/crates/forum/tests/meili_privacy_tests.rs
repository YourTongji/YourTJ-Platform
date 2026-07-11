//! Database visibility tests for the forum Meilisearch projection.

#[path = "helpers/mod.rs"]
mod helpers;

use axum::routing::post;
use axum::{Json, Router};
use helpers::{create_test_account, create_test_app};
use sqlx::PgPool;

async fn seed_thread(
    pool: &PgPool,
    author_id: i64,
    title: &str,
    status: &str,
    hidden: bool,
    archived: bool,
    deleted: bool,
) -> i64 {
    sqlx::query_scalar(
        "INSERT INTO forum.threads (board_id, author_id, title, body, status, \
                                    hidden_at, archived_at, deleted_at) \
         VALUES (1, $1, $2, $3, $4, \
                 CASE WHEN $5 THEN now() ELSE NULL END, \
                 CASE WHEN $6 THEN now() ELSE NULL END, \
                 CASE WHEN $7 THEN now() ELSE NULL END) \
         RETURNING id",
    )
    .bind(author_id)
    .bind(title)
    .bind(format!("body for {title}"))
    .bind(status)
    .bind(hidden)
    .bind(archived)
    .bind(deleted)
    .fetch_one(pool)
    .await
    .expect("seed forum thread")
}

#[tokio::test]
async fn public_search_reconstructs_hits_and_omits_every_non_public_thread_state() {
    let (pool, _) = create_test_app().await;
    let (author_id, _) =
        create_test_account(&pool, "search-author@tongji.edu.cn", "search_author").await;

    let visible_id = seed_thread(&pool, author_id, "visible", "visible", false, false, false).await;
    let hidden_id = seed_thread(&pool, author_id, "hidden", "visible", true, false, false).await;
    let archived_id =
        seed_thread(&pool, author_id, "archived", "visible", false, true, false).await;
    let deleted_id = seed_thread(&pool, author_id, "deleted", "visible", false, false, true).await;
    let pending_id = seed_thread(&pool, author_id, "pending", "pending", false, false, false).await;

    let tag_id: i64 = sqlx::query_scalar(
        "INSERT INTO forum.tags (slug, name) VALUES ('privacy', 'Privacy') \
         ON CONFLICT (slug) DO UPDATE SET name = EXCLUDED.name RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("seed tag");
    sqlx::query("INSERT INTO forum.thread_tags (thread_id, tag_id) VALUES ($1, $2)")
        .bind(visible_id)
        .bind(tag_id)
        .execute(&pool)
        .await
        .expect("tag visible thread");

    let search_response = serde_json::json!({
        "hits": [
            {"id": hidden_id.to_string(), "title": "stale hidden title"},
            {"id": archived_id.to_string(), "title": "stale archived title"},
            {"id": deleted_id.to_string(), "title": "stale deleted title"},
            {"id": pending_id.to_string(), "title": "stale pending title"},
            {"id": visible_id.to_string(), "title": "stale visible title"}
        ],
        "offset": 0,
        "limit": 40,
        "estimatedTotalHits": 5,
        "processingTimeMs": 1,
        "query": "visible"
    });
    let meili = Router::new().route(
        "/indexes/forum_threads/search",
        post(move || {
            let response = search_response.clone();
            async move { Json(response) }
        }),
    );
    let listener =
        tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind fake meilisearch");
    let address = listener.local_addr().expect("fake meilisearch address");
    let server = tokio::spawn(async move {
        axum::serve(listener, meili).await.expect("serve fake meilisearch");
    });

    let results =
        forum::meili::search_threads(&pool, &format!("http://{address}"), "", "visible", 10)
            .await
            .expect("search public threads");
    server.abort();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["id"], visible_id.to_string());
    assert_eq!(results[0]["title"], "visible");
    assert_eq!(results[0]["bodyExcerpt"], "body for visible");
    assert_eq!(results[0]["tags"], serde_json::json!(["privacy"]));
    assert_eq!(results[0]["authorHandle"], "search_author");
}
