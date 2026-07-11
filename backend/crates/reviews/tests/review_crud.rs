//! Integration tests for review CRUD operations.

#[path = "helpers/mod.rs"]
mod helpers;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use helpers::{auth_req, create_test_app, read_json, seed_account, seed_course};
use serde_json::{json, Value};
use tower::ServiceExt;

#[tokio::test]
async fn test_list_reviews_empty_course() {
    let (pool, app) = create_test_app().await;
    let course_id = seed_course(&pool, "CS101", "Intro to CS").await;

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v2/courses/{course_id}/reviews"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = read_json(resp).await;
    // GET returns a paginated ReviewPage object, not a bare array.
    assert!(body["items"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_create_review_requires_auth() {
    let (pool, app) = create_test_app().await;
    let course_id = seed_course(&pool, "CS102", "Data Structures").await;

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/v2/courses/{course_id}/reviews"))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({ "rating": 4 }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_create_and_list_review() {
    let (pool, app) = create_test_app().await;
    let course_id = seed_course(&pool, "CS103", "Algorithms").await;
    let account_id = seed_account(&pool, "reviewer@tongji.edu.cn", "reviewer").await;

    let token = helpers::create_access_token_for(account_id);
    let list_uri = format!("/api/v2/courses/{course_id}/reviews");
    let create_uri = format!("/api/v2/courses/{course_id}/reviews");

    // Create review
    let resp = app
        .clone()
        .oneshot(auth_req(
            Method::POST,
            &create_uri,
            json!({ "rating": 5, "comment": "Great course!", "semester": "2024S" }),
            &token,
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: Value = read_json(resp).await;
    let _review_id = body["id"].as_str().unwrap().to_string();
    assert_eq!(body["rating"], 5);
    assert_eq!(body["comment"], "Great course!");
    assert_eq!(body["semester"], "2024S");
    assert_eq!(body["authorHandle"], "reviewer");
    assert_eq!(body["status"], "visible");
    assert_eq!(body["approveCount"], 0);

    // List reviews
    let resp = app
        .clone()
        .oneshot(Request::builder().method(Method::GET).uri(&list_uri).body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = read_json(resp).await;
    // GET returns a paginated ReviewPage object, not a bare array.
    let items = body["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["rating"], 5);
    assert_eq!(items[0]["authorHandle"], "reviewer");
}

#[tokio::test]
async fn test_edit_own_review() {
    let (pool, app) = create_test_app().await;
    let course_id = seed_course(&pool, "CS104", "OS").await;
    let account_id = seed_account(&pool, "editor@tongji.edu.cn", "editor").await;
    let token = helpers::create_access_token_for(account_id);

    let create_uri = format!("/api/v2/courses/{course_id}/reviews");

    // Create first
    let resp = app
        .clone()
        .oneshot(auth_req(
            Method::POST,
            &create_uri,
            json!({ "rating": 3, "comment": "ok" }),
            &token,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: Value = read_json(resp).await;
    let review_id = body["id"].as_str().unwrap().to_string();

    // Edit
    let resp = app
        .clone()
        .oneshot(auth_req(
            Method::PATCH,
            &format!("/api/v2/reviews/{review_id}"),
            json!({ "rating": 4, "comment": "better", "score": "A" }),
            &token,
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = read_json(resp).await;
    assert_eq!(body["rating"], 4);
    assert_eq!(body["comment"], "better");
    assert_eq!(body["score"], "A");
}

#[tokio::test]
async fn test_cannot_edit_others_review() {
    let (pool, app) = create_test_app().await;
    let course_id = seed_course(&pool, "CS105", "Networks").await;
    let author_id = seed_account(&pool, "author@tongji.edu.cn", "author").await;
    let other_id = seed_account(&pool, "other@tongji.edu.cn", "other").await;

    let author_token = helpers::create_access_token_for(author_id);
    let other_token = helpers::create_access_token_for(other_id);

    let create_uri = format!("/api/v2/courses/{course_id}/reviews");

    // Author creates review
    let resp = app
        .clone()
        .oneshot(auth_req(Method::POST, &create_uri, json!({ "rating": 5 }), &author_token))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: Value = read_json(resp).await;
    let review_id = body["id"].as_str().unwrap().to_string();

    // Other tries to edit
    let resp = app
        .clone()
        .oneshot(auth_req(
            Method::PATCH,
            &format!("/api/v2/reviews/{review_id}"),
            json!({ "rating": 1 }),
            &other_token,
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_edit_nonexistent_review() {
    let (pool, app) = create_test_app().await;
    let _course_id = seed_course(&pool, "CS106", "Databases").await;
    let account_id = seed_account(&pool, "ghost@tongji.edu.cn", "ghost").await;
    let token = helpers::create_access_token_for(account_id);

    let resp = app
        .oneshot(auth_req(Method::PATCH, "/api/v2/reviews/99999", json!({ "rating": 3 }), &token))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_invalid_rating_rejected() {
    let (pool, app) = create_test_app().await;
    let course_id = seed_course(&pool, "CS107", "AI").await;
    let account_id = seed_account(&pool, "rater@tongji.edu.cn", "rater").await;
    let token = helpers::create_access_token_for(account_id);

    let create_uri = format!("/api/v2/courses/{course_id}/reviews");

    let resp = app
        .oneshot(auth_req(Method::POST, &create_uri, json!({ "rating": 6 }), &token))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn create_review_idempotency_replays_before_captcha_and_rejects_key_reuse() {
    let (pool, app) = create_test_app().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let course_id = seed_course(&pool, &format!("IDEMP-{suffix}"), "Idempotent reviews").await;
    let account_id = seed_account(
        &pool,
        &format!("idempotent-{suffix}@tongji.edu.cn"),
        &format!("idempotent-{suffix}"),
    )
    .await;
    let token = helpers::create_access_token_for(account_id);
    let uri = format!("/api/v2/courses/{course_id}/reviews");
    let idempotency_key = uuid::Uuid::new_v4().to_string();
    let request_body = json!({
        "rating": 5,
        "comment": "stable publication",
        "captchaToken": format!("idempotency-captcha-{suffix}")
    });
    let make_request = |body: Value| {
        let mut request = auth_req(Method::POST, &uri, body, &token);
        request
            .headers_mut()
            .insert("Idempotency-Key", idempotency_key.parse().expect("idempotency header"));
        request
    };

    let first = app
        .clone()
        .oneshot(make_request(request_body.clone()))
        .await
        .expect("first idempotent review response");
    assert_eq!(first.status(), StatusCode::CREATED);
    let first_body: Value = read_json(first).await;

    let replay =
        app.clone().oneshot(make_request(request_body)).await.expect("idempotent review replay");
    assert_eq!(replay.status(), StatusCode::CREATED);
    let replay_body: Value = read_json(replay).await;
    assert_eq!(replay_body, first_body);

    let conflict = app
        .oneshot(make_request(json!({
            "rating": 4,
            "comment": "different publication",
            "captchaToken": format!("idempotency-captcha-{suffix}")
        })))
        .await
        .expect("idempotency key conflict");
    assert_eq!(conflict.status(), StatusCode::CONFLICT);

    let review_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM reviews.reviews WHERE course_id = $1 AND account_id = $2",
    )
    .bind(course_id)
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("idempotent review count");
    assert_eq!(review_count, 1);
    let aggregate_count: i32 =
        sqlx::query_scalar("SELECT review_count FROM courses.courses WHERE id = $1")
            .bind(course_id)
            .fetch_one(&pool)
            .await
            .expect("idempotent course aggregate");
    assert_eq!(aggregate_count, 1);
}

#[tokio::test]
async fn search_projection_drops_stale_hidden_candidates_and_preserves_rank() {
    let (pool, _) = create_test_app().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let account_id = seed_account(
        &pool,
        &format!("search-review-{suffix}@tongji.edu.cn"),
        &format!("search-review-{suffix}"),
    )
    .await;
    let course_id = seed_course(&pool, &format!("SEARCH-{suffix}"), "搜索测试课程").await;

    let visible_id: i64 = sqlx::query_scalar(
        "INSERT INTO reviews.reviews (course_id, account_id, rating, comment, status) \
         VALUES ($1, $2, 5, 'visible review', 'visible') RETURNING id",
    )
    .bind(course_id)
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("seed visible review");
    let hidden_id: i64 = sqlx::query_scalar(
        "INSERT INTO reviews.reviews (course_id, account_id, rating, comment, status) \
         VALUES ($1, $2, 1, 'hidden review', 'hidden') RETURNING id",
    )
    .bind(course_id)
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("seed hidden review");

    let hits = reviews::search::load_review_hits(&pool, &[hidden_id, visible_id, 999_999], 10)
        .await
        .expect("load review search hits");

    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, visible_id.to_string());
    assert_eq!(hits[0].course_id, course_id.to_string());
    assert_eq!(hits[0].course_name, "搜索测试课程");
    assert_eq!(hits[0].comment.as_deref(), Some("visible review"));
}
