//! Integration tests for the forum domain — threads.

#[path = "helpers/mod.rs"]
mod helpers;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use helpers::{create_test_account, create_test_app, read_json};
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;

async fn drain_notification_outbox(pool: &PgPool) {
    let worker_id = uuid::Uuid::new_v4();
    loop {
        let events = platform::outbox::claim_events(pool, worker_id, 100)
            .await
            .expect("claim durable thread events");
        if events.is_empty() {
            break;
        }
        for event in events {
            match event.topic.as_str() {
                "notification" => {
                    forum::notification_delivery::deliver_event(pool, &event)
                        .await
                        .expect("deliver durable thread notification");
                }
                "achievement_award" => {
                    platform::achievements::deliver_automatic_award(pool, &event)
                        .await
                        .expect("deliver durable thread achievement");
                }
                topic => panic!("unexpected durable thread topic: {topic}"),
            }
        }
    }
}

async fn create_thread_request(
    app: &axum::Router,
    token: &str,
    body: serde_json::Value,
) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/forum/threads")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(body.to_string()))
                .expect("build thread request"),
        )
        .await
        .expect("thread response")
}

async fn update_thread_request(
    app: &axum::Router,
    thread_id: i64,
    token: &str,
    body: serde_json::Value,
) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(format!("/api/v2/forum/threads/{thread_id}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(body.to_string()))
                .expect("build thread update request"),
        )
        .await
        .expect("thread update response")
}

async fn get_thread_request(
    app: &axum::Router,
    thread_id: i64,
    token: Option<&str>,
) -> axum::response::Response {
    let mut request =
        Request::builder().method(Method::GET).uri(format!("/api/v2/forum/threads/{thread_id}"));
    if let Some(token) = token {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    app.clone()
        .oneshot(request.body(Body::empty()).expect("build thread detail request"))
        .await
        .expect("thread detail response")
}

async fn get_relationship_request(
    app: &axum::Router,
    token: &str,
    handle: &str,
) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v2/users/{handle}/relationship"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("build relationship request"),
        )
        .await
        .expect("relationship response")
}

async fn admin_thread_action_request(
    app: &axum::Router,
    thread_id: i64,
    action: &str,
    token: &str,
    body: serde_json::Value,
) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/v2/admin/forum/threads/{thread_id}/{action}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(body.to_string()))
                .expect("build admin thread request"),
        )
        .await
        .expect("admin thread response")
}

async fn board_thread_count(pool: &PgPool, board_id: i64) -> i32 {
    sqlx::query_scalar("SELECT thread_count FROM forum.boards WHERE id = $1")
        .bind(board_id)
        .fetch_one(pool)
        .await
        .expect("board thread count")
}

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
    let (account_id, token) = create_test_account(&pool, "alice@tongji.edu.cn", "alice").await;

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

    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = read_json(resp).await;
    assert_eq!(body["title"], "My Thread");
    assert_eq!(body["body"], "Content");
    assert_eq!(body["authorHandle"], "alice");
    assert_eq!(body["boardId"], "1");
    assert_eq!(body["replyCount"], 0);
    assert_eq!(body["voteCount"], 0);
    assert!(body["id"].is_string());
    assert!(body["createdAt"].is_i64());
    let activity_count: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(threads_created), 0)::bigint \
         FROM activity.daily_counts WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(activity_count, 1);
    assert_eq!(board_thread_count(&pool, 1).await, 1);
    let achievement_outbox_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM platform.outbox_events \
         WHERE recipient_account_id = $1 AND topic = 'achievement_award' \
           AND event_type = 'first-thread'",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("count first-thread outbox event");
    assert_eq!(achievement_outbox_count, 1);
}

#[tokio::test]
async fn thread_permissions_are_server_authoritative_in_detail_and_list_dtos() {
    let (pool, app) = create_test_app().await;
    let (author_id, author_token) =
        create_test_account(&pool, "permission-author@tongji.edu.cn", "permission-author").await;
    let (moderator_id, moderator_token) =
        create_test_account(&pool, "permission-mod@tongji.edu.cn", "permission-mod").await;
    let (admin_id, _) =
        create_test_account(&pool, "permission-admin@tongji.edu.cn", "permission-admin").await;
    sqlx::query(
        "UPDATE identity.accounts SET role = CASE \
         WHEN id = $1 THEN 'mod'::identity.account_role \
         WHEN id = $2 THEN 'admin'::identity.account_role ELSE role END",
    )
    .bind(moderator_id)
    .bind(admin_id)
    .execute(&pool)
    .await
    .expect("seed content permission roles");
    let thread_id = seed_thread(&pool, author_id, "Permission truth", Some("body")).await;
    let admin_thread_id = seed_thread(&pool, admin_id, "Protected author", None).await;

    let anonymous = read_json(get_thread_request(&app, thread_id, None).await).await;
    assert_eq!(anonymous["contentVersion"], 1);
    assert_eq!(anonymous["canEdit"], false);
    assert_eq!(anonymous["canDelete"], false);
    assert_eq!(anonymous["canModerate"], false);

    let author = read_json(get_thread_request(&app, thread_id, Some(&author_token)).await).await;
    assert_eq!(author["canEdit"], true);
    assert_eq!(author["canDelete"], true);
    assert_eq!(author["canModerate"], false);

    let moderator =
        read_json(get_thread_request(&app, thread_id, Some(&moderator_token)).await).await;
    assert_eq!(moderator["canEdit"], false);
    assert_eq!(moderator["canDelete"], false);
    assert_eq!(moderator["canModerate"], true);
    let protected =
        read_json(get_thread_request(&app, admin_thread_id, Some(&moderator_token)).await).await;
    assert_eq!(protected["canModerate"], false);

    let list_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v2/forum/threads?sort=new")
                .header(header::AUTHORIZATION, format!("Bearer {moderator_token}"))
                .body(Body::empty())
                .expect("build thread list request"),
        )
        .await
        .expect("thread list response");
    assert_eq!(list_response.status(), StatusCode::OK);
    let list = read_json(list_response).await;
    let thread_id_string = thread_id.to_string();
    let user_thread = list["items"]
        .as_array()
        .expect("thread list items")
        .iter()
        .find(|item| item["id"].as_str() == Some(thread_id_string.as_str()))
        .expect("user-authored thread in list");
    assert_eq!(user_thread["contentVersion"], 1);
    assert_eq!(user_thread["canModerate"], true);
}

