//! Integrity coverage for review administration and moderation evidence.

#[path = "helpers/mod.rs"]
mod helpers;

use axum::http::{Method, StatusCode};
use helpers::{auth_req, create_test_app, read_json, seed_account, seed_course};
use serde_json::{json, Value};
use tower::ServiceExt;

async fn seed_admin(pool: &sqlx::PgPool, suffix: &str) -> i64 {
    let admin_id = seed_account(
        pool,
        &format!("review-admin-{suffix}@tongji.edu.cn"),
        &format!("review-admin-{suffix}"),
    )
    .await;
    sqlx::query("UPDATE identity.accounts SET role = 'admin' WHERE id = $1")
        .bind(admin_id)
        .execute(pool)
        .await
        .expect("promote review admin");
    admin_id
}

#[tokio::test]
async fn admin_review_status_all_skips_enum_cast_and_filters_are_validated() {
    let (pool, app) = create_test_app().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let admin_id = seed_admin(&pool, &suffix).await;
    let author_id = seed_account(
        &pool,
        &format!("review-filter-author-{suffix}@tongji.edu.cn"),
        &format!("review-filter-author-{suffix}"),
    )
    .await;
    let course_id = seed_course(&pool, &format!("FILTER-{suffix}"), "Filter course").await;
    for status in ["visible", "hidden", "pending"] {
        sqlx::query(
            "INSERT INTO reviews.reviews (course_id, account_id, rating, status) \
             VALUES ($1, $2, 4, $3::reviews.review_status)",
        )
        .bind(course_id)
        .bind(author_id)
        .bind(status)
        .execute(&pool)
        .await
        .expect("seed filtered review");
    }
    let token = helpers::create_access_token_for(admin_id);

    let all = app
        .clone()
        .oneshot(auth_req(
            Method::GET,
            "/api/v2/admin/reviews?status=all&limit=10",
            json!({}),
            &token,
        ))
        .await
        .expect("list all reviews");
    assert_eq!(all.status(), StatusCode::OK);
    let body: Value = read_json(all).await;
    assert_eq!(body["items"].as_array().expect("review items").len(), 3);

    let invalid_status = app
        .clone()
        .oneshot(auth_req(Method::GET, "/api/v2/admin/reviews?status=deleted", json!({}), &token))
        .await
        .expect("invalid review status");
    assert_eq!(invalid_status.status(), StatusCode::BAD_REQUEST);
    let invalid_limit = app
        .oneshot(auth_req(Method::GET, "/api/v2/admin/reviews?limit=0", json!({}), &token))
        .await
        .expect("invalid review limit");
    assert_eq!(invalid_limit.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn staff_review_patch_is_absent_and_author_non_visible_edit_preserves_aggregate() {
    let (pool, app) = create_test_app().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let admin_id = seed_admin(&pool, &suffix).await;
    let author_id = seed_account(
        &pool,
        &format!("review-edit-author-{suffix}@tongji.edu.cn"),
        &format!("review-edit-author-{suffix}"),
    )
    .await;
    let course_id = seed_course(&pool, &format!("EDIT-{suffix}"), "Edit course").await;
    sqlx::query("UPDATE courses.courses SET review_count = 0, review_avg = 0 WHERE id = $1")
        .bind(course_id)
        .execute(&pool)
        .await
        .expect("reset edit aggregate");
    let review_ids: Vec<i64> = sqlx::query_scalar(
        "INSERT INTO reviews.reviews (course_id, account_id, rating, status) \
         VALUES ($1, $2, 1, 'hidden'), ($1, $2, 2, 'pending') RETURNING id",
    )
    .bind(course_id)
    .bind(author_id)
    .fetch_all(&pool)
    .await
    .expect("seed non-visible reviews");
    let token = helpers::create_access_token_for(admin_id);

    for review_id in &review_ids {
        let response = app
            .clone()
            .oneshot(auth_req(
                Method::PATCH,
                &format!("/api/v2/admin/reviews/{review_id}"),
                json!({
                    "rating": 5,
                    "comment": "staff-corrected metadata",
                    "reason": "correcting imported review metadata"
                }),
                &token,
            ))
            .await
            .expect("edit non-visible review");
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }
    let owner_edit = app
        .oneshot(auth_req(
            Method::PATCH,
            &format!("/api/v2/reviews/{}", review_ids[0]),
            json!({ "rating": 4, "comment": "author clarification" }),
            &helpers::create_access_token_for(author_id),
        ))
        .await
        .expect("author edits hidden review");
    assert_eq!(owner_edit.status(), StatusCode::OK);

    let aggregate: (i32, f64) =
        sqlx::query_as("SELECT review_count, review_avg FROM courses.courses WHERE id = $1")
            .bind(course_id)
            .fetch_one(&pool)
            .await
            .expect("course aggregate after non-visible edits");
    assert_eq!(aggregate, (0, 0.0));
    let ratings: Vec<i32> =
        sqlx::query_scalar("SELECT rating FROM reviews.reviews WHERE id = ANY($1) ORDER BY id")
            .bind(&review_ids)
            .fetch_all(&pool)
            .await
            .expect("edited ratings");
    assert_eq!(ratings, vec![4, 2]);
}

#[tokio::test]
async fn report_queue_includes_bounded_hidden_review_evidence_and_trims_reason() {
    let (pool, app) = create_test_app().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let admin_id = seed_admin(&pool, &suffix).await;
    let author_handle = format!("evidence-author-{suffix}");
    let author_id =
        seed_account(&pool, &format!("evidence-author-{suffix}@tongji.edu.cn"), &author_handle)
            .await;
    let reporter_id = seed_account(
        &pool,
        &format!("evidence-reporter-{suffix}@tongji.edu.cn"),
        &format!("evidence-reporter-{suffix}"),
    )
    .await;
    let course_id = seed_course(&pool, &format!("EVIDENCE-{suffix}"), "Evidence course").await;
    let long_comment = "证".repeat(1200);
    let review_id: i64 = sqlx::query_scalar(
        "INSERT INTO reviews.reviews (course_id, account_id, rating, comment, status) \
         VALUES ($1, $2, 2, $3, 'visible') RETURNING id",
    )
    .bind(course_id)
    .bind(author_id)
    .bind(&long_comment)
    .fetch_one(&pool)
    .await
    .expect("seed evidence review");
    let reporter_token = helpers::create_access_token_for(reporter_id);

    let invalid = app
        .clone()
        .oneshot(auth_req(
            Method::POST,
            &format!("/api/v2/reviews/{review_id}/report"),
            json!({ "reason": "  " }),
            &reporter_token,
        ))
        .await
        .expect("invalid report reason");
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);

    let reported = app
        .clone()
        .oneshot(auth_req(
            Method::POST,
            &format!("/api/v2/reviews/{review_id}/report"),
            json!({ "reason": "  policy violation  " }),
            &reporter_token,
        ))
        .await
        .expect("report hidden review");
    assert_eq!(reported.status(), StatusCode::NO_CONTENT);
    sqlx::query("UPDATE reviews.reviews SET status = 'hidden' WHERE id = $1")
        .bind(review_id)
        .execute(&pool)
        .await
        .expect("hide reported review before queue read");
    let persisted_reason: String =
        sqlx::query_scalar("SELECT reason FROM reviews.review_reports WHERE review_id = $1")
            .bind(review_id)
            .fetch_one(&pool)
            .await
            .expect("persisted report reason");
    assert_eq!(persisted_reason, "policy violation");

    let admin_token = helpers::create_access_token_for(admin_id);
    let queue = app
        .oneshot(auth_req(
            Method::GET,
            "/api/v2/admin/reports?status=open&limit=10",
            json!({}),
            &admin_token,
        ))
        .await
        .expect("review report queue");
    assert_eq!(queue.status(), StatusCode::OK);
    let body: Value = read_json(queue).await;
    let report = &body["items"][0];
    assert_eq!(report["reviewId"], review_id.to_string());
    assert_eq!(report["courseId"], course_id.to_string());
    assert_eq!(report["reviewAuthorHandle"], author_handle);
    assert_eq!(report["reviewRating"], 2);
    assert_eq!(report["reviewStatus"], "hidden");
    assert_eq!(report["reviewExcerpt"].as_str().expect("review excerpt").chars().count(), 1000);
}

#[tokio::test]
async fn hidden_review_history_prevents_course_cascade_deletion() {
    let (pool, _app) = create_test_app().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let author_id = seed_account(
        &pool,
        &format!("course-guard-author-{suffix}@tongji.edu.cn"),
        &format!("course-guard-author-{suffix}"),
    )
    .await;
    let course_id = seed_course(&pool, &format!("GUARD-{suffix}"), "Guarded course").await;
    sqlx::query(
        "INSERT INTO reviews.reviews (course_id, account_id, rating, status) \
         VALUES ($1, $2, 3, 'hidden')",
    )
    .bind(course_id)
    .bind(author_id)
    .execute(&pool)
    .await
    .expect("seed hidden review history");
    sqlx::query("UPDATE courses.courses SET review_count = 0 WHERE id = $1")
        .bind(course_id)
        .execute(&pool)
        .await
        .expect("simulate stale visible aggregate");

    let deletion = sqlx::query("DELETE FROM courses.courses WHERE id = $1")
        .bind(course_id)
        .execute(&pool)
        .await;
    let error = deletion.expect_err("review FK must reject course deletion");
    let sqlx::Error::Database(database_error) = error else {
        panic!("expected database constraint error");
    };
    assert_eq!(database_error.code().as_deref(), Some("23503"));
    assert_eq!(database_error.constraint(), Some("reviews_course_id_fkey"));
    let course_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM courses.courses WHERE id = $1)")
            .bind(course_id)
            .fetch_one(&pool)
            .await
            .expect("guarded course remains");
    assert!(course_exists);
}

