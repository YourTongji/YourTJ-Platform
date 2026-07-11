//! Integration coverage for public community profiles and post visibility.

#[path = "helpers/mod.rs"]
mod helpers;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use helpers::{create_test_account, create_test_app, read_json};
use serde_json::Value;
use tower::ServiceExt;

#[derive(Clone, Copy)]
enum ThreadVisibility {
    Visible,
    Hidden,
    Deleted,
    Archived,
    NonVisible,
}

async fn get(app: &Router, uri: &str) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).expect("build request"))
        .await
        .expect("profile response");
    let status = response.status();
    let body = read_json(response).await;
    (status, body)
}

async fn insert_thread(
    pool: &sqlx::PgPool,
    account_id: i64,
    title: &str,
    visibility: ThreadVisibility,
) -> i64 {
    let thread_id: i64 = sqlx::query_scalar(
        "INSERT INTO forum.threads (board_id, author_id, title, body) \
         VALUES (1, $1, $2, 'profile visibility body') RETURNING id",
    )
    .bind(account_id)
    .bind(title)
    .fetch_one(pool)
    .await
    .expect("insert profile thread");
    let statement = match visibility {
        ThreadVisibility::Visible => None,
        ThreadVisibility::Hidden => {
            Some("UPDATE forum.threads SET hidden_at = now() WHERE id = $1")
        }
        ThreadVisibility::Deleted => {
            Some("UPDATE forum.threads SET deleted_at = now() WHERE id = $1")
        }
        ThreadVisibility::Archived => {
            Some("UPDATE forum.threads SET archived_at = now() WHERE id = $1")
        }
        ThreadVisibility::NonVisible => {
            Some("UPDATE forum.threads SET status = 'pending' WHERE id = $1")
        }
    };
    if let Some(statement) = statement {
        sqlx::query(statement).bind(thread_id).execute(pool).await.expect("set thread state");
    }
    thread_id
}

async fn insert_comment(
    pool: &sqlx::PgPool,
    account_id: i64,
    thread_id: i64,
    body: &str,
    is_hidden: bool,
) -> i64 {
    let comment_id: i64 = sqlx::query_scalar(
        "INSERT INTO forum.comments (thread_id, author_id, body, hidden_at) \
         VALUES ($1, $2, $3, CASE WHEN $4 THEN now() ELSE NULL END) RETURNING id",
    )
    .bind(thread_id)
    .bind(account_id)
    .bind(body)
    .bind(is_hidden)
    .fetch_one(pool)
    .await
    .expect("insert profile comment");
    comment_id
}

