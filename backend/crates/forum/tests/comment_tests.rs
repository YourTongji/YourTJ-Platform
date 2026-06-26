//! Integration tests for the forum domain — comments and materialized paths.

#[path = "helpers/mod.rs"]
mod helpers;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use helpers::{create_test_account, create_test_app, read_json};
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;

/// Seed a thread and return its id.
async fn seed_thread(pool: &PgPool, author_id: i64) -> i64 {
    let (id,): (i64,) = sqlx::query_as(
        "INSERT INTO forum.threads (board_id, author_id, title) \
         VALUES (1, $1, 'Test Thread') RETURNING id",
    )
    .bind(author_id)
    .fetch_one(pool)
    .await
    .expect("seed thread");
    id
}

/// Seed a comment and return its id.
async fn seed_comment(
    pool: &PgPool,
    thread_id: i64,
    author_id: i64,
    body: &str,
    parent_id: Option<i64>,
) -> i64 {
    let path = if let Some(pid) = parent_id {
        let parent_path: String =
            sqlx::query_scalar("SELECT path FROM forum.comments WHERE id = $1")
                .bind(pid)
                .fetch_one(pool)
                .await
                .expect("parent path");
        let max_child: Option<String> = sqlx::query_scalar(
            "SELECT MAX(path) FROM forum.comments \
             WHERE thread_id = $1 AND parent_id = $2",
        )
        .bind(thread_id)
        .bind(pid)
        .fetch_one(pool)
        .await
        .expect("max child path");
        let idx = forum::repo::next_sibling_index(max_child.as_deref().unwrap_or(""), &parent_path);
        format!("{parent_path}.{idx:04x}")
    } else {
        let max_path: Option<String> = sqlx::query_scalar(
            "SELECT MAX(path) FROM forum.comments \
             WHERE thread_id = $1 AND parent_id IS NULL",
        )
        .bind(thread_id)
        .fetch_one(pool)
        .await
        .expect("max top path");
        let idx = forum::repo::next_sibling_index(max_path.as_deref().unwrap_or(""), "");
        format!("{idx:04x}")
    };

    let (id,): (i64,) = sqlx::query_as(
        "INSERT INTO forum.comments (thread_id, parent_id, path, author_id, body) \
         VALUES ($1, $2, $3, $4, $5) RETURNING id",
    )
    .bind(thread_id)
    .bind(parent_id)
    .bind(&path)
    .bind(author_id)
    .bind(body)
    .fetch_one(pool)
    .await
    .expect("seed comment");
    id
}

/// ── create comment ───────────────────────────────────────────────────────

#[tokio::test]
async fn test_create_comment_requires_auth() {
    let (_, app) = create_test_app().await;

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/forum/threads/1/comments")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({"body": "No auth"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_create_top_level_comment() {
    let (pool, app) = create_test_app().await;
    let (author_id, token) = create_test_account(&pool, "eve@tongji.edu.cn", "eve").await;
    let thread_id = seed_thread(&pool, author_id).await;

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/v2/forum/threads/{thread_id}/comments"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(json!({"body": "First comment"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_json(resp).await;
    assert_eq!(body["body"], "First comment");
    assert_eq!(body["authorHandle"], "eve");
    assert!(body["parentId"].is_null());
    // Top-level path should be like "0001".
    let path = body["path"].as_str().unwrap();
    assert!(path.len() == 4, "path should be 4 hex chars, got '{path}'");
    assert_eq!(body["voteCount"], 0);

    // Verify thread reply_count was bumped.
    let reply_count: i32 =
        sqlx::query_scalar("SELECT reply_count FROM forum.threads WHERE id = $1")
            .bind(thread_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(reply_count, 1);
}

#[tokio::test]
async fn test_create_nested_comment() {
    let (pool, app) = create_test_app().await;
    let (author_id, token) = create_test_account(&pool, "frank@tongji.edu.cn", "frank").await;
    let thread_id = seed_thread(&pool, author_id).await;

    // Create top-level comment.
    let c1_id = seed_comment(&pool, thread_id, author_id, "Hello", None).await;

    // Create nested reply.
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/v2/forum/threads/{thread_id}/comments"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(
                    json!({"parentId": c1_id.to_string(), "body": "Reply"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_json(resp).await;
    assert_eq!(body["body"], "Reply");
    assert_eq!(body["parentId"], c1_id.to_string());
    // Path should be like "0001.0001".
    let path = body["path"].as_str().unwrap();
    assert!(path.contains('.'), "nested path should contain a dot, got '{path}'");
}

/// ── list comments ────────────────────────────────────────────────────────

#[tokio::test]
async fn test_list_comments_empty() {
    let (pool, app) = create_test_app().await;
    let (author_id, _token) = create_test_account(&pool, "grace@tongji.edu.cn", "grace").await;
    let thread_id = seed_thread(&pool, author_id).await;

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v2/forum/threads/{thread_id}/comments"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_json(resp).await;
    assert!(body["items"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_list_comments_path_ordering() {
    let (pool, app) = create_test_app().await;
    let (author_id, _token) = create_test_account(&pool, "heidi@tongji.edu.cn", "heidi").await;
    let thread_id = seed_thread(&pool, author_id).await;

    // Create: C1 (top), C1.1 (child of C1), C2 (top).
    let c1 = seed_comment(&pool, thread_id, author_id, "C1", None).await;
    seed_comment(&pool, thread_id, author_id, "C1.1", Some(c1)).await;
    seed_comment(&pool, thread_id, author_id, "C2", None).await;

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v2/forum/threads/{thread_id}/comments?limit=10"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_json(resp).await;
    let items = body["items"].as_array().unwrap();
    assert_eq!(items.len(), 3, "expected 3 comments");
    assert_eq!(items[0]["body"], "C1");
    assert_eq!(items[1]["body"], "C1.1");
    assert_eq!(items[2]["body"], "C2");
}

#[tokio::test]
async fn test_list_comments_pagination() {
    let (pool, app) = create_test_app().await;
    let (author_id, _token) = create_test_account(&pool, "ivan@tongji.edu.cn", "ivan").await;
    let thread_id = seed_thread(&pool, author_id).await;

    // Seed 3 top-level comments.
    seed_comment(&pool, thread_id, author_id, "A", None).await;
    seed_comment(&pool, thread_id, author_id, "B", None).await;
    seed_comment(&pool, thread_id, author_id, "C", None).await;

    // Page with limit=2.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v2/forum/threads/{thread_id}/comments?limit=2"))
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

    let cursor = body["nextCursor"].as_str().unwrap();
    let resp2 = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v2/forum/threads/{thread_id}/comments?limit=2&cursor={cursor}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp2.status(), StatusCode::OK);
    let body2 = read_json(resp2).await;
    let items2 = body2["items"].as_array().unwrap();
    assert_eq!(items2.len(), 1);
}
