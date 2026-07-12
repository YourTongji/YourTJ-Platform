//! Handler-to-database coverage for revision authorization and cursor pagination.

#[path = "helpers/mod.rs"]
mod helpers;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use helpers::{create_test_account, create_test_app, read_json};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;

async fn get(app: &axum::Router, uri: &str, token: &str) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(uri)
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("revision request"),
        )
        .await
        .expect("revision response")
}

async fn set_role(pool: &PgPool, account_id: i64, role: &str) {
    sqlx::query("UPDATE identity.accounts SET role = $2::identity.account_role WHERE id = $1")
        .bind(account_id)
        .bind(role)
        .execute(pool)
        .await
        .expect("set fixture role");
}

async fn insert_thread(pool: &PgPool, author_id: i64, title: &str) -> i64 {
    sqlx::query_scalar(
        "INSERT INTO forum.threads (board_id, author_id, title, body) \
         VALUES (1, $1, $2, 'current body') RETURNING id",
    )
    .bind(author_id)
    .bind(title)
    .fetch_one(pool)
    .await
    .expect("insert revision thread")
}

async fn insert_comment(pool: &PgPool, thread_id: i64, author_id: i64) -> i64 {
    sqlx::query_scalar(
        "INSERT INTO forum.comments (thread_id, author_id, body, path) \
         VALUES ($1, $2, 'current comment', '0001') RETURNING id",
    )
    .bind(thread_id)
    .bind(author_id)
    .fetch_one(pool)
    .await
    .expect("insert revision comment")
}

async fn insert_revision(pool: &PgPool, post_type: &str, post_id: i64, editor_id: i64, seq: i32) {
    sqlx::query(
        "INSERT INTO forum.post_revisions \
         (post_type, post_id, seq, editor_id, old_title, old_body, \
          old_content_format, old_content_version) \
         VALUES ($1, $2, $3, $4, NULL, $5, 'plain_v1', $6)",
    )
    .bind(post_type)
    .bind(post_id)
    .bind(seq)
    .bind(editor_id)
    .bind(format!("old body {seq}"))
    .bind(i64::from(seq))
    .execute(pool)
    .await
    .expect("insert post revision");
}

async fn expect_status(app: &axum::Router, uri: &str, token: &str, expected: StatusCode) {
    let response = get(app, uri, token).await;
    assert_eq!(response.status(), expected, "unexpected status for {uri}");
}