#[tokio::test]
async fn markdown_thread_format_is_explicit_validated_and_revisioned() {
    let (pool, app) = create_test_app().await;
    let (_, token) =
        create_test_account(&pool, "markdown-thread@tongji.edu.cn", "markdown-thread").await;

    let created = create_thread_request(
        &app,
        &token,
        json!({
            "boardId": "1",
            "title": "Markdown source",
            "body": "# Hello\n\n**world** [课程](/courses)",
            "contentFormat": "markdown_v1"
        }),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let created = read_json(created).await;
    assert_eq!(created["contentFormat"], "markdown_v1");
    let thread_id = created["id"].as_str().expect("thread id").parse::<i64>().unwrap();
    let stored_format: String =
        sqlx::query_scalar("SELECT content_format FROM forum.threads WHERE id = $1")
            .bind(thread_id)
            .fetch_one(&pool)
            .await
            .expect("stored thread format");
    assert_eq!(stored_format, "markdown_v1");

    sqlx::query(
        "UPDATE forum.threads SET created_at = now() - interval '10 minutes' WHERE id = $1",
    )
    .bind(thread_id)
    .execute(&pool)
    .await
    .expect("age thread beyond revision grace");
    let updated = update_thread_request(
        &app,
        thread_id,
        &token,
        json!({ "body": "plain replacement", "contentFormat": "plain_v1" }),
    )
    .await;
    assert_eq!(updated.status(), StatusCode::OK);
    assert_eq!(read_json(updated).await["contentFormat"], "plain_v1");
    let revision_format: String = sqlx::query_scalar(
        "SELECT old_content_format FROM forum.post_revisions \
         WHERE post_type = 'thread' AND post_id = $1 ORDER BY seq DESC LIMIT 1",
    )
    .bind(thread_id)
    .fetch_one(&pool)
    .await
    .expect("thread revision format");
    assert_eq!(revision_format, "markdown_v1");

    for unsafe_body in [
        "<script>alert(1)</script>",
        "![remote](https://example.com/image.png)",
        "[unsafe](javascript:alert(1))",
    ] {
        let rejected = create_thread_request(
            &app,
            &token,
            json!({
                "boardId": "1",
                "title": "Rejected Markdown",
                "body": unsafe_body,
                "contentFormat": "markdown_v1"
            }),
        )
        .await;
        assert_eq!(rejected.status(), StatusCode::BAD_REQUEST);
    }
}

#[tokio::test]
async fn board_posting_gates_deny_ordinary_accounts_without_side_effects() {
    let (pool, app) = create_test_app().await;
    let (account_id, token) =
        create_test_account(&pool, "board-gates@tongji.edu.cn", "board-gates").await;
    sqlx::query("UPDATE forum.boards SET is_locked = true, min_trust_to_post = 2 WHERE id = 1")
        .execute(&pool)
        .await
        .expect("lock trust-gated board");

    let locked =
        create_thread_request(&app, &token, json!({"boardId": "1", "title": "Must not exist"}))
            .await;
    assert_eq!(locked.status(), StatusCode::FORBIDDEN);

    sqlx::query("UPDATE forum.boards SET is_locked = false WHERE id = 1")
        .execute(&pool)
        .await
        .expect("unlock board");
    sqlx::query("UPDATE identity.accounts SET trust_level = 1 WHERE id = $1")
        .bind(account_id)
        .execute(&pool)
        .await
        .expect("set insufficient trust");
    let low_trust = create_thread_request(
        &app,
        &token,
        json!({"boardId": "1", "title": "Still must not exist"}),
    )
    .await;
    assert_eq!(low_trust.status(), StatusCode::FORBIDDEN);

    let thread_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM forum.threads")
        .fetch_one(&pool)
        .await
        .expect("thread count");
    assert_eq!(thread_count, 0);
    assert_eq!(board_thread_count(&pool, 1).await, 0);
}

#[tokio::test]
async fn content_moderator_bypasses_only_board_posting_gates() {
    let (pool, app) = create_test_app().await;
    let (moderator_id, token) =
        create_test_account(&pool, "board-mod@tongji.edu.cn", "board-mod").await;
    sqlx::query("UPDATE identity.accounts SET role = 'mod', trust_level = 0 WHERE id = $1")
        .bind(moderator_id)
        .execute(&pool)
        .await
        .expect("promote content moderator");
    sqlx::query("UPDATE forum.boards SET is_locked = true, min_trust_to_post = 3 WHERE id = 1")
        .execute(&pool)
        .await
        .expect("lock trust-gated board");

    let response =
        create_thread_request(&app, &token, json!({"boardId": "1", "title": "Staff notice"})).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(board_thread_count(&pool, 1).await, 1);
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
                .uri(format!("/api/v2/forum/threads/{thread_id}"))
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
                .uri(format!("/api/v2/forum/boards/1/threads?limit=2&cursor={cursor}"))
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

#[tokio::test]
async fn archived_threads_are_excluded_from_every_feed() {
    let (pool, app) = create_test_app().await;
    let (author_id, token) =
        create_test_account(&pool, "archived-feed@tongji.edu.cn", "archived-feed").await;
    let visible_id = seed_thread(&pool, author_id, "Visible", None).await;
    let archived_id = seed_thread(&pool, author_id, "Archived", None).await;
    sqlx::query("UPDATE forum.threads SET archived_at = now() WHERE id = $1")
        .bind(archived_id)
        .execute(&pool)
        .await
        .expect("archive thread");
    sqlx::query(
        "INSERT INTO forum.subscriptions (account_id, target_type, target_id, level) \
         VALUES ($1, 'thread', $2, 'tracking'), ($1, 'thread', $3, 'tracking')",
    )
    .bind(author_id)
    .bind(visible_id)
    .bind(archived_id)
    .execute(&pool)
    .await
    .expect("seed subscriptions");
    sqlx::query("UPDATE forum.threads SET reply_count = 1 WHERE id = ANY($1)")
        .bind(vec![visible_id, archived_id])
        .execute(&pool)
        .await
        .expect("seed unread replies");
    sqlx::query(
        "INSERT INTO forum.comments (thread_id, author_id, body, path) \
         VALUES ($1, $3, 'visible unread reply', '0001'), \
                ($2, $3, 'archived unread reply', '0001')",
    )
    .bind(visible_id)
    .bind(archived_id)
    .bind(author_id)
    .execute(&pool)
    .await
    .expect("seed unread comments");
    sqlx::query(
        "INSERT INTO forum.thread_reads (account_id, thread_id, updated_at) \
         VALUES ($1, $2, now() - interval '1 day'), \
                ($1, $3, now() - interval '1 day')",
    )
    .bind(author_id)
    .bind(visible_id)
    .bind(archived_id)
    .execute(&pool)
    .await
    .expect("seed unread positions");

    for uri in [
        "/api/v2/forum/boards/1/threads",
        "/api/v2/forum/threads",
        "/api/v2/forum/threads?sort=subscriptions",
        "/api/v2/forum/threads?sort=unread",
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(uri)
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::empty())
                    .expect("build feed request"),
            )
            .await
            .expect("feed response");
        assert_eq!(response.status(), StatusCode::OK, "feed {uri}");
        let body = read_json(response).await;
        let ids: Vec<&str> = body["items"]
            .as_array()
            .expect("feed items")
            .iter()
            .filter_map(|item| item["id"].as_str())
            .collect();
        assert!(ids.contains(&visible_id.to_string().as_str()), "feed {uri}");
        assert!(!ids.contains(&archived_id.to_string().as_str()), "feed {uri}");
    }
}