#[tokio::test]
async fn review_moderation_enforces_content_author_role_hierarchy() {
    let (pool, app) = create_test_app().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let admin_id = seed_admin(&pool, &suffix).await;
    let moderator_id = seed_account(
        &pool,
        &format!("hierarchy-mod-actor-{suffix}@tongji.edu.cn"),
        &format!("hierarchy-mod-actor-{suffix}"),
    )
    .await;
    let moderator_author_id = seed_account(
        &pool,
        &format!("hierarchy-mod-author-{suffix}@tongji.edu.cn"),
        &format!("hierarchy-mod-author-{suffix}"),
    )
    .await;
    let admin_author_id = seed_account(
        &pool,
        &format!("hierarchy-admin-author-{suffix}@tongji.edu.cn"),
        &format!("hierarchy-admin-author-{suffix}"),
    )
    .await;
    let user_author_id = seed_account(
        &pool,
        &format!("hierarchy-user-author-{suffix}@tongji.edu.cn"),
        &format!("hierarchy-user-author-{suffix}"),
    )
    .await;
    let reporter_id = seed_account(
        &pool,
        &format!("hierarchy-reporter-{suffix}@tongji.edu.cn"),
        &format!("hierarchy-reporter-{suffix}"),
    )
    .await;
    sqlx::query("UPDATE identity.accounts SET role = 'mod' WHERE id = ANY($1)")
        .bind(vec![moderator_id, moderator_author_id])
        .execute(&pool)
        .await
        .expect("promote hierarchy moderators");
    sqlx::query("UPDATE identity.accounts SET role = 'admin' WHERE id = $1")
        .bind(admin_author_id)
        .execute(&pool)
        .await
        .expect("promote hierarchy admin author");
    let course_id = seed_course(&pool, &format!("HIERARCHY-{suffix}"), "Hierarchy course").await;
    let moderator_review_id: i64 = sqlx::query_scalar(
        "INSERT INTO reviews.reviews (course_id, account_id, rating) \
         VALUES ($1, $2, 3) RETURNING id",
    )
    .bind(course_id)
    .bind(moderator_author_id)
    .fetch_one(&pool)
    .await
    .expect("seed moderator review");
    let admin_review_id: i64 = sqlx::query_scalar(
        "INSERT INTO reviews.reviews (course_id, account_id, rating) \
         VALUES ($1, $2, 3) RETURNING id",
    )
    .bind(course_id)
    .bind(admin_author_id)
    .fetch_one(&pool)
    .await
    .expect("seed admin review");
    let user_review_id: i64 = sqlx::query_scalar(
        "INSERT INTO reviews.reviews (course_id, account_id, rating) \
         VALUES ($1, $2, 3) RETURNING id",
    )
    .bind(course_id)
    .bind(user_author_id)
    .fetch_one(&pool)
    .await
    .expect("seed user review");
    sqlx::query("UPDATE courses.courses SET review_count = 3, review_avg = 3 WHERE id = $1")
        .bind(course_id)
        .execute(&pool)
        .await
        .expect("seed hierarchy aggregate");
    let moderator_token = helpers::create_access_token_for(moderator_id);
    let admin_token = helpers::create_access_token_for(admin_id);

    let mod_on_mod = app
        .clone()
        .oneshot(auth_req(
            Method::DELETE,
            &format!("/api/v2/admin/reviews/{moderator_review_id}"),
            json!({ "reason": "attempt equal-role moderation" }),
            &moderator_token,
        ))
        .await
        .expect("moderator on moderator response");
    assert_eq!(mod_on_mod.status(), StatusCode::FORBIDDEN);
    let mod_on_admin = app
        .clone()
        .oneshot(auth_req(
            Method::POST,
            &format!("/api/v2/admin/reviews/{admin_review_id}/toggle"),
            json!({ "reason": "attempt higher-role moderation" }),
            &moderator_token,
        ))
        .await
        .expect("moderator on admin response");
    assert_eq!(mod_on_admin.status(), StatusCode::FORBIDDEN);
    let admin_on_admin = app
        .clone()
        .oneshot(auth_req(
            Method::DELETE,
            &format!("/api/v2/admin/reviews/{admin_review_id}"),
            json!({ "reason": "attempt admin peer moderation" }),
            &admin_token,
        ))
        .await
        .expect("admin on admin response");
    assert_eq!(admin_on_admin.status(), StatusCode::FORBIDDEN);
    let mod_on_user = app
        .clone()
        .oneshot(auth_req(
            Method::DELETE,
            &format!("/api/v2/admin/reviews/{user_review_id}"),
            json!({ "reason": "valid user-content moderation" }),
            &moderator_token,
        ))
        .await
        .expect("moderator on user response");
    assert_eq!(mod_on_user.status(), StatusCode::NO_CONTENT);

    let moderator_report_id: i64 = sqlx::query_scalar(
        "INSERT INTO reviews.review_reports (review_id, reporter_account_id, reason) \
         VALUES ($1, $2, 'reported moderator review') RETURNING id",
    )
    .bind(moderator_review_id)
    .bind(reporter_id)
    .fetch_one(&pool)
    .await
    .expect("seed moderator review report");
    let admin_report_id: i64 = sqlx::query_scalar(
        "INSERT INTO reviews.review_reports (review_id, reporter_account_id, reason) \
         VALUES ($1, $2, 'reported admin review') RETURNING id",
    )
    .bind(admin_review_id)
    .bind(reporter_id)
    .fetch_one(&pool)
    .await
    .expect("seed admin review report");
    let mod_resolve_mod = app
        .clone()
        .oneshot(auth_req(
            Method::POST,
            &format!("/api/v2/admin/reports/{moderator_report_id}/resolve"),
            json!({ "action": "reject", "note": "attempt equal-role report decision" }),
            &moderator_token,
        ))
        .await
        .expect("moderator report hierarchy response");
    assert_eq!(mod_resolve_mod.status(), StatusCode::FORBIDDEN);
    let admin_resolve_admin = app
        .clone()
        .oneshot(auth_req(
            Method::POST,
            &format!("/api/v2/admin/reports/{admin_report_id}/resolve"),
            json!({ "action": "reject", "note": "attempt admin peer report decision" }),
            &admin_token,
        ))
        .await
        .expect("admin report hierarchy response");
    assert_eq!(admin_resolve_admin.status(), StatusCode::FORBIDDEN);
    let admin_resolve_mod = app
        .oneshot(auth_req(
            Method::POST,
            &format!("/api/v2/admin/reports/{moderator_report_id}/resolve"),
            json!({ "action": "reject", "note": "valid lower-role report decision" }),
            &admin_token,
        ))
        .await
        .expect("admin resolves moderator report");
    assert_eq!(admin_resolve_mod.status(), StatusCode::OK);
}