#[tokio::test]
async fn profile_routes_preserve_contract_and_exclude_unavailable_content() {
    let (pool, app) = create_test_app().await;
    let (account_id, _) =
        create_test_account(&pool, "profile-boundary@tongji.edu.cn", "profile-boundary").await;
    sqlx::query(
        "INSERT INTO forum.user_stats \
         (account_id, threads_created, comments_created, votes_received) \
         VALUES ($1, 8, 13, 21)",
    )
    .bind(account_id)
    .execute(&pool)
    .await
    .expect("insert profile stats");

    let badge_id: i64 = sqlx::query_scalar(
        "INSERT INTO platform.badges (slug, name) VALUES ('boundary-reader', 'Boundary Reader') \
         ON CONFLICT (slug) DO UPDATE SET name = EXCLUDED.name RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("insert profile badge");
    sqlx::query(
        "INSERT INTO platform.account_badges (account_id, badge_id, awarded_by) \
         VALUES ($1, $2, $1)",
    )
    .bind(account_id)
    .bind(badge_id)
    .execute(&pool)
    .await
    .expect("award profile badge");

    let visible_one =
        insert_thread(&pool, account_id, "Visible one", ThreadVisibility::Visible).await;
    let visible_two =
        insert_thread(&pool, account_id, "Visible two", ThreadVisibility::Visible).await;
    let visible_three =
        insert_thread(&pool, account_id, "Visible three", ThreadVisibility::Visible).await;
    let hidden = insert_thread(&pool, account_id, "Hidden", ThreadVisibility::Hidden).await;
    let deleted = insert_thread(&pool, account_id, "Deleted", ThreadVisibility::Deleted).await;
    let archived = insert_thread(&pool, account_id, "Archived", ThreadVisibility::Archived).await;
    let non_visible =
        insert_thread(&pool, account_id, "Pending", ThreadVisibility::NonVisible).await;

    insert_comment(&pool, account_id, visible_one, "visible comment one", false).await;
    insert_comment(&pool, account_id, visible_two, "visible comment two", false).await;
    insert_comment(&pool, account_id, visible_three, "visible comment three", false).await;
    insert_comment(&pool, account_id, visible_three, "hidden comment", true).await;
    insert_comment(&pool, account_id, hidden, "comment under hidden thread", false).await;
    insert_comment(&pool, account_id, deleted, "comment under deleted thread", false).await;
    insert_comment(&pool, account_id, archived, "comment under archived thread", false).await;
    insert_comment(&pool, account_id, non_visible, "comment under pending thread", false).await;

    let (profile_status, profile) = get(&app, "/api/v2/users/PROFILE-BOUNDARY").await;
    assert_eq!(profile_status, StatusCode::OK);
    assert_eq!(profile["id"], account_id.to_string());
    assert_eq!(profile["handle"], "profile-boundary");
    assert_eq!(profile["threadCount"], 8);
    assert_eq!(profile["commentCount"], 13);
    assert_eq!(profile["votesReceived"], 21);
    assert_eq!(profile["badges"][0]["slug"], "boundary-reader");
    assert!(profile.get("email").is_none());
    assert!(profile.get("status").is_none());

    let (thread_page_status, thread_page) =
        get(&app, "/api/v2/users/profile-boundary/threads?limit=2").await;
    assert_eq!(thread_page_status, StatusCode::OK);
    assert_eq!(thread_page["items"].as_array().expect("thread items").len(), 2);
    assert_eq!(thread_page["items"][0]["id"], visible_three.to_string());
    assert_eq!(thread_page["items"][1]["id"], visible_two.to_string());
    let thread_cursor = thread_page["nextCursor"].as_str().expect("thread cursor");
    let (thread_page_two_status, thread_page_two) = get(
        &app,
        &format!("/api/v2/users/profile-boundary/threads?limit=2&cursor={thread_cursor}"),
    )
    .await;
    assert_eq!(thread_page_two_status, StatusCode::OK);
    assert_eq!(thread_page_two["items"].as_array().expect("second thread items").len(), 1);
    assert_eq!(thread_page_two["items"][0]["id"], visible_one.to_string());
    assert!(thread_page_two["nextCursor"].is_null());

    let (comment_page_status, comment_page) =
        get(&app, "/api/v2/users/profile-boundary/comments?limit=2").await;
    assert_eq!(comment_page_status, StatusCode::OK);
    let comment_cursor = comment_page["nextCursor"].as_str().expect("comment cursor");
    let first_page_bodies: Vec<&str> = comment_page["items"]
        .as_array()
        .expect("comment items")
        .iter()
        .map(|item| item["body"].as_str().expect("comment body"))
        .collect();
    assert_eq!(first_page_bodies, vec!["visible comment three", "visible comment two"]);
    let (comment_page_two_status, comment_page_two) = get(
        &app,
        &format!("/api/v2/users/profile-boundary/comments?limit=2&cursor={comment_cursor}"),
    )
    .await;
    assert_eq!(comment_page_two_status, StatusCode::OK);
    assert_eq!(comment_page_two["items"].as_array().expect("second comment items").len(), 1);
    assert_eq!(comment_page_two["items"][0]["body"], "visible comment one");
    assert!(comment_page_two["nextCursor"].is_null());

    let (missing_status, _) = get(&app, "/api/v2/users/profile-does-not-exist").await;
    assert_eq!(missing_status, StatusCode::NOT_FOUND);
}