#[tokio::test]
async fn invalid_thread_and_poll_inputs_leave_no_thread() {
    let (pool, app) = create_test_app().await;
    let (_, token) =
        create_test_account(&pool, "thread-validation@tongji.edu.cn", "thread-validation").await;
    let cases = [
        json!({ "boardId": "1", "title": "   " }),
        json!({ "boardId": "1", "title": "Valid", "tags": ["a", "b", "c", "d"] }),
        json!({ "boardId": "1", "title": "Unknown tag", "tags": ["missing-tag"] }),
        json!({
            "boardId": "1",
            "title": "Invalid poll",
            "poll": { "question": "Question", "options": ["only one"] }
        }),
        json!({
            "boardId": "1",
            "title": "Duplicate poll",
            "poll": { "question": "Question", "options": ["Same", " same "] }
        }),
    ];
    for body in cases {
        let response = create_thread_request(&app, &token, body).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    let thread_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM forum.threads")
        .fetch_one(&pool)
        .await
        .expect("thread count");
    assert_eq!(thread_count, 0);
}

#[tokio::test]
async fn queued_thread_is_hidden_without_activity_credit() {
    let (pool, app) = create_test_app().await;
    let (author_id, token) =
        create_test_account(&pool, "queued-thread@tongji.edu.cn", "queued-thread").await;
    let (mentioned_id, _) = create_test_account(
        &pool,
        "queued-thread-mentioned@tongji.edu.cn",
        "queued-thread-mentioned",
    )
    .await;
    let marker = "queued-thread-marker-51af";
    sqlx::query("INSERT INTO forum.watched_words (word, action) VALUES ($1, 'queue')")
        .bind(marker)
        .execute(&pool)
        .await
        .expect("insert watched word");
    forum::watched_words::reload_watched_words(&pool).await.expect("reload watched words");

    let response = create_thread_request(
        &app,
        &token,
        json!({
            "boardId": "1",
            "title": "Queued",
            "body": format!("{marker} @queued-thread-mentioned"),
            "poll": {
                "question": "Private until approved",
                "multiSelect": false,
                "options": ["Yes", "No"]
            }
        }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = read_json(response).await;
    let thread_id: i64 = body["id"].as_str().expect("thread id").parse().unwrap();
    let poll_id = body["poll"]["id"].as_str().expect("queued poll id");
    assert!(body["hiddenAt"].is_i64());
    let poll_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v2/forum/polls/{poll_id}/results"))
                .body(Body::empty())
                .expect("build queued poll request"),
        )
        .await
        .expect("queued poll response");
    assert_eq!(poll_response.status(), StatusCode::NOT_FOUND);
    let activity_count: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(threads_created), 0)::bigint \
         FROM activity.daily_counts WHERE account_id = $1",
    )
    .bind(author_id)
    .fetch_one(&pool)
    .await
    .expect("thread activity");
    assert_eq!(activity_count, 0);
    assert_eq!(board_thread_count(&pool, 1).await, 0);
    let hidden: bool =
        sqlx::query_scalar("SELECT hidden_at IS NOT NULL FROM forum.threads WHERE id = $1")
            .bind(thread_id)
            .fetch_one(&pool)
            .await
            .expect("thread hidden state");
    assert!(hidden);
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    let notification_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM forum.notifications WHERE account_id = $1")
            .bind(mentioned_id)
            .fetch_one(&pool)
            .await
            .expect("queued mention notifications");
    assert_eq!(notification_count, 0);
    let public_stat_count: i32 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(threads_created), 0)::int FROM forum.user_stats WHERE account_id = $1",
    )
    .bind(author_id)
    .fetch_one(&pool)
    .await
    .expect("queued thread stats");
    assert_eq!(public_stat_count, 0);
    let subscription_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM forum.subscriptions WHERE account_id = $1 AND target_id = $2",
    )
    .bind(author_id)
    .bind(thread_id)
    .fetch_one(&pool)
    .await
    .expect("queued thread subscription");
    assert_eq!(subscription_count, 0);

    sqlx::query("DELETE FROM forum.watched_words WHERE word = $1")
        .bind(marker)
        .execute(&pool)
        .await
        .expect("remove watched word");
    forum::watched_words::reload_watched_words(&pool).await.expect("reload watched words");
}

