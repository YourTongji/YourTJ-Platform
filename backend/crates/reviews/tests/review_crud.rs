//! Integration tests for review CRUD operations.

#[path = "helpers/mod.rs"]
mod helpers;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use helpers::{auth_req, create_test_app, read_json, seed_account, seed_course};
use serde_json::{json, Value};
use tower::ServiceExt;

fn assert_viewer_state(review: &Value, viewer_liked: bool, can_edit: bool, can_report: bool) {
    assert_eq!(review["viewerLiked"], viewer_liked);
    assert_eq!(review["canEdit"], can_edit);
    assert_eq!(review["canReport"], can_report);
}

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
    assert_viewer_state(&body, false, true, false);

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
    assert_viewer_state(&items[0], false, false, false);
}

#[tokio::test]
async fn viewer_permissions_and_like_state_are_server_authoritative() {
    let (pool, app) = create_test_app().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let course_id =
        seed_course(&pool, &format!("VIEWER-{suffix}"), "Viewer-aware review state").await;
    let author_id = seed_account(
        &pool,
        &format!("viewer-author-{suffix}@tongji.edu.cn"),
        &format!("viewer-author-{suffix}"),
    )
    .await;
    let viewer_id = seed_account(
        &pool,
        &format!("review-viewer-{suffix}@tongji.edu.cn"),
        &format!("review-viewer-{suffix}"),
    )
    .await;
    let author_token = helpers::create_access_token_for(author_id);
    let viewer_token = helpers::create_access_token_for(viewer_id);
    let list_uri = format!("/api/v2/courses/{course_id}/reviews");

    let created = app
        .clone()
        .oneshot(auth_req(
            Method::POST,
            &list_uri,
            json!({ "rating": 5, "comment": "viewer-aware" }),
            &author_token,
        ))
        .await
        .expect("create viewer-aware review");
    assert_eq!(created.status(), StatusCode::CREATED);
    let created_body: Value = read_json(created).await;
    assert_viewer_state(&created_body, false, true, false);
    let review_id = created_body["id"].as_str().expect("review id");

    let anonymous_list = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(&list_uri)
                .body(Body::empty())
                .expect("anonymous review list request"),
        )
        .await
        .expect("anonymous review list");
    assert_eq!(anonymous_list.status(), StatusCode::OK);
    let anonymous_body: Value = read_json(anonymous_list).await;
    assert_viewer_state(&anonymous_body["items"][0], false, false, false);

    let owner_list = app
        .clone()
        .oneshot(auth_req(Method::GET, &list_uri, json!({}), &author_token))
        .await
        .expect("owner review list");
    assert_eq!(owner_list.status(), StatusCode::OK);
    let owner_body: Value = read_json(owner_list).await;
    assert_viewer_state(&owner_body["items"][0], false, true, false);

    let viewer_list = app
        .clone()
        .oneshot(auth_req(Method::GET, &list_uri, json!({}), &viewer_token))
        .await
        .expect("viewer review list");
    assert_eq!(viewer_list.status(), StatusCode::OK);
    let viewer_body: Value = read_json(viewer_list).await;
    assert_viewer_state(&viewer_body["items"][0], false, false, true);

    let liked = app
        .clone()
        .oneshot(auth_req(
            Method::POST,
            &format!("/api/v2/reviews/{review_id}/like"),
            json!({}),
            &viewer_token,
        ))
        .await
        .expect("like review as viewer");
    assert_eq!(liked.status(), StatusCode::NO_CONTENT);

    let exact = app
        .clone()
        .oneshot(auth_req(
            Method::GET,
            &format!("/api/v2/reviews/{review_id}"),
            json!({}),
            &viewer_token,
        ))
        .await
        .expect("load exact review as viewer");
    assert_eq!(exact.status(), StatusCode::OK);
    let exact_body: Value = read_json(exact).await;
    assert_viewer_state(&exact_body, true, false, true);

    let reported = app
        .clone()
        .oneshot(auth_req(
            Method::POST,
            &format!("/api/v2/reviews/{review_id}/report"),
            json!({ "reason": "duplicate review content" }),
            &viewer_token,
        ))
        .await
        .expect("report review as viewer");
    assert_eq!(reported.status(), StatusCode::NO_CONTENT);

    let exact_after_report = app
        .clone()
        .oneshot(auth_req(
            Method::GET,
            &format!("/api/v2/reviews/{review_id}"),
            json!({}),
            &viewer_token,
        ))
        .await
        .expect("reload exact review after report");
    assert_eq!(exact_after_report.status(), StatusCode::OK);
    let exact_after_report_body: Value = read_json(exact_after_report).await;
    assert_viewer_state(&exact_after_report_body, true, false, false);

    let list_after_report = app
        .clone()
        .oneshot(auth_req(Method::GET, &list_uri, json!({}), &viewer_token))
        .await
        .expect("reload review list after report");
    assert_eq!(list_after_report.status(), StatusCode::OK);
    let list_after_report_body: Value = read_json(list_after_report).await;
    assert_viewer_state(&list_after_report_body["items"][0], true, false, false);

    let invalid_token = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(&list_uri)
                .header("Authorization", "Bearer invalid-token")
                .body(Body::empty())
                .expect("invalid optional auth request"),
        )
        .await
        .expect("invalid optional auth response");
    assert_eq!(invalid_token.status(), StatusCode::UNAUTHORIZED);

    sqlx::query("UPDATE reviews.reviews SET status = 'hidden' WHERE id = $1")
        .bind(review_id.parse::<i64>().expect("numeric review id"))
        .execute(&pool)
        .await
        .expect("hide exact review");
    let hidden_exact = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v2/reviews/{review_id}"))
                .body(Body::empty())
                .expect("hidden exact review request"),
        )
        .await
        .expect("hidden exact review response");
    assert_eq!(hidden_exact.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn silenced_viewer_has_no_review_write_permissions() {
    let (pool, app) = create_test_app().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let course_id =
        seed_course(&pool, &format!("SILENCED-{suffix}"), "Silenced viewer permissions").await;
    let author_id = seed_account(
        &pool,
        &format!("silenced-author-{suffix}@tongji.edu.cn"),
        &format!("silenced-author-{suffix}"),
    )
    .await;
    let viewer_id = seed_account(
        &pool,
        &format!("silenced-viewer-{suffix}@tongji.edu.cn"),
        &format!("silenced-viewer-{suffix}"),
    )
    .await;
    let other_review_id: i64 = sqlx::query_scalar(
        "INSERT INTO reviews.reviews (course_id, account_id, rating, comment) \
         VALUES ($1, $2, 4, 'other review') RETURNING id",
    )
    .bind(course_id)
    .bind(author_id)
    .fetch_one(&pool)
    .await
    .expect("seed another author's review");
    let own_review_id: i64 = sqlx::query_scalar(
        "INSERT INTO reviews.reviews (course_id, account_id, rating, comment) \
         VALUES ($1, $2, 5, 'own review') RETURNING id",
    )
    .bind(course_id)
    .bind(viewer_id)
    .fetch_one(&pool)
    .await
    .expect("seed viewer-owned review");
    sqlx::query(
        "INSERT INTO identity.sanctions (account_id, kind, reason) \
         VALUES ($1, 'silence', 'review permission test')",
    )
    .bind(viewer_id)
    .execute(&pool)
    .await
    .expect("silence review viewer");

    let viewer_token = helpers::create_access_token_for(viewer_id);
    let list_uri = format!("/api/v2/courses/{course_id}/reviews");
    let listed = app
        .clone()
        .oneshot(auth_req(Method::GET, &list_uri, json!({}), &viewer_token))
        .await
        .expect("list reviews as silenced viewer");
    assert_eq!(listed.status(), StatusCode::OK);
    let listed_body: Value = read_json(listed).await;
    let items = listed_body["items"].as_array().expect("review list items");
    let own_review_id_text = own_review_id.to_string();
    let own_review = items
        .iter()
        .find(|review| review["id"].as_str() == Some(own_review_id_text.as_str()))
        .expect("viewer-owned review in list");
    assert_viewer_state(own_review, false, false, false);
    let other_review_id_text = other_review_id.to_string();
    let other_review = items
        .iter()
        .find(|review| review["id"].as_str() == Some(other_review_id_text.as_str()))
        .expect("other review in list");
    assert_viewer_state(other_review, false, false, false);

    for review_id in [own_review_id, other_review_id] {
        let exact = app
            .clone()
            .oneshot(auth_req(
                Method::GET,
                &format!("/api/v2/reviews/{review_id}"),
                json!({}),
                &viewer_token,
            ))
            .await
            .expect("load exact review as silenced viewer");
        assert_eq!(exact.status(), StatusCode::OK);
        let exact_body: Value = read_json(exact).await;
        assert_viewer_state(&exact_body, false, false, false);
    }

    let edit = app
        .clone()
        .oneshot(auth_req(
            Method::PATCH,
            &format!("/api/v2/reviews/{own_review_id}"),
            json!({ "rating": 3 }),
            &viewer_token,
        ))
        .await
        .expect("edit as silenced review owner");
    assert_eq!(edit.status(), StatusCode::FORBIDDEN);

    let report = app
        .oneshot(auth_req(
            Method::POST,
            &format!("/api/v2/reviews/{other_review_id}/report"),
            json!({ "reason": "silenced report attempt" }),
            &viewer_token,
        ))
        .await
        .expect("report as silenced viewer");
    assert_eq!(report.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn review_list_defaults_to_hot_and_caches_each_limit_separately() {
    let (pool, app) = create_test_app().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let course_id =
        seed_course(&pool, &format!("LIST-PARAMS-{suffix}"), "Review list parameters").await;
    let older_review_id: i64 = sqlx::query_scalar(
        "INSERT INTO reviews.reviews \
         (course_id, rating, comment, approve_count, created_at, updated_at) \
         VALUES ($1, 5, 'older hot review', 10, now() - interval '1 hour', \
                 now() - interval '1 hour') RETURNING id",
    )
    .bind(course_id)
    .fetch_one(&pool)
    .await
    .expect("seed older hot review");
    let newer_review_id: i64 = sqlx::query_scalar(
        "INSERT INTO reviews.reviews (course_id, rating, comment, approve_count) \
         VALUES ($1, 4, 'newer review', 0) RETURNING id",
    )
    .bind(course_id)
    .fetch_one(&pool)
    .await
    .expect("seed newer review");
    let base_uri = format!("/api/v2/courses/{course_id}/reviews");

    let default_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(&base_uri)
                .body(Body::empty())
                .expect("default review list request"),
        )
        .await
        .expect("default review list response");
    assert_eq!(default_response.status(), StatusCode::OK);
    let default_body: Value = read_json(default_response).await;
    assert_eq!(default_body["items"][0]["id"], older_review_id.to_string());

    let new_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("{base_uri}?sort=new"))
                .body(Body::empty())
                .expect("new review list request"),
        )
        .await
        .expect("new review list response");
    assert_eq!(new_response.status(), StatusCode::OK);
    let new_body: Value = read_json(new_response).await;
    assert_eq!(new_body["items"][0]["id"], newer_review_id.to_string());

    let first_limit = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("{base_uri}?sort=new&limit=1"))
                .body(Body::empty())
                .expect("one-review page request"),
        )
        .await
        .expect("one-review page response");
    let first_limit_body: Value = read_json(first_limit).await;
    assert_eq!(first_limit_body["items"].as_array().expect("one-review items").len(), 1);

    let second_limit = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("{base_uri}?sort=new&limit=2"))
                .body(Body::empty())
                .expect("two-review page request"),
        )
        .await
        .expect("two-review page response");
    let second_limit_body: Value = read_json(second_limit).await;
    assert_eq!(second_limit_body["items"].as_array().expect("two-review items").len(), 2);
}

