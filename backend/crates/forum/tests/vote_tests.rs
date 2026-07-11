//! Integration tests for the forum domain — votes.

#[path = "helpers/mod.rs"]
mod helpers;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use helpers::{create_test_account, create_test_app};
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;

async fn vote_request(
    app: &axum::Router,
    post_id: i64,
    post_type: &str,
    token: &str,
) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/v2/forum/posts/{post_id}/vote"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(json!({ "value": "up", "postType": post_type }).to_string()))
                .expect("build vote request"),
        )
        .await
        .expect("vote response")
}

/// Seed a thread and return its id.
async fn seed_thread(pool: &PgPool, author_id: i64) -> i64 {
    let (id,): (i64,) = sqlx::query_as(
        "INSERT INTO forum.threads (board_id, author_id, title) \
         VALUES (1, $1, 'Vote Test') RETURNING id",
    )
    .bind(author_id)
    .fetch_one(pool)
    .await
    .expect("seed thread");
    id
}

/// Seed a comment and return its id.
async fn seed_comment(pool: &PgPool, thread_id: i64, author_id: i64, body: &str) -> i64 {
    let (id,): (i64,) = sqlx::query_as(
        "INSERT INTO forum.comments (thread_id, author_id, body, path) \
         VALUES ($1, $2, $3, 'dead') RETURNING id",
    )
    .bind(thread_id)
    .bind(author_id)
    .bind(body)
    .fetch_one(pool)
    .await
    .expect("seed comment");
    id
}

/// ── vote on threads ──────────────────────────────────────────────────────

#[tokio::test]
async fn test_vote_thread_up() {
    let (pool, app) = create_test_app().await;
    let (author_id, _) = create_test_account(&pool, "julia@tongji.edu.cn", "julia").await;
    let (voter_id, token) =
        create_test_account(&pool, "julia-voter@tongji.edu.cn", "julia-voter").await;
    let thread_id = seed_thread(&pool, author_id).await;

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/v2/forum/posts/{thread_id}/vote"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(json!({"value": "up", "postType": "thread"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    let vote_count: i32 = sqlx::query_scalar("SELECT vote_count FROM forum.threads WHERE id = $1")
        .bind(thread_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(vote_count, 1);
    let activity_count: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(likes_given), 0)::bigint \
         FROM activity.daily_counts WHERE account_id = $1",
    )
    .bind(voter_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(activity_count, 1);
}