#[tokio::test]
async fn thread_mentions_apply_recipient_policy_without_rejecting_literal_text() {
    let (pool, app) = create_test_app().await;
    let (actor_id, actor_token) =
        create_test_account(&pool, "mention-actor@tongji.edu.cn", "mention-actor").await;
    let (everyone_id, _) =
        create_test_account(&pool, "mention-everyone@tongji.edu.cn", "mention-everyone").await;
    let (following_id, _) =
        create_test_account(&pool, "mention-following@tongji.edu.cn", "mention-following").await;
    let (not_following_id, _) =
        create_test_account(&pool, "mention-not-following@tongji.edu.cn", "mention-not-following")
            .await;
    let (nobody_id, _) =
        create_test_account(&pool, "mention-nobody@tongji.edu.cn", "mention-nobody").await;
    let (blocked_id, _) =
        create_test_account(&pool, "mention-blocked@tongji.edu.cn", "mention-blocked").await;
    let (suspended_id, _) =
        create_test_account(&pool, "mention-suspended@tongji.edu.cn", "mention-suspended").await;
    let (inactive_id, _) =
        create_test_account(&pool, "mention-inactive@tongji.edu.cn", "mention-inactive").await;

    for (account_id, policy) in [
        (everyone_id, "everyone"),
        (following_id, "following"),
        (not_following_id, "following"),
        (nobody_id, "nobody"),
        (blocked_id, "everyone"),
        (suspended_id, "everyone"),
        (inactive_id, "everyone"),
    ] {
        sqlx::query(
            "INSERT INTO identity.profile_privacy (account_id, mention_policy) VALUES ($1, $2) \
             ON CONFLICT (account_id) DO UPDATE SET mention_policy = EXCLUDED.mention_policy",
        )
        .bind(account_id)
        .bind(policy)
        .execute(&pool)
        .await
        .expect("set mention policy");
    }
    sqlx::query("INSERT INTO forum.user_follows (follower_id, followed_id) VALUES ($1, $2)")
        .bind(following_id)
        .bind(actor_id)
        .execute(&pool)
        .await
        .expect("seed recipient follow");
    sqlx::query("INSERT INTO forum.user_ignores (account_id, ignored_account_id) VALUES ($1, $2)")
        .bind(blocked_id)
        .bind(actor_id)
        .execute(&pool)
        .await
        .expect("seed mention block");
    sqlx::query(
        "INSERT INTO identity.sanctions (account_id, kind, reason, starts_at) \
         VALUES ($1, 'suspend', 'mention privacy test', now())",
    )
    .bind(suspended_id)
    .execute(&pool)
    .await
    .expect("suspend mention target");
    sqlx::query(
        "UPDATE identity.accounts SET status = 'deleted', \
             deletion_requested_at = now() - interval '31 days', \
             deletion_recover_until = now() - interval '1 day', deleted_at = now() \
         WHERE id = $1",
    )
    .bind(inactive_id)
    .execute(&pool)
    .await
    .expect("close mention target");

    let literal_body = "@mention-everyone @mention-following @mention-not-following \
                        @mention-nobody @mention-blocked @mention-suspended \
                        @mention-inactive @MENTION-ACTOR @unknown-handle";
    let response = create_thread_request(
        &app,
        &actor_token,
        json!({
            "boardId": "1",
            "title": "Recipient-controlled mentions",
            "body": literal_body,
            "contentFormat": "plain_v1"
        }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let created = read_json(response).await;
    assert_eq!(created["body"], literal_body);

    let mention_candidate_ids = vec![
        everyone_id,
        following_id,
        not_following_id,
        nobody_id,
        blocked_id,
        suspended_id,
        inactive_id,
        actor_id,
    ];

    drain_notification_outbox(&pool).await;
    let recipients: Vec<i64> = sqlx::query_scalar(
        "SELECT account_id FROM forum.notifications \
         WHERE type = 'mention' AND account_id = ANY($1) ORDER BY account_id",
    )
    .bind(&mention_candidate_ids)
    .fetch_all(&pool)
    .await
    .expect("mention recipients");
    let mut expected_recipients = vec![everyone_id, following_id];
    expected_recipients.sort_unstable();
    assert_eq!(recipients, expected_recipients);

    for (handle, expected) in [
        ("mention-everyone", true),
        ("mention-following", true),
        ("mention-not-following", false),
        ("mention-nobody", false),
        ("mention-blocked", false),
        ("mention-actor", false),
    ] {
        let response = get_relationship_request(&app, &actor_token, handle).await;
        assert_eq!(response.status(), StatusCode::OK, "relationship for {handle}");
        let relationship = read_json(response).await;
        assert_eq!(relationship["canMention"], expected, "relationship for {handle}");
    }
    assert_eq!(
        get_relationship_request(&app, &actor_token, "mention-suspended").await.status(),
        StatusCode::NOT_FOUND
    );
    assert!(!recipients.contains(&nobody_id));
    assert!(!recipients.contains(&blocked_id));
    assert!(!recipients.contains(&suspended_id));
    assert!(!recipients.contains(&inactive_id));
    assert!(!recipients.contains(&actor_id));
    assert!(!recipients.contains(&not_following_id));
}

#[tokio::test]
async fn thread_edits_share_create_validation_and_persist_canonical_censoring() {
    let (pool, app) = create_test_app().await;
    let (author_id, token) =
        create_test_account(&pool, "thread-edit-policy@tongji.edu.cn", "thread-edit-policy").await;
    let thread_id = seed_thread(&pool, author_id, "Original title", Some("Original body")).await;
    sqlx::query(
        "UPDATE forum.threads SET created_at = now() - interval '10 minutes' WHERE id = $1",
    )
    .bind(thread_id)
    .execute(&pool)
    .await
    .expect("age thread");
    let blocked_marker = "thread-edit-blocked-9d4f";
    let censored_marker = "thread-edit-censored-63b2";
    sqlx::query(
        "INSERT INTO forum.watched_words (word, action) VALUES ($1, 'block'), ($2, 'censor')",
    )
    .bind(blocked_marker)
    .bind(censored_marker)
    .execute(&pool)
    .await
    .expect("insert edit policy words");
    forum::watched_words::reload_watched_words(&pool).await.expect("reload watched words");

    let oversized =
        update_thread_request(&app, thread_id, &token, json!({ "body": "x".repeat(50_001) })).await;
    assert_eq!(oversized.status(), StatusCode::BAD_REQUEST);
    let blocked = update_thread_request(
        &app,
        thread_id,
        &token,
        json!({ "title": format!("Blocked {blocked_marker}") }),
    )
    .await;
    assert_eq!(blocked.status(), StatusCode::BAD_REQUEST);

    let censored = update_thread_request(
        &app,
        thread_id,
        &token,
        json!({
            "title": format!("Title {censored_marker}"),
            "body": format!("Body {censored_marker}")
        }),
    )
    .await;
    assert_eq!(censored.status(), StatusCode::OK);
    let response_body = read_json(censored).await;
    let stored: (String, Option<String>) =
        sqlx::query_as("SELECT title, body FROM forum.threads WHERE id = $1")
            .bind(thread_id)
            .fetch_one(&pool)
            .await
            .expect("stored canonical thread");
    assert_eq!(response_body["title"].as_str(), Some(stored.0.as_str()));
    assert_eq!(response_body["body"].as_str(), stored.1.as_deref());
    assert!(!stored.0.contains(censored_marker));
    assert!(!stored.1.expect("thread body").contains(censored_marker));
    let revisions: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM forum.post_revisions WHERE post_type = 'thread' AND post_id = $1",
    )
    .bind(thread_id)
    .fetch_one(&pool)
    .await
    .expect("thread revision count");
    assert_eq!(revisions, 1);

    sqlx::query("DELETE FROM forum.watched_words WHERE word = ANY($1)")
        .bind(vec![blocked_marker.to_owned(), censored_marker.to_owned()])
        .execute(&pool)
        .await
        .expect("remove edit policy words");
    forum::watched_words::reload_watched_words(&pool).await.expect("reload watched words");
}

#[tokio::test]
async fn queued_thread_edit_removes_public_counter_and_activity() {
    let (pool, app) = create_test_app().await;
    let (author_id, token) =
        create_test_account(&pool, "thread-edit-queue@tongji.edu.cn", "thread-edit-queue").await;
    let created = create_thread_request(
        &app,
        &token,
        json!({ "boardId": "1", "title": "Initially visible", "body": "clean" }),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let thread_id: i64 = read_json(created).await["id"]
        .as_str()
        .expect("thread id")
        .parse()
        .expect("numeric thread id");
    assert_eq!(board_thread_count(&pool, 1).await, 1);
    let marker = "thread-edit-queue-marker-65d1";
    sqlx::query("INSERT INTO forum.watched_words (word, action) VALUES ($1, 'queue')")
        .bind(marker)
        .execute(&pool)
        .await
        .expect("insert queue word");
    forum::watched_words::reload_watched_words(&pool).await.expect("reload watched words");

    let response = update_thread_request(&app, thread_id, &token, json!({ "body": marker })).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(read_json(response).await["hiddenAt"].is_i64());
    assert_eq!(board_thread_count(&pool, 1).await, 0);
    let activity_count: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(threads_created), 0)::bigint FROM activity.daily_counts WHERE account_id = $1",
    )
    .bind(author_id)
    .fetch_one(&pool)
    .await
    .expect("thread activity after queue edit");
    assert_eq!(activity_count, 0);

    sqlx::query("DELETE FROM forum.watched_words WHERE word = $1")
        .bind(marker)
        .execute(&pool)
        .await
        .expect("remove queue word");
    forum::watched_words::reload_watched_words(&pool).await.expect("reload watched words");
}