#[tokio::test]
async fn revisions_allow_authors_and_only_strictly_lower_role_staff_targets() {
    let (pool, app) = create_test_app().await;
    let (user_author_id, user_author_token) =
        create_test_account(&pool, "revision-user@tongji.edu.cn", "revision-user").await;
    let (_, ordinary_other_token) =
        create_test_account(&pool, "revision-other@tongji.edu.cn", "revision-other").await;
    let (moderator_author_id, moderator_author_token) =
        create_test_account(&pool, "revision-mod-author@tongji.edu.cn", "revision-mod-author")
            .await;
    let (moderator_viewer_id, moderator_viewer_token) =
        create_test_account(&pool, "revision-mod-viewer@tongji.edu.cn", "revision-mod-viewer")
            .await;
    let (administrator_author_id, administrator_author_token) =
        create_test_account(&pool, "revision-admin-author@tongji.edu.cn", "revision-admin-author")
            .await;
    let (administrator_viewer_id, administrator_viewer_token) =
        create_test_account(&pool, "revision-admin-viewer@tongji.edu.cn", "revision-admin-viewer")
            .await;
    for account_id in [moderator_author_id, moderator_viewer_id] {
        set_role(&pool, account_id, "mod").await;
    }
    for account_id in [administrator_author_id, administrator_viewer_id] {
        set_role(&pool, account_id, "admin").await;
    }

    let user_thread_id = insert_thread(&pool, user_author_id, "user revisions").await;
    let moderator_thread_id =
        insert_thread(&pool, moderator_author_id, "moderator revisions").await;
    let administrator_thread_id =
        insert_thread(&pool, administrator_author_id, "administrator revisions").await;
    let administrator_own_thread_id =
        insert_thread(&pool, administrator_viewer_id, "administrator own revisions").await;
    for (thread_id, editor_id) in [
        (user_thread_id, user_author_id),
        (moderator_thread_id, moderator_author_id),
        (administrator_thread_id, administrator_author_id),
        (administrator_own_thread_id, administrator_viewer_id),
    ] {
        insert_revision(&pool, "thread", thread_id, editor_id, 1).await;
    }

    expect_status(
        &app,
        &format!("/api/v2/forum/threads/{user_thread_id}/revisions"),
        &user_author_token,
        StatusCode::OK,
    )
    .await;
    expect_status(
        &app,
        &format!("/api/v2/forum/threads/{moderator_thread_id}/revisions"),
        &moderator_author_token,
        StatusCode::OK,
    )
    .await;
    expect_status(
        &app,
        &format!("/api/v2/forum/threads/{administrator_own_thread_id}/revisions"),
        &administrator_viewer_token,
        StatusCode::OK,
    )
    .await;
    expect_status(
        &app,
        &format!("/api/v2/forum/threads/{user_thread_id}/revisions"),
        &ordinary_other_token,
        StatusCode::FORBIDDEN,
    )
    .await;
    expect_status(
        &app,
        &format!("/api/v2/forum/threads/{user_thread_id}/revisions"),
        &moderator_viewer_token,
        StatusCode::OK,
    )
    .await;
    for thread_id in [moderator_thread_id, administrator_thread_id] {
        expect_status(
            &app,
            &format!("/api/v2/forum/threads/{thread_id}/revisions"),
            &moderator_viewer_token,
            StatusCode::FORBIDDEN,
        )
        .await;
    }
    expect_status(
        &app,
        &format!("/api/v2/forum/threads/{moderator_thread_id}/revisions"),
        &administrator_viewer_token,
        StatusCode::OK,
    )
    .await;
    expect_status(
        &app,
        &format!("/api/v2/forum/threads/{administrator_thread_id}/revisions"),
        &administrator_viewer_token,
        StatusCode::FORBIDDEN,
    )
    .await;

    let comment_id = insert_comment(&pool, user_thread_id, user_author_id).await;
    insert_revision(&pool, "comment", comment_id, user_author_id, 1).await;
    for (token, expected) in [
        (&user_author_token, StatusCode::OK),
        (&ordinary_other_token, StatusCode::FORBIDDEN),
        (&moderator_viewer_token, StatusCode::OK),
        (&administrator_author_token, StatusCode::OK),
    ] {
        expect_status(
            &app,
            &format!("/api/v2/forum/comments/{comment_id}/revisions"),
            token,
            expected,
        )
        .await;
    }
}

#[tokio::test]
async fn revision_history_uses_bounded_opaque_cursor_pages() {
    let (pool, app) = create_test_app().await;
    let (author_id, token) =
        create_test_account(&pool, "revision-page@tongji.edu.cn", "revision-page").await;
    let thread_id = insert_thread(&pool, author_id, "paginated revisions").await;
    for seq in 1..=3 {
        insert_revision(&pool, "thread", thread_id, author_id, seq).await;
    }

    let first =
        get(&app, &format!("/api/v2/forum/threads/{thread_id}/revisions?limit=2"), &token).await;
    assert_eq!(first.status(), StatusCode::OK);
    let first: Value = read_json(first).await;
    assert_eq!(first["items"].as_array().map(Vec::len), Some(2));
    assert_eq!(first["items"][0]["seq"], 3);
    assert_eq!(first["items"][1]["seq"], 2);
    assert_eq!(first["hasMore"], true);
    let cursor = first["nextCursor"].as_str().expect("next revision cursor");

    let second = get(
        &app,
        &format!("/api/v2/forum/threads/{thread_id}/revisions?limit=2&cursor={cursor}"),
        &token,
    )
    .await;
    assert_eq!(second.status(), StatusCode::OK);
    let second = read_json(second).await;
    assert_eq!(second["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(second["items"][0]["seq"], 1);
    assert_eq!(second["hasMore"], false);
    assert!(second["nextCursor"].is_null());

    for query in ["limit=0", "limit=101", "cursor=bm90LW51bWVyaWM"] {
        expect_status(
            &app,
            &format!("/api/v2/forum/threads/{thread_id}/revisions?{query}"),
            &token,
            StatusCode::BAD_REQUEST,
        )
        .await;
    }
}
