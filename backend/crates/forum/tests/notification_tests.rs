//! Integration tests for notification filtering, pagination, and read state.

#[path = "helpers/mod.rs"]
mod helpers;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use helpers::{create_test_account, create_test_app, read_json};
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;

async fn seed_notification(pool: &PgPool, account_id: i64, is_read: bool) -> i64 {
    sqlx::query_scalar(
        "INSERT INTO forum.notifications (account_id, type, payload, read_at) \
         VALUES ($1, 'reply', '{\"threadId\":\"1\"}'::jsonb, \
                 CASE WHEN $2 THEN now() ELSE NULL END) \
         RETURNING id",
    )
    .bind(account_id)
    .bind(is_read)
    .fetch_one(pool)
    .await
    .expect("seed notification")
}

async fn list_notifications(
    app: &axum::Router,
    token: &str,
    query: &str,
) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v2/notifications?{query}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("build notification list request"),
        )
        .await
        .expect("notification list response")
}

async fn mark_notifications_read(
    app: &axum::Router,
    token: &str,
    body: serde_json::Value,
) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/notifications/read")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .expect("build mark-read request"),
        )
        .await
        .expect("mark-read response")
}

#[tokio::test]
async fn unread_filter_paginates_without_skipping_the_lookahead_row() {
    let (pool, app) = create_test_app().await;
    let (account_id, token) =
        create_test_account(&pool, "notify-page@tongji.edu.cn", "notify-page").await;

    let oldest_unread_id = seed_notification(&pool, account_id, false).await;
    seed_notification(&pool, account_id, true).await;
    let middle_unread_id = seed_notification(&pool, account_id, false).await;
    let newest_unread_id = seed_notification(&pool, account_id, false).await;

    let first_response = list_notifications(&app, &token, "unread=true&limit=2").await;
    assert_eq!(first_response.status(), StatusCode::OK);
    let first_page = read_json(first_response).await;
    assert_eq!(first_page["items"][0]["id"], newest_unread_id.to_string());
    assert_eq!(first_page["items"][1]["id"], middle_unread_id.to_string());
    assert_eq!(first_page["nextCursor"], middle_unread_id.to_string());
    assert_eq!(first_page["hasMore"], true);
    assert_eq!(first_page["items"][0]["targetUrl"], "/forum/threads/1");

    let second_response =
        list_notifications(&app, &token, &format!("unread=true&limit=2&cursor={middle_unread_id}"))
            .await;
    assert_eq!(second_response.status(), StatusCode::OK);
    let second_page = read_json(second_response).await;
    assert_eq!(second_page["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(second_page["items"][0]["id"], oldest_unread_id.to_string());
    assert_eq!(second_page["hasMore"], false);
}

#[tokio::test]
async fn mark_read_is_scoped_to_the_authenticated_account() {
    let (pool, app) = create_test_app().await;
    let (account_id, token) =
        create_test_account(&pool, "notify-owner@tongji.edu.cn", "notify-owner").await;
    let (other_account_id, _) =
        create_test_account(&pool, "notify-other@tongji.edu.cn", "notify-other").await;
    let selected_id = seed_notification(&pool, account_id, false).await;
    let remaining_id = seed_notification(&pool, account_id, false).await;
    let foreign_id = seed_notification(&pool, other_account_id, false).await;

    let selected_response = mark_notifications_read(
        &app,
        &token,
        json!({ "ids": [selected_id.to_string(), foreign_id.to_string()] }),
    )
    .await;
    assert_eq!(selected_response.status(), StatusCode::NO_CONTENT);

    let selected_is_read: bool =
        sqlx::query_scalar("SELECT read_at IS NOT NULL FROM forum.notifications WHERE id = $1")
            .bind(selected_id)
            .fetch_one(&pool)
            .await
            .expect("selected read state");
    let remaining_is_read: bool =
        sqlx::query_scalar("SELECT read_at IS NOT NULL FROM forum.notifications WHERE id = $1")
            .bind(remaining_id)
            .fetch_one(&pool)
            .await
            .expect("remaining read state");
    let foreign_is_read: bool =
        sqlx::query_scalar("SELECT read_at IS NOT NULL FROM forum.notifications WHERE id = $1")
            .bind(foreign_id)
            .fetch_one(&pool)
            .await
            .expect("foreign read state");
    assert!(selected_is_read);
    assert!(!remaining_is_read);
    assert!(!foreign_is_read);

    let all_response = mark_notifications_read(&app, &token, json!({ "all": true })).await;
    assert_eq!(all_response.status(), StatusCode::NO_CONTENT);
    let account_unread: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM forum.notifications WHERE account_id = $1 AND read_at IS NULL",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("owner unread count");
    let foreign_unread: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM forum.notifications WHERE account_id = $1 AND read_at IS NULL",
    )
    .bind(other_account_id)
    .fetch_one(&pool)
    .await
    .expect("foreign unread count");
    assert_eq!(account_unread, 0);
    assert_eq!(foreign_unread, 1);
}

#[tokio::test]
async fn rejects_invalid_list_limits_and_ambiguous_mark_read_input() {
    let (pool, app) = create_test_app().await;
    let (_, token) = create_test_account(&pool, "notify-input@tongji.edu.cn", "notify-input").await;

    assert_eq!(list_notifications(&app, &token, "limit=0").await.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        list_notifications(&app, &token, "limit=101").await.status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        mark_notifications_read(&app, &token, json!({ "ids": [] })).await.status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        mark_notifications_read(&app, &token, json!({ "ids": ["1"], "all": true })).await.status(),
        StatusCode::BAD_REQUEST
    );
}