#[tokio::test]
async fn concurrent_thread_edits_reject_stale_writes_without_revision_gaps() {
    let (pool, app) = create_test_app().await;
    let (author_id, token) = create_test_account(
        &pool,
        "thread-revision-concurrency@tongji.edu.cn",
        "thread-revision-concurrency",
    )
    .await;
    let thread_id = seed_thread(&pool, author_id, "Concurrent", Some("original")).await;
    sqlx::query(
        "UPDATE forum.threads SET created_at = now() - interval '10 minutes' WHERE id = $1",
    )
    .bind(thread_id)
    .execute(&pool)
    .await
    .expect("age concurrent thread");

    let first = update_thread_request(
        &app,
        thread_id,
        &token,
        json!({ "body": "first edit", "expectedVersion": 1 }),
    );
    let second = update_thread_request(
        &app,
        thread_id,
        &token,
        json!({ "body": "second edit", "expectedVersion": 1 }),
    );
    let (first_response, second_response) = tokio::join!(first, second);
    let (success_response, conflict_response) = if first_response.status() == StatusCode::OK {
        (first_response, second_response)
    } else {
        (second_response, first_response)
    };
    assert_eq!(success_response.status(), StatusCode::OK);
    assert_eq!(conflict_response.status(), StatusCode::CONFLICT);
    let success = read_json(success_response).await;
    let conflict = read_json(conflict_response).await;
    assert_eq!(success["contentVersion"], 2);
    assert_eq!(conflict["error"]["code"], "VERSION_CONFLICT");
    assert_eq!(conflict["error"]["details"]["currentVersion"], 2);

    let revisions: Vec<(i32, String)> = sqlx::query_as(
        "SELECT seq, old_body FROM forum.post_revisions \
         WHERE post_type = 'thread' AND post_id = $1 ORDER BY seq",
    )
    .bind(thread_id)
    .fetch_all(&pool)
    .await
    .expect("concurrent revisions");
    assert_eq!(revisions.len(), 1);
    assert_eq!(revisions[0], (1, "original".into()));

    let legacy_retry =
        update_thread_request(&app, thread_id, &token, json!({ "body": "legacy retry" })).await;
    assert_eq!(legacy_retry.status(), StatusCode::CONFLICT);
    assert_eq!(read_json(legacy_retry).await["error"]["details"]["currentVersion"], 2);

    let resolved = update_thread_request(
        &app,
        thread_id,
        &token,
        json!({ "body": "resolved edit", "expectedVersion": 2 }),
    )
    .await;
    assert_eq!(resolved.status(), StatusCode::OK);
    assert_eq!(read_json(resolved).await["contentVersion"], 3);
    let canonical: (String, i64) =
        sqlx::query_as("SELECT body, content_version FROM forum.threads WHERE id = $1")
            .bind(thread_id)
            .fetch_one(&pool)
            .await
            .expect("canonical thread after conflict recovery");
    assert_eq!(canonical, ("resolved edit".into(), 3));
    let revision_sequences: Vec<i32> = sqlx::query_scalar(
        "SELECT seq FROM forum.post_revisions \
         WHERE post_type = 'thread' AND post_id = $1 ORDER BY seq",
    )
    .bind(thread_id)
    .fetch_all(&pool)
    .await
    .expect("revision sequence after conflict recovery");
    assert_eq!(revision_sequences, vec![1, 2]);

    let legacy_writer_version: i64 = sqlx::query_scalar(
        "UPDATE forum.threads SET body = 'legacy writer edit' WHERE id = $1 \
         RETURNING content_version",
    )
    .bind(thread_id)
    .fetch_one(&pool)
    .await
    .expect("legacy thread writer version bump");
    assert_eq!(legacy_writer_version, 4);
    let stale_after_legacy = update_thread_request(
        &app,
        thread_id,
        &token,
        json!({ "body": "must conflict", "expectedVersion": 3 }),
    )
    .await;
    assert_eq!(stale_after_legacy.status(), StatusCode::CONFLICT);
    assert_eq!(read_json(stale_after_legacy).await["error"]["details"]["currentVersion"], 4);
}

