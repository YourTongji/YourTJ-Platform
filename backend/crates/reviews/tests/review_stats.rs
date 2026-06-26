//! Integration tests for incremental review stats.

#[path = "helpers/mod.rs"]
mod helpers;

use axum::http::Method;
use helpers::{auth_req, create_test_app, read_json, seed_account, seed_course};
use serde_json::{json, Value};
use tower::ServiceExt;

#[tokio::test]
async fn test_review_count_increments() {
    let (pool, app) = create_test_app().await;
    let course_id = seed_course(&pool, "CS401", "Stats").await;
    let account_id = seed_account(&pool, "rev1@tongji.edu.cn", "rev1").await;
    let token = helpers::create_access_token_for(account_id);

    let create_uri = format!("/api/v2/courses/{course_id}/reviews");

    app.clone()
        .oneshot(auth_req(Method::POST, &create_uri, json!({ "rating": 4 }), &token))
        .await
        .unwrap();

    let count: i32 = sqlx::query_scalar("SELECT review_count FROM courses.courses WHERE id = $1")
        .bind(course_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn test_review_avg_updates() {
    let (pool, app) = create_test_app().await;
    let course_id = seed_course(&pool, "CS402", "Math").await;
    let a1 = seed_account(&pool, "r1@tongji.edu.cn", "r1").await;
    let a2 = seed_account(&pool, "r2@tongji.edu.cn", "r2").await;
    let t1 = helpers::create_access_token_for(a1);
    let t2 = helpers::create_access_token_for(a2);

    let create_uri = format!("/api/v2/courses/{course_id}/reviews");

    // First review: rating 2
    app.clone()
        .oneshot(auth_req(Method::POST, &create_uri, json!({ "rating": 2 }), &t1))
        .await
        .unwrap();

    let avg: f64 = sqlx::query_scalar("SELECT review_avg FROM courses.courses WHERE id = $1")
        .bind(course_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!((avg - 2.0).abs() < 0.01, "expected avg=2, got {avg}");

    // Second review: rating 4 → avg should be (2+4)/2 = 3
    app.clone()
        .oneshot(auth_req(Method::POST, &create_uri, json!({ "rating": 4 }), &t2))
        .await
        .unwrap();

    let avg: f64 = sqlx::query_scalar("SELECT review_avg FROM courses.courses WHERE id = $1")
        .bind(course_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!((avg - 3.0).abs() < 0.01, "expected avg=3, got {avg}");

    let count: i32 = sqlx::query_scalar("SELECT review_count FROM courses.courses WHERE id = $1")
        .bind(course_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 2);
}

#[tokio::test]
async fn test_review_avg_updates_on_edit() {
    let (pool, app) = create_test_app().await;
    let course_id = seed_course(&pool, "CS403", "Physics").await;
    let a1 = seed_account(&pool, "e1@tongji.edu.cn", "e1").await;
    let a2 = seed_account(&pool, "e2@tongji.edu.cn", "e2").await;
    let t1 = helpers::create_access_token_for(a1);
    let t2 = helpers::create_access_token_for(a2);

    let create_uri = format!("/api/v2/courses/{course_id}/reviews");

    // Two reviews: ratings 1 and 5
    let resp = app
        .clone()
        .oneshot(auth_req(Method::POST, &create_uri, json!({ "rating": 1 }), &t1))
        .await
        .unwrap();
    let body: Value = read_json(resp).await;
    let r1_id = body["id"].as_str().unwrap().to_string();

    app.clone()
        .oneshot(auth_req(Method::POST, &create_uri, json!({ "rating": 5 }), &t2))
        .await
        .unwrap();

    // Edit review 1: change rating from 1 → 5. Avg should become (5+5)/2 = 5
    app.clone()
        .oneshot(auth_req(
            Method::PATCH,
            &format!("/api/v2/reviews/{r1_id}"),
            json!({ "rating": 5 }),
            &t1,
        ))
        .await
        .unwrap();

    let avg: f64 = sqlx::query_scalar("SELECT review_avg FROM courses.courses WHERE id = $1")
        .bind(course_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!((avg - 5.0).abs() < 0.01, "expected avg=5 after edit, got {avg}");

    let count: i32 = sqlx::query_scalar("SELECT review_count FROM courses.courses WHERE id = $1")
        .bind(course_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 2, "count should not change on edit");
}
