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
                .body(axum::body::Body::from(
                    json!({ "reason": "bad", "captchaToken": "unauthenticated-report" })
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn admin_uphold_hides_review_and_records_audit() {
    let (pool, app) = create_test_app().await;
    let course_id = seed_course(&pool, "CS305", "Community Safety").await;
    let author_id = seed_account(&pool, "author9@tongji.edu.cn", "author9").await;
    let reporter_id = seed_account(&pool, "reporter4@tongji.edu.cn", "reporter4").await;
    let admin_id = seed_account(&pool, "admin1@tongji.edu.cn", "admin1").await;
    sqlx::query("UPDATE identity.accounts SET role = 'admin' WHERE id = $1")
        .bind(admin_id)
        .execute(&pool)
        .await
        .expect("promote test admin");

    let create = app
        .clone()
        .oneshot(auth_req(
            Method::POST,
            &format!("/api/v2/courses/{course_id}/reviews"),
            json!({ "rating": 1, "comment": "reported review" }),
            &helpers::create_access_token_for(author_id),
        ))
        .await
        .expect("create review request");
    let review: Value = read_json(create).await;
    let review_id = review["id"].as_str().expect("review id");
    let report = app
        .clone()
        .oneshot(auth_req(
            Method::POST,
            &format!("/api/v2/reviews/{review_id}/report"),
            json!({ "reason": "policy violation" }),
            &helpers::create_access_token_for(reporter_id),
        ))
        .await
        .expect("report request");
    assert_eq!(report.status(), StatusCode::NO_CONTENT);
    let report_id: i64 =
        sqlx::query_scalar("SELECT id FROM reviews.review_reports WHERE review_id = $1")
            .bind(review_id.parse::<i64>().expect("numeric review id"))
            .fetch_one(&pool)
            .await
            .expect("report id");

    let decision = app
        .clone()
        .oneshot(auth_req(
            Method::POST,
            &format!("/api/v2/admin/reports/{report_id}/resolve"),
            json!({ "action": "uphold", "note": "confirmed policy violation" }),
            &helpers::create_access_token_for(admin_id),
        ))
        .await
        .expect("decision request");
    assert_eq!(decision.status(), StatusCode::OK);
    let decision_body: Value = read_json(decision).await;
    assert_eq!(decision_body["status"], "upheld");
    let status: String =
        sqlx::query_scalar("SELECT status::text FROM reviews.reviews WHERE id = $1")
            .bind(review_id.parse::<i64>().expect("numeric review id"))
            .fetch_one(&pool)
            .await
            .expect("review status");
    assert_eq!(status, "hidden");
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM governance.audit_events \
         WHERE action = 'reviews.report.decided' AND target_id = $1",
    )
    .bind(report_id.to_string())
    .fetch_one(&pool)
    .await
    .expect("audit count");
    assert_eq!(audit_count, 1);

    let duplicate = app
        .oneshot(auth_req(
            Method::POST,
            &format!("/api/v2/admin/reports/{report_id}/resolve"),
            json!({ "action": "reject", "note": "attempted second decision" }),
            &helpers::create_access_token_for(admin_id),
        ))
        .await
        .expect("duplicate decision request");
    assert_eq!(duplicate.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn terminal_report_allows_a_new_open_report_from_same_reporter() {
    let (pool, app) = create_test_app().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let course_id = seed_course(&pool, &format!("REREPORT-{suffix}"), "Re-report policy").await;
    let author_id = seed_account(
        &pool,
        &format!("rereport-author-{suffix}@tongji.edu.cn"),
        &format!("rereport-author-{suffix}"),
    )
    .await;
    let reporter_id = seed_account(
        &pool,
        &format!("rereport-user-{suffix}@tongji.edu.cn"),
        &format!("rereport-user-{suffix}"),
    )
    .await;
    let admin_id = seed_account(
        &pool,
        &format!("rereport-admin-{suffix}@tongji.edu.cn"),
        &format!("rereport-admin-{suffix}"),
    )
    .await;
    sqlx::query("UPDATE identity.accounts SET role = 'admin' WHERE id = $1")
        .bind(admin_id)
        .execute(&pool)
        .await
        .expect("promote re-report admin");
    let created = app
        .clone()
        .oneshot(auth_req(
            Method::POST,
            &format!("/api/v2/courses/{course_id}/reviews"),
            json!({ "rating": 2 }),
            &helpers::create_access_token_for(author_id),
        ))
        .await
        .expect("create re-report review");
    let review: Value = read_json(created).await;
    let review_id = review["id"].as_str().expect("review id");
    let reporter_token = helpers::create_access_token_for(reporter_id);
    let first = app
        .clone()
        .oneshot(auth_req(
            Method::POST,
            &format!("/api/v2/reviews/{review_id}/report"),
            json!({ "reason": "first report" }),
            &reporter_token,
        ))
        .await
        .expect("first report");
    assert_eq!(first.status(), StatusCode::NO_CONTENT);
    let first_report_id: i64 = sqlx::query_scalar(
        "SELECT id FROM reviews.review_reports \
         WHERE review_id = $1 AND reporter_account_id = $2 AND status = 'open'",
    )
    .bind(review_id.parse::<i64>().expect("numeric review id"))
    .bind(reporter_id)
    .fetch_one(&pool)
    .await
    .expect("first report id");
    let rejected = app
        .clone()
        .oneshot(auth_req(
            Method::POST,
            &format!("/api/v2/admin/reports/{first_report_id}/resolve"),
            json!({ "action": "reject", "note": "first report not substantiated" }),
            &helpers::create_access_token_for(admin_id),
        ))
        .await
        .expect("reject first report");
    assert_eq!(rejected.status(), StatusCode::OK);

    let second = app
        .oneshot(auth_req(
            Method::POST,
            &format!("/api/v2/reviews/{review_id}/report"),
            json!({ "reason": "new conduct after edit" }),
            &reporter_token,
        ))
        .await
        .expect("second report");
    assert_eq!(second.status(), StatusCode::NO_CONTENT);
    let statuses: Vec<String> = sqlx::query_scalar(
        "SELECT status FROM reviews.review_reports \
         WHERE review_id = $1 AND reporter_account_id = $2 ORDER BY id",
    )
    .bind(review_id.parse::<i64>().expect("numeric review id"))
    .bind(reporter_id)
    .fetch_all(&pool)
    .await
    .expect("report history");
    assert_eq!(statuses, vec!["rejected", "open"]);
}

#[tokio::test]
async fn report_requires_visible_review_and_rejects_self_report() {
    let (pool, app) = create_test_app().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let course_id = seed_course(&pool, &format!("REPORTABLE-{suffix}"), "Report visibility").await;
    let author_id = seed_account(
        &pool,
        &format!("reportable-author-{suffix}@tongji.edu.cn"),
        &format!("reportable-author-{suffix}"),
    )
    .await;
    let reporter_id = seed_account(
        &pool,
        &format!("reportable-user-{suffix}@tongji.edu.cn"),
        &format!("reportable-user-{suffix}"),
    )
    .await;
    let created = app
        .clone()
        .oneshot(auth_req(
            Method::POST,
            &format!("/api/v2/courses/{course_id}/reviews"),
            json!({ "rating": 3 }),
            &helpers::create_access_token_for(author_id),
        ))
        .await
        .expect("create reportable review");
    let review: Value = read_json(created).await;
    let review_id = review["id"].as_str().expect("review id");

    let self_report = app
        .clone()
        .oneshot(auth_req(
            Method::POST,
            &format!("/api/v2/reviews/{review_id}/report"),
            json!({ "reason": "self report attempt" }),
            &helpers::create_access_token_for(author_id),
        ))
        .await
        .expect("self report response");
    assert_eq!(self_report.status(), StatusCode::BAD_REQUEST);

    sqlx::query("UPDATE reviews.reviews SET status = 'hidden' WHERE id = $1")
        .bind(review_id.parse::<i64>().expect("numeric review id"))
        .execute(&pool)
        .await
        .expect("hide review fixture");
    let hidden_report = app
        .oneshot(auth_req(
            Method::POST,
            &format!("/api/v2/reviews/{review_id}/report"),
            json!({ "reason": "hidden review attempt" }),
            &helpers::create_access_token_for(reporter_id),
        ))
        .await
        .expect("hidden review response");
    assert_eq!(hidden_report.status(), StatusCode::NOT_FOUND);
}
