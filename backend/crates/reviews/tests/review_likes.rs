//! Integration tests for review likes.

#[path = "helpers/mod.rs"]
mod helpers;

use axum::http::{Method, StatusCode};
use helpers::{auth_req, create_test_app, read_json, seed_account, seed_course};
use serde_json::{json, Value};
use tower::ServiceExt;

#[tokio::test]
async fn test_like_review() {
    let (pool, app) = create_test_app().await;
    let course_id = seed_course(&pool, "CS201", "Compilers").await;
    let author_id = seed_account(&pool, "author2@tongji.edu.cn", "author2").await;
    let liker_id = seed_account(&pool, "liker@tongji.edu.cn", "liker").await;

    let author_token = helpers::create_access_token_for(author_id);
    let liker_token = helpers::create_access_token_for(liker_id);

    // Author creates review
    let create_uri = format!("/api/v2/courses/{course_id}/reviews");
    let resp = app
        .clone()
        .oneshot(auth_req(Method::POST, &create_uri, json!({ "rating": 4 }), &author_token))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: Value = read_json(resp).await;
    let review_id = body["id"].as_str().unwrap().to_string();
    let review_id_i64: i64 = body["id"].as_str().unwrap().parse().unwrap();

    // Liker likes the review
    let resp = app
        .clone()
        .oneshot(auth_req(
            Method::POST,
            &format!("/api/v2/reviews/{review_id}/like"),
            json!({}),
            &liker_token,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Verify approve count incremented
    let count: i32 = sqlx::query_scalar("SELECT approve_count FROM reviews.reviews WHERE id = $1")
        .bind(review_id_i64)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn test_double_like_is_idempotent() {
    let (pool, app) = create_test_app().await;
    let course_id = seed_course(&pool, "CS202", "Graphics").await;
    let author_id = seed_account(&pool, "author3@tongji.edu.cn", "author3").await;
    let liker_id = seed_account(&pool, "liker2@tongji.edu.cn", "liker2").await;

    let author_token = helpers::create_access_token_for(author_id);
    let liker_token = helpers::create_access_token_for(liker_id);

    let create_uri = format!("/api/v2/courses/{course_id}/reviews");
    let resp = app
        .clone()
        .oneshot(auth_req(Method::POST, &create_uri, json!({ "rating": 3 }), &author_token))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: Value = read_json(resp).await;
    let review_id = body["id"].as_str().unwrap().to_string();
    let review_id_i64: i64 = body["id"].as_str().unwrap().parse().unwrap();

    // Like twice — second should be no-op
    app.clone()
        .oneshot(auth_req(
            Method::POST,
            &format!("/api/v2/reviews/{review_id}/like"),
            json!({}),
            &liker_token,
        ))
        .await
        .unwrap();

    app.clone()
        .oneshot(auth_req(
            Method::POST,
            &format!("/api/v2/reviews/{review_id}/like"),
            json!({}),
            &liker_token,
        ))
        .await
        .unwrap();

    let count: i32 = sqlx::query_scalar("SELECT approve_count FROM reviews.reviews WHERE id = $1")
        .bind(review_id_i64)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1, "double like should not increment count twice");
}

#[tokio::test]
async fn test_unlike_review() {
    let (pool, app) = create_test_app().await;
    let course_id = seed_course(&pool, "CS203", "Architecture").await;
    let author_id = seed_account(&pool, "author4@tongji.edu.cn", "author4").await;
    let liker_id = seed_account(&pool, "liker3@tongji.edu.cn", "liker3").await;

    let author_token = helpers::create_access_token_for(author_id);
    let liker_token = helpers::create_access_token_for(liker_id);

    let create_uri = format!("/api/v2/courses/{course_id}/reviews");
    let resp = app
        .clone()
        .oneshot(auth_req(Method::POST, &create_uri, json!({ "rating": 5 }), &author_token))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: Value = read_json(resp).await;
    let review_id = body["id"].as_str().unwrap().to_string();
    let review_id_i64: i64 = body["id"].as_str().unwrap().parse().unwrap();

    // Like then unlike
    app.clone()
        .oneshot(auth_req(
            Method::POST,
            &format!("/api/v2/reviews/{review_id}/like"),
            json!({}),
            &liker_token,
        ))
        .await
        .unwrap();

    app.clone()
        .oneshot(auth_req(
            Method::POST,
            &format!("/api/v2/reviews/{review_id}/unlike"),
            json!({}),
            &liker_token,
        ))
        .await
        .unwrap();

    let count: i32 = sqlx::query_scalar("SELECT approve_count FROM reviews.reviews WHERE id = $1")
        .bind(review_id_i64)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn test_like_requires_auth() {
    let (pool, app) = create_test_app().await;
    let course_id = seed_course(&pool, "CS204", "Distributed").await;
    let author_id = seed_account(&pool, "author5@tongji.edu.cn", "author5").await;
    let token = helpers::create_access_token_for(author_id);

    let create_uri = format!("/api/v2/courses/{course_id}/reviews");
    let resp = app
        .clone()
        .oneshot(auth_req(Method::POST, &create_uri, json!({ "rating": 5 }), &token))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: Value = read_json(resp).await;
    let review_id = body["id"].as_str().unwrap().to_string();

    let resp = app
        .oneshot(
            axum::http::Request::builder()
                .method(Method::POST)
                .uri(format!("/api/v2/reviews/{review_id}/like"))
                .header("Content-Type", "application/json")
                .body(axum::body::Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
