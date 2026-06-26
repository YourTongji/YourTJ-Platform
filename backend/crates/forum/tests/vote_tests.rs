//! Integration tests for the forum domain — votes.

#[path = "helpers/mod.rs"]
mod helpers;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use helpers::{create_test_account, create_test_app};
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;

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
    let (author_id, token) = create_test_account(&pool, "julia@tongji.edu.cn", "julia").await;
    let thread_id = seed_thread(&pool, author_id).await;

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(&format!("/api/v2/forum/posts/{thread_id}/vote"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(json!({"value": "up"}).to_string()))
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
}

#[tokio::test]
async fn test_vote_thread_down() {
    let (pool, app) = create_test_app().await;
    let (author_id, token) = create_test_account(&pool, "karl@tongji.edu.cn", "karl").await;
    let thread_id = seed_thread(&pool, author_id).await;

    // Up first, then down.
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(&format!("/api/v2/forum/posts/{thread_id}/vote"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(json!({"value": "up"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(&format!("/api/v2/forum/posts/{thread_id}/vote"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(json!({"value": "down"}).to_string()))
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
    assert_eq!(vote_count, 0);
}

/// ── vote on comment ──────────────────────────────────────────────────────

#[tokio::test]
async fn test_vote_comment_up() {
    let (pool, app) = create_test_app().await;
    let (author_id, token) = create_test_account(&pool, "liam@tongji.edu.cn", "liam").await;
    let thread_id = seed_thread(&pool, author_id).await;
    let comment_id = seed_comment(&pool, thread_id, author_id, "Nice post").await;

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(&format!("/api/v2/forum/posts/{comment_id}/vote"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(json!({"value": "up"}).to_string()))
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
                .body(Body::from(json!({"value": "up"}).to_string()))
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
                .body(Body::from(json!({"value": "up"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