#[tokio::test]
async fn review_list_rejects_invalid_sort_cursor_and_limit() {
    let (pool, app) = create_test_app().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let course_id =
        seed_course(&pool, &format!("INVALID-LIST-{suffix}"), "Invalid review list parameters")
            .await;
    let base_uri = format!("/api/v2/courses/{course_id}/reviews");

    for query in [
        "sort=popular",
        "cursor=0",
        "cursor=-1",
        "cursor=not-a-review-id",
        "limit=0",
        "limit=-1",
        "limit=101",
        "limit=not-a-number",
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(format!("{base_uri}?{query}"))
                    .body(Body::empty())
                    .expect("invalid review list request"),
            )
            .await
            .expect("invalid review list response");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "query: {query}");
    }
}

#[tokio::test]
async fn legacy_reviewer_avatar_is_never_projected_to_public_clients() {
    let (pool, app) = create_test_app().await;
    let course_id = seed_course(&pool, "LEGACY-AVATAR", "Legacy avatar privacy").await;
    sqlx::query(
        "INSERT INTO reviews.reviews \
         (course_id, rating, reviewer_name, reviewer_avatar) VALUES ($1, 5, $2, $3)",
    )
    .bind(course_id)
    .bind("legacy-reviewer")
    .bind("https://tracker.example/collect/avatar.png?review=secret")
    .execute(&pool)
    .await
    .expect("seed legacy review with arbitrary remote avatar");

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v2/courses/{course_id}/reviews"))
                .body(Body::empty())
                .expect("build legacy review list request"),
        )
        .await
        .expect("list legacy reviews");

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = read_json(response).await;
    assert_eq!(body["items"][0]["authorHandle"], "legacy-reviewer");
    assert!(body["items"][0]["authorAvatar"].is_null());
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
    assert_viewer_state(&body, false, true, false);
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
