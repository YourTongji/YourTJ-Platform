//! Integration tests for review reports.

#[path = "helpers/mod.rs"]
mod helpers;

use axum::http::{Method, StatusCode};
use helpers::{auth_req, create_test_app, read_json, seed_account, seed_course};
use serde_json::{json, Value};
use tower::ServiceExt;

#[tokio::test]
async fn test_report_review() {
    let (pool, app) = create_test_app().await;
    let course_id = seed_course(&pool, "CS301", "ML").await;
    let author_id = seed_account(&pool, "author6@tongji.edu.cn", "author6").await;
    let reporter_id = seed_account(&pool, "reporter@tongji.edu.cn", "reporter").await;

    let author_token = helpers::create_access_token_for(author_id);
    let reporter_token = helpers::create_access_token_for(reporter_id);

    let create_uri = format!("/api/v2/courses/{course_id}/reviews");
    let resp = app
        .clone()
        .oneshot(auth_req(Method::POST, &create_uri, json!({ "rating": 2 }), &author_token))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: Value = read_json(resp).await;
    let review_id = body["id"].as_str().unwrap().to_string();
    let review_id_i64: i64 = body["id"].as_str().unwrap().parse().unwrap();

    let resp = app
        .oneshot(auth_req(
            Method::POST,
            &format!("/api/v2/reviews/{review_id}/report"),
            json!({ "reason": "inappropriate" }),
            &reporter_token,
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM reviews.review_reports WHERE review_id = $1")
            .bind(review_id_i64)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn test_duplicate_report_prevented() {
    let (pool, app) = create_test_app().await;
    let course_id = seed_course(&pool, "CS302", "NLP").await;
    let author_id = seed_account(&pool, "author7@tongji.edu.cn", "author7").await;
    let reporter_id = seed_account(&pool, "reporter2@tongji.edu.cn", "reporter2").await;

    let author_token = helpers::create_access_token_for(author_id);
    let reporter_token = helpers::create_access_token_for(reporter_id);

    let create_uri = format!("/api/v2/courses/{course_id}/reviews");
    let resp = app
        .clone()
        .oneshot(auth_req(Method::POST, &create_uri, json!({ "rating": 1 }), &author_token))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: Value = read_json(resp).await;
    let review_id = body["id"].as_str().unwrap().to_string();

    // First report
    let resp = app
        .clone()
        .oneshot(auth_req(
            Method::POST,
            &format!("/api/v2/reviews/{review_id}/report"),
            json!({ "reason": "spam" }),
            &reporter_token,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Second report (same reporter, same review)
    let resp = app
        .oneshot(auth_req(
            Method::POST,
            &format!("/api/v2/reviews/{review_id}/report"),
            json!({ "reason": "still spam" }),
            &reporter_token,
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_report_nonexistent_review() {
    let (pool, app) = create_test_app().await;
    let _course_id = seed_course(&pool, "CS303", "CV").await;
    let reporter_id = seed_account(&pool, "reporter3@tongji.edu.cn", "reporter3").await;
    let token = helpers::create_access_token_for(reporter_id);

    let resp = app
        .oneshot(auth_req(
            Method::POST,
            "/api/v2/reviews/99999/report",
            json!({ "reason": "does not exist" }),
            &token,
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_report_requires_auth() {
    let (pool, app) = create_test_app().await;
    let course_id = seed_course(&pool, "CS304", "Robotics").await;
    let author_id = seed_account(&pool, "author8@tongji.edu.cn", "author8").await;
    let token = helpers::create_access_token_for(author_id);

    let create_uri = format!("/api/v2/courses/{course_id}/reviews");
    let resp = app
        .clone()
        .oneshot(auth_req(Method::POST, &create_uri, json!({ "rating": 3 }), &token))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: Value = read_json(resp).await;
    let review_id = body["id"].as_str().unwrap().to_string();

    let resp = app
        .oneshot(
            axum::http::Request::builder()
                .method(Method::POST)
                .uri(format!("/api/v2/reviews/{review_id}/report"))
                .header("Content-Type", "application/json")
                .body(axum::body::Body::from(json!({ "reason": "bad" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
