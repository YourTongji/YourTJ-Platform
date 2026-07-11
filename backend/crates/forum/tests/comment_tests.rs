//! Integration tests for the forum domain — comments and materialized paths.

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
            .expect("claim durable comment events");
        if events.is_empty() {
            break;
        }
        for event in events {
            match event.topic.as_str() {
                "notification" => {
                    forum::notification_delivery::deliver_event(pool, &event)
                        .await
                        .expect("deliver durable comment notification");
                }
                "achievement_award" => {
                    platform::achievements::deliver_automatic_award(pool, &event)
                        .await
                        .expect("deliver durable comment achievement");
                }
                topic => panic!("unexpected durable comment topic: {topic}"),
            }
        }
    }
}

async fn create_comment_request(
    app: &axum::Router,
    thread_id: i64,
    token: &str,
    body: serde_json::Value,
) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/v2/forum/threads/{thread_id}/comments"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(body.to_string()))
                .expect("build comment request"),
        )
        .await
        .expect("comment response")
}

async fn update_comment_request(
    app: &axum::Router,
    comment_id: i64,
    token: &str,
    body: serde_json::Value,
) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(format!("/api/v2/forum/comments/{comment_id}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(body.to_string()))
                .expect("build comment update request"),
        )
        .await
        .expect("comment update response")
}

async fn list_comments_request(
    app: &axum::Router,
    thread_id: i64,
    token: Option<&str>,
) -> axum::response::Response {
    let mut request = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v2/forum/threads/{thread_id}/comments"));
    if let Some(token) = token {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    app.clone()
        .oneshot(request.body(Body::empty()).expect("build comments list request"))
        .await
        .expect("comments list response")
}

async fn admin_comment_action_request(
    app: &axum::Router,
    comment_id: i64,
    action: &str,
    token: &str,
) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/v2/admin/forum/comments/{comment_id}/{action}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(json!({ "reason": "role hierarchy moderation test" }).to_string()))
                .expect("build admin comment request"),
        )
        .await
        .expect("admin comment response")
}

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
async fn markdown_comment_format_is_explicit_and_rejects_raw_html() {
    let (pool, app) = create_test_app().await;
    let (author_id, token) =
        create_test_account(&pool, "markdown-comment@tongji.edu.cn", "markdown-comment").await;
    let thread_id = seed_thread(&pool, author_id).await;

    let created = create_comment_request(
        &app,
        thread_id,
        &token,
        json!({ "body": "**useful** reply", "contentFormat": "markdown_v1" }),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let created = read_json(created).await;
    assert_eq!(created["contentFormat"], "markdown_v1");
    let comment_id = created["id"].as_str().expect("comment id").parse::<i64>().unwrap();
    let achievement_outbox_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM platform.outbox_events \
         WHERE recipient_account_id = $1 AND topic = 'achievement_award' \
           AND event_type = 'first-comment'",
    )
    .bind(author_id)
    .fetch_one(&pool)
    .await
    .expect("count first-comment outbox event");
    assert_eq!(achievement_outbox_count, 1);

    let rejected = create_comment_request(
        &app,
        thread_id,
        &token,
        json!({ "body": "<iframe src='https://example.com'>", "contentFormat": "markdown_v1" }),
    )
    .await;
    assert_eq!(rejected.status(), StatusCode::BAD_REQUEST);

    sqlx::query(
        "UPDATE forum.comments SET created_at = now() - interval '10 minutes' WHERE id = $1",
    )
    .bind(comment_id)
    .execute(&pool)
    .await
    .expect("age comment beyond revision grace");
    let updated = update_comment_request(
        &app,
        comment_id,
        &token,
        json!({ "body": "plain reply", "contentFormat": "plain_v1" }),
    )
    .await;
    assert_eq!(updated.status(), StatusCode::OK);
    assert_eq!(read_json(updated).await["contentFormat"], "plain_v1");
    let revision_format: String = sqlx::query_scalar(
        "SELECT old_content_format FROM forum.post_revisions \
         WHERE post_type = 'comment' AND post_id = $1 ORDER BY seq DESC LIMIT 1",
    )
    .bind(comment_id)
    .fetch_one(&pool)
    .await
    .expect("comment revision format");
    assert_eq!(revision_format, "markdown_v1");
}