#[tokio::test]
async fn test_vote_thread_down() {
    let (pool, app) = create_test_app().await;
    let (author_id, _) = create_test_account(&pool, "karl@tongji.edu.cn", "karl").await;
    let (voter_id, token) =
        create_test_account(&pool, "karl-voter@tongji.edu.cn", "karl-voter").await;
    let thread_id = seed_thread(&pool, author_id).await;

    // Up first, then down.
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/v2/forum/posts/{thread_id}/vote"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(json!({"value": "up", "postType": "thread"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/v2/forum/posts/{thread_id}/vote"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(json!({"value": "down", "postType": "thread"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    let vote_count: i32 = sqlx::query_scalar("SELECT vote_count FROM forum.threads WHERE id = $1")
        .bind(thread_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    // One-vote-per-user with no stacking: switching up → down yields -1
    // (see docs/ARCH_REVIEW_AND_E2E_PLAN.md §vote).
    assert_eq!(vote_count, -1);
    let activity_count: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(likes_given), 0)::bigint \
         FROM activity.daily_counts WHERE account_id = $1",
    )
    .bind(voter_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(activity_count, 0);
}

/// ── vote on comment ──────────────────────────────────────────────────────

#[tokio::test]
async fn test_vote_comment_up() {
    let (pool, app) = create_test_app().await;
    let (author_id, _) = create_test_account(&pool, "liam@tongji.edu.cn", "liam").await;
    let (_, token) = create_test_account(&pool, "liam-voter@tongji.edu.cn", "liam-voter").await;
    let thread_id = seed_thread(&pool, author_id).await;
    let comment_id = seed_comment(&pool, thread_id, author_id, "Nice post").await;

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/v2/forum/posts/{comment_id}/vote"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(json!({"value": "up", "postType": "comment"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    let vote_count: i32 = sqlx::query_scalar("SELECT vote_count FROM forum.comments WHERE id = $1")
        .bind(comment_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(vote_count, 1);
}

#[tokio::test]
async fn test_vote_own_content_is_rejected() {
    let (pool, app) = create_test_app().await;
    let (author_id, token) =
        create_test_account(&pool, "self-vote@tongji.edu.cn", "self-vote").await;
    let thread_id = seed_thread(&pool, author_id).await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/v2/forum/posts/{thread_id}/vote"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(json!({"value": "up", "postType": "thread"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

/// ── vote requires auth ───────────────────────────────────────────────────

#[tokio::test]
async fn test_vote_requires_auth() {
    let (_, app) = create_test_app().await;

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/forum/posts/1/vote")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({"value": "up", "postType": "thread"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// ── vote not found ───────────────────────────────────────────────────────

#[tokio::test]
async fn test_vote_nonexistent_post() {
    let (pool, app) = create_test_app().await;
    let (_, token) = create_test_account(&pool, "mia@tongji.edu.cn", "mia").await;

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/forum/posts/99999/vote")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(json!({"value": "up", "postType": "thread"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn votes_reject_unavailable_targets_without_activity() {
    let (pool, app) = create_test_app().await;
    let (author_id, _) =
        create_test_account(&pool, "hidden-vote-author@tongji.edu.cn", "hidden-vote-author").await;
    let (voter_id, voter_token) =
        create_test_account(&pool, "hidden-vote-voter@tongji.edu.cn", "hidden-vote-voter").await;

    for column in ["hidden_at", "deleted_at", "archived_at"] {
        let thread_id = seed_thread(&pool, author_id).await;
        sqlx::query(&format!("UPDATE forum.threads SET {column} = now() WHERE id = $1"))
            .bind(thread_id)
            .execute(&pool)
            .await
            .expect("set thread unavailable");
        let response = vote_request(&app, thread_id, "thread", &voter_token).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "thread state {column}");
    }

    let hidden_comment_thread = seed_thread(&pool, author_id).await;
    let hidden_comment =
        seed_comment(&pool, hidden_comment_thread, author_id, "hidden comment").await;
    sqlx::query("UPDATE forum.comments SET hidden_at = now() WHERE id = $1")
        .bind(hidden_comment)
        .execute(&pool)
        .await
        .expect("hide comment");
    assert_eq!(
        vote_request(&app, hidden_comment, "comment", &voter_token).await.status(),
        StatusCode::NOT_FOUND
    );

    let deleted_comment_thread = seed_thread(&pool, author_id).await;
    let deleted_comment =
        seed_comment(&pool, deleted_comment_thread, author_id, "deleted comment").await;
    sqlx::query("UPDATE forum.comments SET deleted_at = now() WHERE id = $1")
        .bind(deleted_comment)
        .execute(&pool)
        .await
        .expect("delete comment");
    assert_eq!(
        vote_request(&app, deleted_comment, "comment", &voter_token).await.status(),
        StatusCode::NOT_FOUND
    );

    let hidden_parent_thread = seed_thread(&pool, author_id).await;
    let child_comment = seed_comment(&pool, hidden_parent_thread, author_id, "hidden parent").await;
    sqlx::query("UPDATE forum.threads SET hidden_at = now() WHERE id = $1")
        .bind(hidden_parent_thread)
        .execute(&pool)
        .await
        .expect("hide parent thread");
    assert_eq!(
        vote_request(&app, child_comment, "comment", &voter_token).await.status(),
        StatusCode::NOT_FOUND
    );

    let vote_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM forum.votes")
        .fetch_one(&pool)
        .await
        .expect("vote count");
    assert_eq!(vote_count, 0);
    let activity_count: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(likes_given), 0)::bigint \
         FROM activity.daily_counts WHERE account_id = $1",
    )
    .bind(voter_id)
    .fetch_one(&pool)
    .await
    .expect("vote activity");
    assert_eq!(activity_count, 0);
}

#[tokio::test]
async fn moderation_hiding_and_restoring_target_reverses_positive_vote_activity() {
    let (pool, app) = create_test_app().await;
    let (author_id, _) =
        create_test_account(&pool, "vote-mod-author@tongji.edu.cn", "vote-mod-author").await;
    let (voter_id, voter_token) =
        create_test_account(&pool, "vote-mod-voter@tongji.edu.cn", "vote-mod-voter").await;
    let (moderator_id, moderator_token) =
        create_test_account(&pool, "vote-mod@tongji.edu.cn", "vote-mod").await;
    sqlx::query("UPDATE identity.accounts SET role = 'mod' WHERE id = $1")
        .bind(moderator_id)
        .execute(&pool)
        .await
        .expect("promote moderator");
    let thread_id = seed_thread(&pool, author_id).await;
    assert_eq!(
        vote_request(&app, thread_id, "thread", &voter_token).await.status(),
        StatusCode::OK
    );

    let activity_count = |pool: PgPool| async move {
        sqlx::query_scalar::<_, i64>(
            "SELECT COALESCE(SUM(likes_given), 0)::bigint \
             FROM activity.daily_counts WHERE account_id = $1",
        )
        .bind(voter_id)
        .fetch_one(&pool)
        .await
        .expect("vote activity")
    };
    assert_eq!(activity_count(pool.clone()).await, 1);

    for (action, expected_count) in [("hide", 0), ("unhide", 1)] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("/api/v2/admin/forum/threads/{thread_id}/{action}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::AUTHORIZATION, format!("Bearer {moderator_token}"))
                    .body(Body::from(
                        json!({ "reason": "verified moderation transition" }).to_string(),
                    ))
                    .expect("build moderation request"),
            )
            .await
            .expect("moderation response");
        assert_eq!(response.status(), StatusCode::OK, "action {action}");
        assert_eq!(activity_count(pool.clone()).await, expected_count, "action {action}");
    }
}
