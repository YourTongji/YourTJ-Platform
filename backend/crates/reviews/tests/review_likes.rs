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
    let activity_count: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(likes_given), 0)::bigint \
         FROM activity.daily_counts WHERE account_id = $1",
    )
    .bind(liker_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(activity_count, 1, "double like should project one activity contribution");
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
    let activity_count: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(likes_given), 0)::bigint \
         FROM activity.daily_counts WHERE account_id = $1",
    )
    .bind(liker_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(activity_count, 0, "unlike should reverse the original contribution day");
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

#[tokio::test]
async fn test_like_own_review_is_rejected() {
    let (pool, app) = create_test_app().await;
    let course_id = seed_course(&pool, "CS205", "Responsible Communities").await;
    let author_id = seed_account(&pool, "self-like@tongji.edu.cn", "self-like").await;
    let token = helpers::create_access_token_for(author_id);

    let created = app
        .clone()
        .oneshot(auth_req(
            Method::POST,
            &format!("/api/v2/courses/{course_id}/reviews"),
            json!({ "rating": 5 }),
            &token,
        ))
        .await
        .unwrap();
    let review: Value = read_json(created).await;
    let review_id = review["id"].as_str().expect("review id");

    let response = app
        .oneshot(auth_req(
            Method::POST,
            &format!("/api/v2/reviews/{review_id}/like"),
            json!({}),
            &token,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn hiding_and_restoring_review_reverses_and_reactivates_like_activity() {
    let (pool, app) = create_test_app().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let course_id = seed_course(&pool, &format!("LIKE-VIS-{suffix}"), "Like visibility").await;
    let author_id = seed_account(
        &pool,
        &format!("like-vis-author-{suffix}@tongji.edu.cn"),
        &format!("like-vis-author-{suffix}"),
    )
    .await;
    let liker_id = seed_account(
        &pool,
        &format!("like-vis-liker-{suffix}@tongji.edu.cn"),
        &format!("like-vis-liker-{suffix}"),
    )
    .await;
    let admin_id = seed_account(
        &pool,
        &format!("like-vis-admin-{suffix}@tongji.edu.cn"),
        &format!("like-vis-admin-{suffix}"),
    )
    .await;
    sqlx::query("UPDATE identity.accounts SET role = 'admin' WHERE id = $1")
        .bind(admin_id)
        .execute(&pool)
        .await
        .expect("promote like visibility admin");
    let created = app
        .clone()
        .oneshot(auth_req(
            Method::POST,
            &format!("/api/v2/courses/{course_id}/reviews"),
            json!({ "rating": 4 }),
            &helpers::create_access_token_for(author_id),
        ))
        .await
        .expect("create review for visibility test");
    let review: Value = read_json(created).await;
    let review_id = review["id"].as_str().expect("review id");
    let liker_token = helpers::create_access_token_for(liker_id);
    let liked = app
        .clone()
        .oneshot(auth_req(
            Method::POST,
            &format!("/api/v2/reviews/{review_id}/like"),
            json!({}),
            &liker_token,
        ))
        .await
        .expect("like visible review");
    assert_eq!(liked.status(), StatusCode::NO_CONTENT);

    let admin_token = helpers::create_access_token_for(admin_id);
    let hidden = app
        .clone()
        .oneshot(auth_req(
            Method::POST,
            &format!("/api/v2/admin/reviews/{review_id}/toggle"),
            json!({ "reason": "hide policy-violating review" }),
            &admin_token,
        ))
        .await
        .expect("hide liked review");
    assert_eq!(hidden.status(), StatusCode::OK);
    let hidden_activity: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(likes_given), 0)::bigint \
         FROM activity.daily_counts WHERE account_id = $1",
    )
    .bind(liker_id)
    .fetch_one(&pool)
    .await
    .expect("hidden like activity");
    assert_eq!(hidden_activity, 0);

    let restored = app
        .oneshot(auth_req(
            Method::POST,
            &format!("/api/v2/admin/reviews/{review_id}/toggle"),
            json!({ "reason": "restore review after moderation appeal" }),
            &admin_token,
        ))
        .await
        .expect("restore liked review");
    assert_eq!(restored.status(), StatusCode::OK);
    let restored_activity: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(likes_given), 0)::bigint \
         FROM activity.daily_counts WHERE account_id = $1",
    )
    .bind(liker_id)
    .fetch_one(&pool)
    .await
    .expect("restored like activity");
    assert_eq!(restored_activity, 1);
    let active_events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activity.events event \
         WHERE event.source_key = $1 AND event.delta = 1 \
           AND NOT EXISTS (SELECT 1 FROM activity.events reversal \
                           WHERE reversal.reverses_event_id = event.id)",
    )
    .bind(format!("review_like:{review_id}:{liker_id}"))
    .fetch_one(&pool)
    .await
    .expect("active restored like event");
    assert_eq!(active_events, 1);
}
