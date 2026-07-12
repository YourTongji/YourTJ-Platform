//! Integration tests for typed, versioned cross-device drafts.

#[path = "helpers/mod.rs"]
mod helpers;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use helpers::{create_test_account, create_test_app, read_json};
use serde_json::{json, Value};
use tower::ServiceExt;

async fn request(
    app: &axum::Router,
    token: &str,
    method: Method,
    path: &str,
    body: Option<Value>,
) -> axum::response::Response {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header(header::AUTHORIZATION, format!("Bearer {token}"));
    let request_body = if let Some(value) = body {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
        Body::from(value.to_string())
    } else {
        Body::empty()
    };
    app.clone()
        .oneshot(builder.body(request_body).expect("build draft request"))
        .await
        .expect("draft response")
}

fn thread_draft(expected_version: i64, title: &str) -> Value {
    json!({
        "draftKey": "thread:new",
        "expectedVersion": expected_version,
        "payload": {
            "kind": "thread",
            "boardId": "1",
            "title": title,
            "body": "**unfinished**",
            "contentFormat": "markdown_v1",
            "tags": ["campus"],
            "pollQuestion": "",
            "pollOptions": []
        }
    })
}

#[tokio::test]
async fn draft_contract_round_trips_full_versioned_page() {
    let (pool, app) = create_test_app().await;
    let (_, token) = create_test_account(&pool, "draft@tongji.edu.cn", "draft-owner").await;

    let created = request(
        &app,
        &token,
        Method::PUT,
        "/api/v2/me/drafts",
        Some(thread_draft(0, "First title")),
    )
    .await;
    assert_eq!(created.status(), StatusCode::OK);
    let created_json = read_json(created).await;
    assert_eq!(created_json["draftKey"], "thread:new");
    assert_eq!(created_json["version"], 1);
    assert_eq!(created_json["payload"]["kind"], "thread");

    let updated = request(
        &app,
        &token,
        Method::PUT,
        "/api/v2/me/drafts",
        Some(thread_draft(1, "Second title")),
    )
    .await;
    assert_eq!(updated.status(), StatusCode::OK);
    let updated_json = read_json(updated).await;
    assert_eq!(updated_json["version"], 2);

    let fetched = request(&app, &token, Method::GET, "/api/v2/me/drafts/thread:new", None).await;
    assert_eq!(fetched.status(), StatusCode::OK);
    let fetched_json = read_json(fetched).await;
    assert_eq!(fetched_json["payload"]["title"], "Second title");

    let listed = request(&app, &token, Method::GET, "/api/v2/me/drafts", None).await;
    assert_eq!(listed.status(), StatusCode::OK);
    let page = read_json(listed).await;
    assert_eq!(page["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(page["items"][0]["version"], 2);
    assert_eq!(page["hasMore"], false);
    assert!(page["nextCursor"].is_null());
}

#[tokio::test]
async fn stale_draft_write_conflicts_without_losing_the_newer_content() {
    let (pool, app) = create_test_app().await;
    let (_, token) = create_test_account(&pool, "draft-race@tongji.edu.cn", "draft-race").await;
    let created =
        request(&app, &token, Method::PUT, "/api/v2/me/drafts", Some(thread_draft(0, "Initial")))
            .await;
    assert_eq!(created.status(), StatusCode::OK);

    let first_app = app.clone();
    let first_token = token.clone();
    let second_app = app.clone();
    let second_token = token.clone();
    let (first, second) = tokio::join!(
        request(
            &first_app,
            &first_token,
            Method::PUT,
            "/api/v2/me/drafts",
            Some(thread_draft(1, "Tab A")),
        ),
        request(
            &second_app,
            &second_token,
            Method::PUT,
            "/api/v2/me/drafts",
            Some(thread_draft(1, "Tab B")),
        )
    );
    let statuses = [first.status(), second.status()];
    assert_eq!(statuses.iter().filter(|status| **status == StatusCode::OK).count(), 1);
    assert_eq!(statuses.iter().filter(|status| **status == StatusCode::CONFLICT).count(), 1);

    let fetched = request(&app, &token, Method::GET, "/api/v2/me/drafts/thread:new", None).await;
    let fetched_json = read_json(fetched).await;
    assert_eq!(fetched_json["version"], 2);
    assert!(matches!(fetched_json["payload"]["title"].as_str(), Some("Tab A" | "Tab B")));
}

#[tokio::test]
async fn drafts_are_owner_scoped_and_reject_mismatched_targets() {
    let (pool, app) = create_test_app().await;
    let (_, owner_token) =
        create_test_account(&pool, "draft-owner@tongji.edu.cn", "draft-private").await;
    let (_, other_token) =
        create_test_account(&pool, "draft-other@tongji.edu.cn", "draft-stranger").await;

    let created = request(
        &app,
        &owner_token,
        Method::PUT,
        "/api/v2/me/drafts",
        Some(thread_draft(0, "Private")),
    )
    .await;
    assert_eq!(created.status(), StatusCode::OK);

    let foreign =
        request(&app, &other_token, Method::GET, "/api/v2/me/drafts/thread:new", None).await;
    assert_eq!(foreign.status(), StatusCode::NOT_FOUND);

    let mismatched = request(
        &app,
        &owner_token,
        Method::PUT,
        "/api/v2/me/drafts",
        Some(json!({
            "draftKey": "comment:7",
            "expectedVersion": 0,
            "payload": {
                "kind": "comment",
                "threadId": "8",
                "body": "draft",
                "contentFormat": "markdown_v1",
                "parentId": null
            }
        })),
    )
    .await;
    assert_eq!(mismatched.status(), StatusCode::BAD_REQUEST);
}