#[tokio::test]
async fn concurrent_comment_edits_reject_stale_writes_atomically() {
    let (pool, app) = create_test_app().await;
    let (author_id, token) =
        create_test_account(&pool, "comment-cas@tongji.edu.cn", "comment-cas").await;
    let thread_id = seed_thread(&pool, author_id).await;
    let comment_id = seed_comment(&pool, thread_id, author_id, "original", None).await;
    sqlx::query(
        "UPDATE forum.comments SET created_at = now() - interval '10 minutes' WHERE id = $1",
    )
    .bind(comment_id)
    .execute(&pool)
    .await
    .expect("age concurrent comment");

    let first = update_comment_request(
        &app,
        comment_id,
        &token,
        json!({ "body": "first edit", "expectedVersion": 1 }),
    );
    let second = update_comment_request(
        &app,
        comment_id,
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
    assert_eq!(read_json(success_response).await["contentVersion"], 2);
    let conflict = read_json(conflict_response).await;
    assert_eq!(conflict["error"]["code"], "VERSION_CONFLICT");
    assert_eq!(conflict["error"]["details"]["currentVersion"], 2);

    let canonical: (String, i64) =
        sqlx::query_as("SELECT body, content_version FROM forum.comments WHERE id = $1")
            .bind(comment_id)
            .fetch_one(&pool)
            .await
            .expect("canonical comment after concurrent edits");
    assert!(matches!(canonical.0.as_str(), "first edit" | "second edit"));
    assert_eq!(canonical.1, 2);
    let revisions: Vec<(i32, String)> = sqlx::query_as(
        "SELECT seq, old_body FROM forum.post_revisions \
         WHERE post_type = 'comment' AND post_id = $1 ORDER BY seq",
    )
    .bind(comment_id)
    .fetch_all(&pool)
    .await
    .expect("comment revisions after concurrent edits");
    assert_eq!(revisions, vec![(1, "original".into())]);

    let legacy_writer_version: i64 = sqlx::query_scalar(
        "UPDATE forum.comments SET body = 'legacy writer edit' WHERE id = $1 \
         RETURNING content_version",
    )
    .bind(comment_id)
    .fetch_one(&pool)
    .await
    .expect("legacy comment writer version bump");
    assert_eq!(legacy_writer_version, 3);
    let stale_after_legacy = update_comment_request(
        &app,
        comment_id,
        &token,
        json!({ "body": "must conflict", "expectedVersion": 2 }),
    )
    .await;
    assert_eq!(stale_after_legacy.status(), StatusCode::CONFLICT);
    assert_eq!(read_json(stale_after_legacy).await["error"]["details"]["currentVersion"], 3);
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

    assert_eq!(resp.status(), StatusCode::CREATED);
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
    let activity_count: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(comments_created), 0)::bigint \
         FROM activity.daily_counts WHERE account_id = $1",
    )
    .bind(author_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(activity_count, 1);
}

