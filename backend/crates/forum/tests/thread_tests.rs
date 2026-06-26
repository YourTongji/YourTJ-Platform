//! Integration tests for the forum domain — threads.

#[path = "helpers/mod.rs"]
mod helpers;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use helpers::{create_test_account, create_test_app, read_json};
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;

/// Seed a thread and return its id.
async fn seed_thread(pool: &PgPool, author_id: i64, title: &str, body: Option<&str>) -> i64 {
    let (id,): (i64,) = sqlx::query_as(
        "INSERT INTO forum.threads (board_id, author_id, title, body) \
         VALUES (1, $1, $2, $3) RETURNING id",
    )
    .bind(author_id)
    .bind(title)
    .bind(body)
    .fetch_one(pool)
    .await
    .expect("seed thread");
    id
}

/// ── create thread ────────────────────────────────────────────────────────

#[tokio::test]
async fn test_create_thread_requires_auth() {
    let (_, app) = create_test_app().await;

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/forum/threads")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({"boardId": "1", "title": "Hello"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_create_thread_returns_detail_dto() {
    let (pool, app) = create_test_app().await;
    let (_, token) = create_test_account(&pool, "alice@tongji.edu.cn", "alice").await;

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/forum/threads")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(
                    json!({"boardId": "1", "title": "My Thread", "body": "Content"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_json(resp).await;
    assert_eq!(body["title"], "My Thread");
    assert_eq!(body["body"], "Content");
    assert_eq!(body["authorHandle"], "alice");
    assert_eq!(body["boardId"], "1");
    assert_eq!(body["replyCount"], 0);
    assert_eq!(body["voteCount"], 0);
    assert!(body["id"].is_string());
    assert!(body["createdAt"].is_i64());
}

/// ── get thread ───────────────────────────────────────────────────────────

#[tokio::test]
async fn test_get_thread_not_found() {
    let (_, app) = create_test_app().await;

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v2/forum/threads/99999")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_thread_returns_detail() {
    let (pool, app) = create_test_app().await;
    let (author_id, _token) = create_test_account(&pool, "bob@tongji.edu.cn", "bob").await;
    let thread_id = seed_thread(&pool, author_id, "Bob's Thread", Some("Hello world")).await;

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("/api/v2/forum/threads/{thread_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_json(resp).await;
    assert_eq!(body["title"], "Bob's Thread");
    assert_eq!(body["body"], "Hello world");
    assert_eq!(body["authorHandle"], "bob");
    assert_eq!(body["id"], thread_id.to_string());
}

/// ── list threads ─────────────────────────────────────────────────────────

#[tokio::test]
async fn test_list_threads_empty() {
    let (_, app) = create_test_app().await;

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v2/forum/boards/1/threads")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_json(resp).await;
    assert!(body["items"].as_array().unwrap().is_empty());
    assert_eq!(body["hasMore"], false);
    assert!(body["nextCursor"].is_null());
}

#[tokio::test]
async fn test_list_threads_pagination() {
    let (pool, app) = create_test_app().await;
    let (author_id, _token) = create_test_account(&pool, "carol@tongji.edu.cn", "carol").await;

    // Seed 3 threads.
    for i in 1..=3 {
        seed_thread(&pool, author_id, &format!("Thread {i}"), None).await;
    }

    // Page 1: limit=2.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v2/forum/boards/1/threads?limit=2")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_json(resp).await;
    let items = body["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(body["hasMore"], true);
    let cursor = body["nextCursor"].as_str().unwrap().to_string();

    // Page 2: use cursor.
    let resp2 = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("/api/v2/forum/boards/1/threads?limit=2&cursor={cursor}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp2.status(), StatusCode::OK);
    let body2 = read_json(resp2).await;
    let items2 = body2["items"].as_array().unwrap();
    assert_eq!(items2.len(), 1);
    // With only 1 item remaining, has_more should be false.
    assert_eq!(body2["hasMore"], false);
}

/// ── list threads sorting ─────────────────────────────────────────────────

#[tokio::test]
async fn test_list_threads_default_sort_is_new() {
    let (pool, app) = create_test_app().await;
    let (author_id, _token) = create_test_account(&pool, "dave@tongji.edu.cn", "dave").await;

    seed_thread(&pool, author_id, "Old Thread", None).await;
    // Small delay to ensure different created_at timestamps.
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    seed_thread(&pool, author_id, "New Thread", None).await;

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v2/forum/boards/1/threads?limit=10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_json(resp).await;
    let items = body["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    // Newest first.
    assert_eq!(items[0]["title"], "New Thread");
    assert_eq!(items[1]["title"], "Old Thread");
}
