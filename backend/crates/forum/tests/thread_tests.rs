//! Integration tests for the forum domain — threads.

#[path = "helpers/mod.rs"]
mod helpers;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use helpers::{create_test_account, create_test_app, read_json};
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;

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
        "/api/v2/forum/threads?sort=following",
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
        json!({ "boardId": "1", "title": "Queued", "body": marker }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = read_json(response).await;
    let thread_id: i64 = body["id"].as_str().expect("thread id").parse().unwrap();
    assert!(body["hiddenAt"].is_i64());
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

    sqlx::query("DELETE FROM forum.watched_words WHERE word = $1")
        .bind(marker)
        .execute(&pool)
        .await
        .expect("remove watched word");
    forum::watched_words::reload_watched_words(&pool).await.expect("reload watched words");
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