#[tokio::test]
async fn comment_permissions_are_server_authoritative_and_hierarchy_aware() {
    let (pool, app) = create_test_app().await;
    let (author_id, author_token) = create_test_account(
        &pool,
        "comment-permission-author@tongji.edu.cn",
        "comment-permission-author",
    )
    .await;
    let (moderator_id, moderator_token) = create_test_account(
        &pool,
        "comment-permission-mod@tongji.edu.cn",
        "comment-permission-mod",
    )
    .await;
    let (admin_id, _) = create_test_account(
        &pool,
        "comment-permission-admin@tongji.edu.cn",
        "comment-permission-admin",
    )
    .await;
    sqlx::query(
        "UPDATE identity.accounts SET role = CASE \
         WHEN id = $1 THEN 'mod'::identity.account_role \
         WHEN id = $2 THEN 'admin'::identity.account_role ELSE role END",
    )
    .bind(moderator_id)
    .bind(admin_id)
    .execute(&pool)
    .await
    .expect("seed comment permission roles");
    let thread_id = seed_thread(&pool, author_id).await;
    let comment_id = seed_comment(&pool, thread_id, author_id, "author reply", None).await;
    let admin_comment_id = seed_comment(&pool, thread_id, admin_id, "admin reply", None).await;
    let comment_id_string = comment_id.to_string();
    let admin_comment_id_string = admin_comment_id.to_string();

    let anonymous = read_json(list_comments_request(&app, thread_id, None).await).await;
    let anonymous_comment = anonymous["items"]
        .as_array()
        .expect("anonymous comments")
        .iter()
        .find(|item| item["id"].as_str() == Some(comment_id_string.as_str()))
        .expect("anonymous author comment");
    assert_eq!(anonymous_comment["contentVersion"], 1);
    assert_eq!(anonymous_comment["canEdit"], false);
    assert_eq!(anonymous_comment["canDelete"], false);
    assert_eq!(anonymous_comment["canModerate"], false);

    let author = read_json(list_comments_request(&app, thread_id, Some(&author_token)).await).await;
    let author_comment = author["items"]
        .as_array()
        .expect("author comments")
        .iter()
        .find(|item| item["id"].as_str() == Some(comment_id_string.as_str()))
        .expect("author-owned comment");
    assert_eq!(author_comment["canEdit"], true);
    assert_eq!(author_comment["canDelete"], true);
    assert_eq!(author_comment["canModerate"], false);

    let moderator =
        read_json(list_comments_request(&app, thread_id, Some(&moderator_token)).await).await;
    let items = moderator["items"].as_array().expect("moderator comments");
    let user_comment = items
        .iter()
        .find(|item| item["id"].as_str() == Some(comment_id_string.as_str()))
        .expect("moderatable comment");
    let protected_comment = items
        .iter()
        .find(|item| item["id"].as_str() == Some(admin_comment_id_string.as_str()))
        .expect("protected comment");
    assert_eq!(user_comment["canEdit"], false);
    assert_eq!(user_comment["canDelete"], false);
    assert_eq!(user_comment["canModerate"], true);
    assert_eq!(protected_comment["canModerate"], false);
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

    assert_eq!(resp.status(), StatusCode::CREATED);
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

#[tokio::test]
async fn list_comments_hides_unavailable_parent_from_users_but_allows_staff() {
    let (pool, app) = create_test_app().await;
    let (author_id, user_token) =
        create_test_account(&pool, "parent-state-user@tongji.edu.cn", "parent-state-user").await;
    let (moderator_id, moderator_token) =
        create_test_account(&pool, "parent-state-mod@tongji.edu.cn", "parent-state-mod").await;
    sqlx::query("UPDATE identity.accounts SET role = 'mod' WHERE id = $1")
        .bind(moderator_id)
        .execute(&pool)
        .await
        .expect("promote moderator");
    let thread_id = seed_thread(&pool, author_id).await;
    seed_comment(&pool, thread_id, author_id, "recoverable comment", None).await;
    sqlx::query("UPDATE forum.threads SET hidden_at = now() WHERE id = $1")
        .bind(thread_id)
        .execute(&pool)
        .await
        .expect("hide parent thread");

    for token in [None, Some(user_token.as_str())] {
        let mut request = Request::builder()
            .method(Method::GET)
            .uri(format!("/api/v2/forum/threads/{thread_id}/comments"));
        if let Some(token) = token {
            request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        let response = app
            .clone()
            .oneshot(request.body(Body::empty()).expect("build list request"))
            .await
            .expect("list response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    let staff_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v2/forum/threads/{thread_id}/comments"))
                .header(header::AUTHORIZATION, format!("Bearer {moderator_token}"))
                .body(Body::empty())
                .expect("build staff list request"),
        )
        .await
        .expect("staff list response");
    assert_eq!(staff_response.status(), StatusCode::OK);

    sqlx::query("UPDATE forum.threads SET hidden_at = NULL, deleted_at = now() WHERE id = $1")
        .bind(thread_id)
        .execute(&pool)
        .await
        .expect("delete parent thread");
    let deleted_parent_response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v2/forum/threads/{thread_id}/comments"))
                .body(Body::empty())
                .expect("build deleted-parent request"),
        )
        .await
        .expect("deleted-parent response");
    assert_eq!(deleted_parent_response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn create_comment_rejects_non_writable_threads() {
    let (pool, app) = create_test_app().await;
    let (author_id, token) =
        create_test_account(&pool, "comment-state@tongji.edu.cn", "comment-state").await;

    for (column, expected_status) in [
        ("closed_at", StatusCode::CONFLICT),
        ("archived_at", StatusCode::CONFLICT),
        ("hidden_at", StatusCode::NOT_FOUND),
        ("deleted_at", StatusCode::NOT_FOUND),
    ] {
        let thread_id = seed_thread(&pool, author_id).await;
        sqlx::query(&format!("UPDATE forum.threads SET {column} = now() WHERE id = $1"))
            .bind(thread_id)
            .execute(&pool)
            .await
            .expect("set thread state");
        let response = create_comment_request(
            &app,
            thread_id,
            &token,
            json!({ "body": "must not be inserted" }),
        )
        .await;
        assert_eq!(response.status(), expected_status, "state column {column}");
    }

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM forum.comments")
        .fetch_one(&pool)
        .await
        .expect("comment count");
    assert_eq!(count, 0);
}

#[tokio::test]
async fn invalid_comment_body_is_rejected_before_insert() {
    let (pool, app) = create_test_app().await;
    let (author_id, token) =
        create_test_account(&pool, "comment-validation@tongji.edu.cn", "comment-validation").await;
    let thread_id = seed_thread(&pool, author_id).await;
    for body in ["   ".to_string(), "x".repeat(16_001)] {
        let response =
            create_comment_request(&app, thread_id, &token, json!({ "body": body })).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM forum.comments")
        .fetch_one(&pool)
        .await
        .expect("comment count");
    assert_eq!(count, 0);
}

#[tokio::test]
async fn queued_comment_is_hidden_without_activity_credit() {
    let (pool, app) = create_test_app().await;
    let (thread_author_id, _) = create_test_account(
        &pool,
        "queued-comment-thread-author@tongji.edu.cn",
        "queued-comment-thread-author",
    )
    .await;
    let (comment_author_id, token) =
        create_test_account(&pool, "queued-comment@tongji.edu.cn", "queued-comment").await;
    let (quoted_author_id, _) =
        create_test_account(&pool, "queued-comment-quoted@tongji.edu.cn", "queued-comment-quoted")
            .await;
    let (mentioned_id, _) = create_test_account(
        &pool,
        "queued-comment-mentioned@tongji.edu.cn",
        "queued-comment-mentioned",
    )
    .await;
    let (watcher_id, _) = create_test_account(
        &pool,
        "queued-comment-watcher@tongji.edu.cn",
        "queued-comment-watcher",
    )
    .await;
    let thread_id = seed_thread(&pool, thread_author_id).await;
    let quoted_comment_id =
        seed_comment(&pool, thread_id, quoted_author_id, "quoted content", None).await;
    sqlx::query("UPDATE forum.threads SET reply_count = 1 WHERE id = $1")
        .bind(thread_id)
        .execute(&pool)
        .await
        .expect("seed public reply count");
    sqlx::query(
        "INSERT INTO forum.subscriptions (account_id, target_type, target_id, level) \
         VALUES ($1, 'thread', $2, 'watching')",
    )
    .bind(watcher_id)
    .bind(thread_id)
    .execute(&pool)
    .await
    .expect("seed watcher");
    let marker = "queued-comment-marker-7c1a";
    sqlx::query("INSERT INTO forum.watched_words (word, action) VALUES ($1, 'queue')")
        .bind(marker)
        .execute(&pool)
        .await
        .expect("insert watched word");
    forum::watched_words::reload_watched_words(&pool).await.expect("reload watched words");

    let response = create_comment_request(
        &app,
        thread_id,
        &token,
        json!({
            "body": format!("{marker} @queued-comment-mentioned"),
            "quotedCommentId": quoted_comment_id.to_string()
        }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let response_body = read_json(response).await;
    let comment_id: i64 = response_body["id"].as_str().expect("comment id").parse().unwrap();
    let hidden_at: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT hidden_at FROM forum.comments WHERE id = $1")
            .bind(comment_id)
            .fetch_one(&pool)
            .await
            .expect("hidden state");
    assert!(hidden_at.is_some());
    let activity_count: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(comments_created), 0)::bigint \
         FROM activity.daily_counts WHERE account_id = $1",
    )
    .bind(comment_author_id)
    .fetch_one(&pool)
    .await
    .expect("comment activity");
    assert_eq!(activity_count, 0);
    let reply_count: i32 =
        sqlx::query_scalar("SELECT reply_count FROM forum.threads WHERE id = $1")
            .bind(thread_id)
            .fetch_one(&pool)
            .await
            .expect("reply count after queued comment");
    assert_eq!(reply_count, 1);
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    let notification_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM forum.notifications WHERE account_id = ANY($1)")
            .bind(vec![
                thread_author_id,
                comment_author_id,
                quoted_author_id,
                mentioned_id,
                watcher_id,
            ])
            .fetch_one(&pool)
            .await
            .expect("queued comment notifications");
    assert_eq!(notification_count, 0);
    let public_stat_count: i32 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(comments_created), 0)::int FROM forum.user_stats WHERE account_id = $1",
    )
    .bind(comment_author_id)
    .fetch_one(&pool)
    .await
    .expect("queued comment stats");
    assert_eq!(public_stat_count, 0);
    let auto_subscription_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM forum.subscriptions WHERE account_id = $1 AND target_id = $2",
    )
    .bind(comment_author_id)
    .bind(thread_id)
    .fetch_one(&pool)
    .await
    .expect("queued comment subscription");
    assert_eq!(auto_subscription_count, 0);

    sqlx::query("DELETE FROM forum.watched_words WHERE word = $1")
        .bind(marker)
        .execute(&pool)
        .await
        .expect("remove watched word");
    forum::watched_words::reload_watched_words(&pool).await.expect("reload watched words");
}

