//! Handler-to-database coverage for the canonical user-following feed.

#[path = "helpers/mod.rs"]
mod helpers;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use helpers::{create_test_account, create_test_app, read_json};
use sqlx::PgPool;
use tower::ServiceExt;

async fn seed_thread(pool: &PgPool, author_id: i64, title: &str, body: &str) -> i64 {
    sqlx::query_scalar(
        "INSERT INTO forum.threads (board_id, author_id, title, body) \
         VALUES (1, $1, $2, $3) RETURNING id",
    )
    .bind(author_id)
    .bind(title)
    .bind(body)
    .fetch_one(pool)
    .await
    .expect("seed thread")
}

async fn get_feed(
    app: &axum::Router,
    token: Option<&str>,
    query: &str,
) -> axum::response::Response {
    let mut request = Request::builder().uri(format!("/api/v2/forum/threads?{query}"));
    if let Some(token) = token {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    app.clone()
        .oneshot(request.body(Body::empty()).expect("build feed request"))
        .await
        .expect("feed response")
}

#[tokio::test]
async fn following_feed_enforces_relationship_visibility_and_stable_viewer_aware_pages() {
    let (pool, app) = create_test_app().await;
    let (viewer_id, viewer_token) =
        create_test_account(&pool, "following-viewer@tongji.edu.cn", "following-viewer").await;
    let (followed_id, _) =
        create_test_account(&pool, "following-author@tongji.edu.cn", "following-author").await;
    let (unfollowed_id, _) =
        create_test_account(&pool, "unfollowed-author@tongji.edu.cn", "unfollowed-author").await;
    let (muted_id, _) =
        create_test_account(&pool, "muted-author@tongji.edu.cn", "muted-author").await;
    let (blocked_id, _) =
        create_test_account(&pool, "blocked-author@tongji.edu.cn", "blocked-author").await;
    let (suspended_id, _) =
        create_test_account(&pool, "suspended-author@tongji.edu.cn", "suspended-author").await;

    sqlx::query(
        "INSERT INTO forum.user_follows (follower_id, followed_id) \
         VALUES ($1, $2), ($1, $3), ($1, $4), ($1, $5)",
    )
    .bind(viewer_id)
    .bind(followed_id)
    .bind(muted_id)
    .bind(blocked_id)
    .bind(suspended_id)
    .execute(&pool)
    .await
    .expect("seed follows");
    sqlx::query("INSERT INTO forum.user_mutes (account_id, muted_account_id) VALUES ($1, $2)")
        .bind(viewer_id)
        .bind(muted_id)
        .execute(&pool)
        .await
        .expect("seed mute");
    sqlx::query("INSERT INTO forum.user_ignores (account_id, ignored_account_id) VALUES ($1, $2)")
        .bind(blocked_id)
        .bind(viewer_id)
        .execute(&pool)
        .await
        .expect("seed reverse block");
    sqlx::query(
        "INSERT INTO identity.sanctions (account_id, kind, reason, issued_by) \
         VALUES ($1, 'suspend', 'test suspension', $2)",
    )
    .bind(suspended_id)
    .bind(viewer_id)
    .execute(&pool)
    .await
    .expect("seed suspension");

    let newest_id = seed_thread(
        &pool,
        followed_id,
        "Newest followed thread",
        "**Canonical** following excerpt",
    )
    .await;
    let older_id = seed_thread(&pool, followed_id, "Older followed thread", "Older body").await;
    let unrelated_ids = vec![
        seed_thread(&pool, unfollowed_id, "Not followed", "not followed").await,
        seed_thread(&pool, muted_id, "Muted followed", "muted").await,
        seed_thread(&pool, blocked_id, "Blocked followed", "blocked").await,
        seed_thread(&pool, suspended_id, "Suspended followed", "suspended").await,
    ];
    sqlx::query(
        "UPDATE forum.threads SET created_at = CASE id \
           WHEN $1 THEN now() - interval '1 minute' \
           WHEN $2 THEN now() - interval '2 minutes' \
           ELSE now() END, \
           content_format = CASE WHEN id = $1 THEN 'markdown_v1' ELSE content_format END \
         WHERE id = ANY($3)",
    )
    .bind(newest_id)
    .bind(older_id)
    .bind(
        std::iter::once(newest_id)
            .chain(std::iter::once(older_id))
            .chain(unrelated_ids)
            .collect::<Vec<_>>(),
    )
    .execute(&pool)
    .await
    .expect("order feed fixtures");
    sqlx::query(
        "INSERT INTO forum.votes (post_type, post_id, account_id, value) \
         VALUES ('thread', $1, $2, 1)",
    )
    .bind(newest_id)
    .bind(viewer_id)
    .execute(&pool)
    .await
    .expect("seed viewer vote");
    sqlx::query(
        "INSERT INTO forum.bookmarks (account_id, target_type, target_id) \
         VALUES ($1, 'thread', $2)",
    )
    .bind(viewer_id)
    .bind(newest_id)
    .execute(&pool)
    .await
    .expect("seed viewer bookmark");
    let tag_id: i64 = sqlx::query_scalar(
        "INSERT INTO forum.tags (slug, name) VALUES ('following-focus', 'Following Focus') \
         RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("seed feed tag");
    sqlx::query("INSERT INTO forum.thread_tags (thread_id, tag_id) VALUES ($1, $2)")
        .bind(older_id)
        .bind(tag_id)
        .execute(&pool)
        .await
        .expect("tag older followed thread");

    let first_page = get_feed(&app, Some(&viewer_token), "sort=following&limit=1").await;
    assert_eq!(first_page.status(), StatusCode::OK);
    let first_page = read_json(first_page).await;
    assert_eq!(first_page["items"].as_array().expect("items").len(), 1);
    assert_eq!(first_page["items"][0]["id"], newest_id.to_string());
    assert_eq!(first_page["items"][0]["bodyExcerpt"], "Canonical following excerpt");
    assert_eq!(first_page["items"][0]["viewerVote"], "up");
    assert_eq!(first_page["items"][0]["isBookmarked"], true);
    let cursor = first_page["nextCursor"].as_str().expect("stable cursor");

    let second_page =
        get_feed(&app, Some(&viewer_token), &format!("sort=following&limit=1&cursor={cursor}"))
            .await;
    assert_eq!(second_page.status(), StatusCode::OK);
    let second_page = read_json(second_page).await;
    assert_eq!(second_page["items"][0]["id"], older_id.to_string());
    assert_eq!(second_page["hasMore"], false);

    let tagged = get_feed(&app, Some(&viewer_token), "sort=following&tag=following-focus").await;
    assert_eq!(tagged.status(), StatusCode::OK);
    let tagged = read_json(tagged).await;
    assert_eq!(tagged["items"].as_array().expect("tagged items").len(), 1);
    assert_eq!(tagged["items"][0]["id"], older_id.to_string());

    assert_eq!(get_feed(&app, None, "sort=following").await.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        get_feed(&app, Some(&viewer_token), "sort=following&cursor=invalid").await.status(),
        StatusCode::BAD_REQUEST
    );
}