#[tokio::test]
async fn failed_thread_update_rolls_back_its_revision() {
    let (pool, app) = create_test_app().await;
    let (author_id, token) = create_test_account(
        &pool,
        "thread-revision-rollback@tongji.edu.cn",
        "thread-revision-rollback",
    )
    .await;
    let thread_id = seed_thread(&pool, author_id, "Rollback", Some("original")).await;
    sqlx::query(
        "UPDATE forum.threads SET created_at = now() - interval '10 minutes' WHERE id = $1",
    )
    .bind(thread_id)
    .execute(&pool)
    .await
    .expect("age rollback thread");
    sqlx::query(
        "ALTER TABLE forum.threads DROP CONSTRAINT IF EXISTS thread_revision_rollback_test",
    )
    .execute(&pool)
    .await
    .expect("drop stale rollback constraint");
    sqlx::query(
        "ALTER TABLE forum.threads ADD CONSTRAINT thread_revision_rollback_test \
         CHECK (body <> 'revision-rollback-marker')",
    )
    .execute(&pool)
    .await
    .expect("add rollback constraint");

    let response = update_thread_request(
        &app,
        thread_id,
        &token,
        json!({ "body": "revision-rollback-marker" }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    sqlx::query("ALTER TABLE forum.threads DROP CONSTRAINT thread_revision_rollback_test")
        .execute(&pool)
        .await
        .expect("remove rollback constraint");

    let stored_body: Option<String> =
        sqlx::query_scalar("SELECT body FROM forum.threads WHERE id = $1")
            .bind(thread_id)
            .fetch_one(&pool)
            .await
            .expect("body after rollback");
    assert_eq!(stored_body.as_deref(), Some("original"));
    let revision_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM forum.post_revisions WHERE post_type = 'thread' AND post_id = $1",
    )
    .bind(thread_id)
    .fetch_one(&pool)
    .await
    .expect("revision count after rollback");
    assert_eq!(revision_count, 0);
}

#[tokio::test]
async fn thread_tags_poll_and_activity_roll_back_together() {
    let (pool, app) = create_test_app().await;
    let (author_id, token) =
        create_test_account(&pool, "thread-create-atomic@tongji.edu.cn", "thread-create-atomic")
            .await;
    sqlx::query("DELETE FROM forum.tags WHERE slug = 'atomic-tag'")
        .execute(&pool)
        .await
        .expect("remove stale atomic tag");
    let tag_id: i64 = sqlx::query_scalar(
        "INSERT INTO forum.tags (slug, name) VALUES ('atomic-tag', 'Atomic') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("create atomic tag");
    sqlx::query(
        "ALTER TABLE forum.poll_options DROP CONSTRAINT IF EXISTS poll_option_atomicity_test",
    )
    .execute(&pool)
    .await
    .expect("drop stale poll constraint");
    sqlx::query(
        "ALTER TABLE forum.poll_options ADD CONSTRAINT poll_option_atomicity_test \
         CHECK (label <> 'poll-rollback-marker')",
    )
    .execute(&pool)
    .await
    .expect("add poll rollback constraint");

    let response = create_thread_request(
        &app,
        &token,
        json!({
            "boardId": "1",
            "title": "Atomic thread",
            "tags": ["atomic-tag"],
            "poll": {
                "question": "Atomic poll",
                "options": ["valid option", "poll-rollback-marker"]
            }
        }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    sqlx::query("ALTER TABLE forum.poll_options DROP CONSTRAINT poll_option_atomicity_test")
        .execute(&pool)
        .await
        .expect("remove poll rollback constraint");

    let thread_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM forum.threads WHERE title = 'Atomic thread'")
            .fetch_one(&pool)
            .await
            .expect("rolled-back thread count");
    assert_eq!(thread_count, 0);
    let thread_tag_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM forum.thread_tags WHERE tag_id = $1")
            .bind(tag_id)
            .fetch_one(&pool)
            .await
            .expect("rolled-back thread tag count");
    assert_eq!(thread_tag_count, 0);
    let poll_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM forum.polls")
        .fetch_one(&pool)
        .await
        .expect("rolled-back poll count");
    assert_eq!(poll_count, 0);
    let tag_count: i32 = sqlx::query_scalar("SELECT thread_count FROM forum.tags WHERE id = $1")
        .bind(tag_id)
        .fetch_one(&pool)
        .await
        .expect("tag count after rollback");
    assert_eq!(tag_count, 0);
    let activity_count: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(threads_created), 0)::bigint FROM activity.daily_counts WHERE account_id = $1",
    )
    .bind(author_id)
    .fetch_one(&pool)
    .await
    .expect("activity after rollback");
    assert_eq!(activity_count, 0);
}

#[tokio::test]
async fn thread_edit_rejects_hidden_deleted_and_archived_targets() {
    let (pool, app) = create_test_app().await;
    let (author_id, token) =
        create_test_account(&pool, "thread-edit-state@tongji.edu.cn", "thread-edit-state").await;
    let thread_id = seed_thread(&pool, author_id, "State guarded", Some("original")).await;

    sqlx::query("UPDATE forum.threads SET hidden_at = now() WHERE id = $1")
        .bind(thread_id)
        .execute(&pool)
        .await
        .expect("hide thread");
    let hidden =
        update_thread_request(&app, thread_id, &token, json!({ "body": "hidden edit" })).await;
    assert_eq!(hidden.status(), StatusCode::NOT_FOUND);

    sqlx::query("UPDATE forum.threads SET hidden_at = NULL, archived_at = now() WHERE id = $1")
        .bind(thread_id)
        .execute(&pool)
        .await
        .expect("archive thread");
    let archived =
        update_thread_request(&app, thread_id, &token, json!({ "body": "archived edit" })).await;
    assert_eq!(archived.status(), StatusCode::CONFLICT);

    sqlx::query("UPDATE forum.threads SET archived_at = NULL, deleted_at = now() WHERE id = $1")
        .bind(thread_id)
        .execute(&pool)
        .await
        .expect("delete thread");
    let deleted =
        update_thread_request(&app, thread_id, &token, json!({ "body": "deleted edit" })).await;
    assert_eq!(deleted.status(), StatusCode::NOT_FOUND);

    let stored_body: Option<String> =
        sqlx::query_scalar("SELECT body FROM forum.threads WHERE id = $1")
            .bind(thread_id)
            .fetch_one(&pool)
            .await
            .expect("state-guarded body");
    assert_eq!(stored_body.as_deref(), Some("original"));
}

#[tokio::test]
async fn board_thread_count_tracks_visible_thread_state_transitions() {
    let (pool, app) = create_test_app().await;
    let (author_id, author_token) =
        create_test_account(&pool, "board-count-author@tongji.edu.cn", "board-count-author").await;
    let (moderator_id, moderator_token) =
        create_test_account(&pool, "board-count-mod@tongji.edu.cn", "board-count-mod").await;
    sqlx::query("UPDATE identity.accounts SET role = 'mod' WHERE id = $1")
        .bind(moderator_id)
        .execute(&pool)
        .await
        .expect("promote moderator");
    let second_board_id: i64 = sqlx::query_scalar(
        "INSERT INTO forum.boards (slug, name) VALUES ('secondary', 'Secondary') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("create second board");

    let create_response = create_thread_request(
        &app,
        &author_token,
        json!({ "boardId": "1", "title": "Counter target", "body": "visible" }),
    )
    .await;
    assert_eq!(create_response.status(), StatusCode::CREATED);
    let thread_id: i64 = read_json(create_response).await["id"]
        .as_str()
        .expect("thread id")
        .parse()
        .expect("numeric thread id");
    assert_eq!(board_thread_count(&pool, 1).await, 1);
    assert_eq!(board_thread_count(&pool, second_board_id).await, 0);

    let transitions = [
        (
            "move",
            json!({ "reason": "move to the correct board", "boardId": second_board_id.to_string() }),
            (0, 1),
        ),
        ("delete", json!({ "reason": "remove confirmed content" }), (0, 0)),
        ("restore", json!({ "reason": "restore after successful appeal" }), (0, 1)),
        ("hide", json!({ "reason": "temporarily hide for review" }), (0, 0)),
        ("unhide", json!({ "reason": "review completed successfully" }), (0, 1)),
        ("archive", json!({ "reason": "archive inactive discussion" }), (0, 0)),
        ("unarchive", json!({ "reason": "discussion is relevant again" }), (0, 1)),
    ];
    for (action, body, expected_counts) in transitions {
        let response =
            admin_thread_action_request(&app, thread_id, action, &moderator_token, body).await;
        assert_eq!(response.status(), StatusCode::OK, "failed transition {action}");
        assert_eq!(board_thread_count(&pool, 1).await, expected_counts.0);
        assert_eq!(board_thread_count(&pool, second_board_id).await, expected_counts.1);
    }

    let author_delete_response = app
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri(format!("/api/v2/forum/threads/{thread_id}"))
                .header(header::AUTHORIZATION, format!("Bearer {author_token}"))
                .body(Body::empty())
                .expect("build author delete request"),
        )
        .await
        .expect("author delete response");
    assert_eq!(author_delete_response.status(), StatusCode::OK);
    assert_eq!(board_thread_count(&pool, second_board_id).await, 0);
    let lifetime_thread_count: i32 =
        sqlx::query_scalar("SELECT threads_created FROM forum.user_stats WHERE account_id = $1")
            .bind(author_id)
            .fetch_one(&pool)
            .await
            .expect("lifetime thread count");
    assert_eq!(lifetime_thread_count, 1);
}

#[tokio::test]
async fn admin_thread_actions_respect_target_author_role_hierarchy() {
    let (pool, app) = create_test_app().await;
    let (moderator_id, moderator_token) =
        create_test_account(&pool, "thread-hierarchy-mod@tongji.edu.cn", "thread-hierarchy-mod")
            .await;
    let (administrator_id, administrator_token) = create_test_account(
        &pool,
        "thread-hierarchy-admin@tongji.edu.cn",
        "thread-hierarchy-admin",
    )
    .await;
    let (user_author_id, _) =
        create_test_account(&pool, "thread-hierarchy-user@tongji.edu.cn", "thread-hierarchy-user")
            .await;
    let (moderator_author_id, _) = create_test_account(
        &pool,
        "thread-hierarchy-mod-author@tongji.edu.cn",
        "thread-hierarchy-mod-author",
    )
    .await;
    let (administrator_author_id, _) = create_test_account(
        &pool,
        "thread-hierarchy-admin-author@tongji.edu.cn",
        "thread-hierarchy-admin-author",
    )
    .await;
    sqlx::query(
        "UPDATE identity.accounts SET role = CASE \
           WHEN id IN ($1, $2) THEN 'mod'::identity.account_role \
           WHEN id IN ($3, $4) THEN 'admin'::identity.account_role ELSE role END",
    )
    .bind(moderator_id)
    .bind(moderator_author_id)
    .bind(administrator_id)
    .bind(administrator_author_id)
    .execute(&pool)
    .await
    .expect("assign hierarchy roles");
    let user_thread_id = seed_thread(&pool, user_author_id, "User target", None).await;
    let moderator_thread_id =
        seed_thread(&pool, moderator_author_id, "Moderator target", None).await;
    let administrator_thread_id =
        seed_thread(&pool, administrator_author_id, "Administrator target", None).await;

    for (thread_id, token) in [
        (moderator_thread_id, moderator_token.as_str()),
        (administrator_thread_id, moderator_token.as_str()),
        (administrator_thread_id, administrator_token.as_str()),
    ] {
        let response = admin_thread_action_request(
            &app,
            thread_id,
            "hide",
            token,
            json!({ "reason": "role hierarchy test" }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
    let feature_response = admin_thread_action_request(
        &app,
        administrator_thread_id,
        "feature",
        &administrator_token,
        json!({ "featured": true, "reason": "role hierarchy feature test" }),
    )
    .await;
    assert_eq!(feature_response.status(), StatusCode::FORBIDDEN);

    let allowed_response = admin_thread_action_request(
        &app,
        user_thread_id,
        "hide",
        &moderator_token,
        json!({ "reason": "moderator reviewed user content" }),
    )
    .await;
    assert_eq!(allowed_response.status(), StatusCode::OK);
    let hidden_ids: Vec<i64> =
        sqlx::query_scalar("SELECT id FROM forum.threads WHERE hidden_at IS NOT NULL ORDER BY id")
            .fetch_all(&pool)
            .await
            .expect("hidden thread ids");
    assert_eq!(hidden_ids, vec![user_thread_id]);
}

#[tokio::test]
async fn hot_rank_candidates_include_only_visible_threads() {
    let (pool, _) = create_test_app().await;
    let (author_id, _) =
        create_test_account(&pool, "hot-rank-author@tongji.edu.cn", "hot-rank-author").await;
    let visible_thread_id = seed_thread(&pool, author_id, "Visible hot thread", None).await;
    sqlx::query(
        "INSERT INTO forum.threads (board_id, author_id, title, hidden_at) VALUES \
           (1, $1, 'Hidden hot thread', now()), \
           (1, $1, 'Deleted hot thread', NULL), \
           (1, $1, 'Archived hot thread', NULL), \
           (1, $1, 'Legacy normal state', NULL)",
    )
    .bind(author_id)
    .execute(&pool)
    .await
    .expect("seed unavailable hot threads");
    for statement in [
        "UPDATE forum.threads SET deleted_at = now() WHERE title = 'Deleted hot thread'",
        "UPDATE forum.threads SET archived_at = now() WHERE title = 'Archived hot thread'",
        "UPDATE forum.threads SET status = 'normal' WHERE title = 'Legacy normal state'",
    ] {
        sqlx::query(statement).execute(&pool).await.expect("set unavailable hot state");
    }

    let candidates = forum::repo::hot_rank::list_visible_hot_rank_threads(&pool)
        .await
        .expect("load hot rank candidates");
    let candidate_ids: Vec<i64> =
        candidates.into_iter().map(|(thread_id, _, _)| thread_id).collect();
    assert_eq!(candidate_ids, vec![visible_thread_id]);
}

#[tokio::test]
async fn moderator_can_recover_hidden_deleted_thread_detail() {
    let (pool, app) = create_test_app().await;
    let (author_id, user_token) =
        create_test_account(&pool, "recover-thread-user@tongji.edu.cn", "recover-thread-user")
            .await;
    let (moderator_id, moderator_token) =
        create_test_account(&pool, "recover-thread-mod@tongji.edu.cn", "recover-thread-mod").await;
    sqlx::query("UPDATE identity.accounts SET role = 'mod' WHERE id = $1")
        .bind(moderator_id)
        .execute(&pool)
        .await
        .expect("promote moderator");
    let thread_id = seed_thread(&pool, author_id, "Recovery title", Some("Recovery body")).await;
    sqlx::query("UPDATE forum.threads SET hidden_at = now(), deleted_at = now() WHERE id = $1")
        .bind(thread_id)
        .execute(&pool)
        .await
        .expect("moderate thread");

    let user_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v2/admin/forum/threads/{thread_id}"))
                .header(header::AUTHORIZATION, format!("Bearer {user_token}"))
                .body(Body::empty())
                .expect("build user recovery request"),
        )
        .await
        .expect("user recovery response");
    assert_eq!(user_response.status(), StatusCode::FORBIDDEN);

    let moderator_response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v2/admin/forum/threads/{thread_id}"))
                .header(header::AUTHORIZATION, format!("Bearer {moderator_token}"))
                .body(Body::empty())
                .expect("build moderator recovery request"),
        )
        .await
        .expect("moderator recovery response");
    assert_eq!(moderator_response.status(), StatusCode::OK);
    let detail = read_json(moderator_response).await;
    assert_eq!(detail["title"], "Recovery title");
    assert_eq!(detail["body"], "Recovery body");
    assert!(detail["hiddenAt"].is_i64());
    assert!(detail["deletedAt"].is_i64());
}