#[tokio::test]
async fn comment_mentions_share_the_recipient_policy_boundary() {
    let (pool, app) = create_test_app().await;
    let (actor_id, token) =
        create_test_account(&pool, "comment-mention-actor@tongji.edu.cn", "comment-mention-actor")
            .await;
    let (allowed_id, _) = create_test_account(
        &pool,
        "comment-mention-allowed@tongji.edu.cn",
        "comment-mention-allowed",
    )
    .await;
    let (denied_id, _) = create_test_account(
        &pool,
        "comment-mention-denied@tongji.edu.cn",
        "comment-mention-denied",
    )
    .await;
    for (account_id, policy) in [(allowed_id, "everyone"), (denied_id, "nobody")] {
        sqlx::query(
            "INSERT INTO identity.profile_privacy (account_id, mention_policy) VALUES ($1, $2) \
             ON CONFLICT (account_id) DO UPDATE SET mention_policy = EXCLUDED.mention_policy",
        )
        .bind(account_id)
        .bind(policy)
        .execute(&pool)
        .await
        .expect("set comment mention policy");
    }
    let thread_id = seed_thread(&pool, actor_id).await;
    let response = create_comment_request(
        &app,
        thread_id,
        &token,
        json!({
            "body": "hello @comment-mention-allowed and @comment-mention-denied",
            "contentFormat": "plain_v1"
        }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let created = read_json(response).await;
    let comment_id = created["id"].as_str().expect("created comment id");

    drain_notification_outbox(&pool).await;
    let notifications: Vec<(i64, serde_json::Value)> = sqlx::query_as(
        "SELECT account_id, payload FROM forum.notifications WHERE type = 'mention'",
    )
    .fetch_all(&pool)
    .await
    .expect("comment mention notifications");
    assert_eq!(notifications.len(), 1);
    assert_eq!(notifications[0].0, allowed_id);
    assert_eq!(notifications[0].1["commentId"], comment_id);
    assert_ne!(notifications[0].0, denied_id);
}

#[tokio::test]
async fn comment_edits_share_create_policy_and_return_complete_canonical_row() {
    let (pool, app) = create_test_app().await;
    let (author_id, token) =
        create_test_account(&pool, "comment-edit-policy@tongji.edu.cn", "comment-edit-policy")
            .await;
    let thread_id = seed_thread(&pool, author_id).await;
    let blocked_marker = "comment-edit-blocked-872f";
    let censored_marker = "comment-edit-censored-bd31";
    sqlx::query(
        "INSERT INTO forum.watched_words (word, action) VALUES ($1, 'block'), ($2, 'censor')",
    )
    .bind(blocked_marker)
    .bind(censored_marker)
    .execute(&pool)
    .await
    .expect("insert comment policy words");
    forum::watched_words::reload_watched_words(&pool).await.expect("reload watched words");

    let created = create_comment_request(
        &app,
        thread_id,
        &token,
        json!({ "body": format!("Created {censored_marker}") }),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let created_body = read_json(created).await;
    let comment_id: i64 =
        created_body["id"].as_str().expect("comment id").parse().expect("numeric comment id");
    assert!(!created_body["body"].as_str().expect("created body").contains(censored_marker));
    let stored_created: String =
        sqlx::query_scalar("SELECT body FROM forum.comments WHERE id = $1")
            .bind(comment_id)
            .fetch_one(&pool)
            .await
            .expect("stored created comment");
    assert_eq!(created_body["body"].as_str(), Some(stored_created.as_str()));
    sqlx::query(
        "UPDATE forum.comments SET created_at = now() - interval '10 minutes' WHERE id = $1",
    )
    .bind(comment_id)
    .execute(&pool)
    .await
    .expect("age comment");

    for invalid_body in ["   ".to_string(), "x".repeat(16_001), blocked_marker.to_owned()] {
        let response =
            update_comment_request(&app, comment_id, &token, json!({ "body": invalid_body })).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    let updated = update_comment_request(
        &app,
        comment_id,
        &token,
        json!({ "body": format!("Updated {censored_marker}") }),
    )
    .await;
    assert_eq!(updated.status(), StatusCode::OK);
    let updated_body = read_json(updated).await;
    let comment_id_string = comment_id.to_string();
    let thread_id_string = thread_id.to_string();
    let author_id_string = author_id.to_string();
    assert_eq!(updated_body["id"].as_str(), Some(comment_id_string.as_str()));
    assert_eq!(updated_body["threadId"].as_str(), Some(thread_id_string.as_str()));
    assert_eq!(updated_body["authorId"].as_str(), Some(author_id_string.as_str()));
    assert_eq!(updated_body["isDeleted"].as_bool(), Some(false));
    assert_eq!(updated_body["isHidden"].as_bool(), Some(false));
    assert!(updated_body["editedAt"].is_i64());
    let stored_updated: String =
        sqlx::query_scalar("SELECT body FROM forum.comments WHERE id = $1")
            .bind(comment_id)
            .fetch_one(&pool)
            .await
            .expect("stored updated comment");
    assert_eq!(updated_body["body"].as_str(), Some(stored_updated.as_str()));
    assert!(!stored_updated.contains(censored_marker));
    let revisions: Vec<String> = sqlx::query_scalar(
        "SELECT old_body FROM forum.post_revisions \
         WHERE post_type = 'comment' AND post_id = $1 ORDER BY seq",
    )
    .bind(comment_id)
    .fetch_all(&pool)
    .await
    .expect("comment revisions");
    assert_eq!(revisions, vec![stored_created]);

    sqlx::query("DELETE FROM forum.watched_words WHERE word = ANY($1)")
        .bind(vec![blocked_marker.to_owned(), censored_marker.to_owned()])
        .execute(&pool)
        .await
        .expect("remove comment policy words");
    forum::watched_words::reload_watched_words(&pool).await.expect("reload watched words");
}

#[tokio::test]
async fn queued_comment_edit_removes_public_reply_and_activity() {
    let (pool, app) = create_test_app().await;
    let (thread_author_id, _) = create_test_account(
        &pool,
        "comment-edit-queue-thread@tongji.edu.cn",
        "comment-edit-queue-thread",
    )
    .await;
    let (comment_author_id, token) = create_test_account(
        &pool,
        "comment-edit-queue-author@tongji.edu.cn",
        "comment-edit-queue-author",
    )
    .await;
    let thread_id = seed_thread(&pool, thread_author_id).await;
    let created =
        create_comment_request(&app, thread_id, &token, json!({ "body": "Initially visible" }))
            .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let comment_id: i64 = read_json(created).await["id"]
        .as_str()
        .expect("comment id")
        .parse()
        .expect("numeric comment id");
    let marker = "comment-edit-queue-marker-e022";
    sqlx::query("INSERT INTO forum.watched_words (word, action) VALUES ($1, 'queue')")
        .bind(marker)
        .execute(&pool)
        .await
        .expect("insert comment queue word");
    forum::watched_words::reload_watched_words(&pool).await.expect("reload watched words");

    let response =
        update_comment_request(&app, comment_id, &token, json!({ "body": marker })).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(read_json(response).await["isHidden"].as_bool(), Some(true));
    let reply_count: i32 =
        sqlx::query_scalar("SELECT reply_count FROM forum.threads WHERE id = $1")
            .bind(thread_id)
            .fetch_one(&pool)
            .await
            .expect("reply count after queue edit");
    assert_eq!(reply_count, 0);
    let activity_count: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(comments_created), 0)::bigint FROM activity.daily_counts WHERE account_id = $1",
    )
    .bind(comment_author_id)
    .fetch_one(&pool)
    .await
    .expect("comment activity after queue edit");
    assert_eq!(activity_count, 0);

    sqlx::query("DELETE FROM forum.watched_words WHERE word = $1")
        .bind(marker)
        .execute(&pool)
        .await
        .expect("remove comment queue word");
    forum::watched_words::reload_watched_words(&pool).await.expect("reload watched words");
}

#[tokio::test]
async fn comment_quote_must_target_available_comment_in_same_thread() {
    let (pool, app) = create_test_app().await;
    let (author_id, token) =
        create_test_account(&pool, "cross-thread-quote@tongji.edu.cn", "cross-thread-quote").await;
    let first_thread_id = seed_thread(&pool, author_id).await;
    let second_thread_id = seed_thread(&pool, author_id).await;
    let foreign_comment_id =
        seed_comment(&pool, second_thread_id, author_id, "foreign quote", None).await;

    let response = create_comment_request(
        &app,
        first_thread_id,
        &token,
        json!({ "body": "Invalid quote", "quotedCommentId": foreign_comment_id.to_string() }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let inserted: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM forum.comments WHERE thread_id = $1")
            .bind(first_thread_id)
            .fetch_one(&pool)
            .await
            .expect("cross-thread insert count");
    assert_eq!(inserted, 0);
}

#[tokio::test]
async fn comment_edit_rejects_hidden_deleted_and_archived_targets() {
    let (pool, app) = create_test_app().await;
    let (author_id, token) =
        create_test_account(&pool, "comment-edit-state@tongji.edu.cn", "comment-edit-state").await;
    let thread_id = seed_thread(&pool, author_id).await;
    let comment_id = seed_comment(&pool, thread_id, author_id, "original", None).await;

    sqlx::query("UPDATE forum.comments SET hidden_at = now() WHERE id = $1")
        .bind(comment_id)
        .execute(&pool)
        .await
        .expect("hide comment");
    let hidden =
        update_comment_request(&app, comment_id, &token, json!({ "body": "hidden edit" })).await;
    assert_eq!(hidden.status(), StatusCode::NOT_FOUND);

    sqlx::query("UPDATE forum.comments SET hidden_at = NULL WHERE id = $1")
        .bind(comment_id)
        .execute(&pool)
        .await
        .expect("unhide comment");
    sqlx::query("UPDATE forum.threads SET archived_at = now() WHERE id = $1")
        .bind(thread_id)
        .execute(&pool)
        .await
        .expect("archive parent thread");
    let archived =
        update_comment_request(&app, comment_id, &token, json!({ "body": "archived edit" })).await;
    assert_eq!(archived.status(), StatusCode::CONFLICT);

    sqlx::query("UPDATE forum.threads SET archived_at = NULL WHERE id = $1")
        .bind(thread_id)
        .execute(&pool)
        .await
        .expect("unarchive parent thread");
    sqlx::query("UPDATE forum.comments SET deleted_at = now() WHERE id = $1")
        .bind(comment_id)
        .execute(&pool)
        .await
        .expect("delete comment");
    let deleted =
        update_comment_request(&app, comment_id, &token, json!({ "body": "deleted edit" })).await;
    assert_eq!(deleted.status(), StatusCode::NOT_FOUND);

    let stored_body: String = sqlx::query_scalar("SELECT body FROM forum.comments WHERE id = $1")
        .bind(comment_id)
        .fetch_one(&pool)
        .await
        .expect("state-guarded comment body");
    assert_eq!(stored_body, "original");
}

#[tokio::test]
async fn moderator_can_recover_hidden_deleted_comment_detail() {
    let (pool, app) = create_test_app().await;
    let (author_id, user_token) =
        create_test_account(&pool, "recover-comment-user@tongji.edu.cn", "recover-comment-user")
            .await;
    let (moderator_id, moderator_token) =
        create_test_account(&pool, "recover-comment-mod@tongji.edu.cn", "recover-comment-mod")
            .await;
    sqlx::query("UPDATE identity.accounts SET role = 'mod' WHERE id = $1")
        .bind(moderator_id)
        .execute(&pool)
        .await
        .expect("promote moderator");
    let thread_id = seed_thread(&pool, author_id).await;
    let comment_id = seed_comment(&pool, thread_id, author_id, "full recovery body", None).await;
    sqlx::query("UPDATE forum.comments SET hidden_at = now(), deleted_at = now() WHERE id = $1")
        .bind(comment_id)
        .execute(&pool)
        .await
        .expect("moderate comment");

    let user_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v2/admin/forum/comments/{comment_id}"))
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
                .uri(format!("/api/v2/admin/forum/comments/{comment_id}"))
                .header(header::AUTHORIZATION, format!("Bearer {moderator_token}"))
                .body(Body::empty())
                .expect("build moderator recovery request"),
        )
        .await
        .expect("moderator recovery response");
    assert_eq!(moderator_response.status(), StatusCode::OK);
    let detail = read_json(moderator_response).await;
    assert_eq!(detail["body"], "full recovery body");
    assert_eq!(detail["isHidden"], true);
    assert_eq!(detail["isDeleted"], true);
}

#[tokio::test]
async fn admin_comment_actions_respect_target_author_role_hierarchy() {
    let (pool, app) = create_test_app().await;
    let (moderator_id, moderator_token) =
        create_test_account(&pool, "comment-hierarchy-mod@tongji.edu.cn", "comment-hierarchy-mod")
            .await;
    let (administrator_id, administrator_token) = create_test_account(
        &pool,
        "comment-hierarchy-admin@tongji.edu.cn",
        "comment-hierarchy-admin",
    )
    .await;
    let (user_author_id, _) = create_test_account(
        &pool,
        "comment-hierarchy-user@tongji.edu.cn",
        "comment-hierarchy-user",
    )
    .await;
    let (moderator_author_id, _) = create_test_account(
        &pool,
        "comment-hierarchy-mod-author@tongji.edu.cn",
        "comment-hierarchy-mod-author",
    )
    .await;
    let (administrator_author_id, _) = create_test_account(
        &pool,
        "comment-hierarchy-admin-author@tongji.edu.cn",
        "comment-hierarchy-admin-author",
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
    let thread_id = seed_thread(&pool, user_author_id).await;
    let user_comment_id = seed_comment(&pool, thread_id, user_author_id, "user target", None).await;
    let moderator_comment_id =
        seed_comment(&pool, thread_id, moderator_author_id, "moderator target", None).await;
    let administrator_comment_id =
        seed_comment(&pool, thread_id, administrator_author_id, "administrator target", None).await;

    for (comment_id, token) in [
        (moderator_comment_id, moderator_token.as_str()),
        (administrator_comment_id, moderator_token.as_str()),
        (administrator_comment_id, administrator_token.as_str()),
    ] {
        let response = admin_comment_action_request(&app, comment_id, "hide", token).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    let allowed_response =
        admin_comment_action_request(&app, user_comment_id, "hide", &moderator_token).await;
    assert_eq!(allowed_response.status(), StatusCode::OK);
    let hidden_ids: Vec<i64> =
        sqlx::query_scalar("SELECT id FROM forum.comments WHERE hidden_at IS NOT NULL ORDER BY id")
            .fetch_all(&pool)
            .await
            .expect("hidden comment ids");
    assert_eq!(hidden_ids, vec![user_comment_id]);
}
