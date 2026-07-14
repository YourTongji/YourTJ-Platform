//! Integration tests for the selection (选课) domain — exercises repo functions
//! directly. These tests require `DATABASE_URL` to be set. When it is not, every
//! test is skipped.

mod common;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use courses::selection_repo;
use serde_json::{json, Value};
use shared::AppState;
use sqlx::PgPool;
use tower::ServiceExt;

macro_rules! pool_or_skip {
    () => {
        match common::try_connect().await {
            Some(p) => p,
            None => return,
        }
    };
}

fn test_state(pool: PgPool) -> AppState {
    AppState {
        db: pool,
        config: shared::Config::from_env().expect("load selection test config"),
        jwt_secret: "selection-integration-test-secret".into(),
        jwt_ttl: 900,
        refresh_ttl: 604_800,
        meili_url: String::new(),
        meili_master_key: String::new(),
        redis: None,
        system_private_key: vec![0; 32],
        system_public_key_b64: String::new(),
        email_encryption: None,
        captcha_verifier: None,
        sse_tx: None,
    }
}

async fn get_json(app: axum::Router, uri: &str) -> Value {
    let response = app
        .oneshot(Request::builder().uri(uri).body(Body::empty()).expect("build selection request"))
        .await
        .expect("send selection request");
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 1024 * 1024).await.expect("read selection response");
    serde_json::from_slice(&body).expect("parse selection response")
}

#[tokio::test]
async fn list_calendars() {
    let pool = pool_or_skip!();
    common::seed_selection_data(&pool).await;
    let rows = selection_repo::list_calendars(&pool).await.unwrap();
    assert!(!rows.is_empty());
    // Current should be first
}

#[tokio::test]
async fn list_campuses() {
    let pool = pool_or_skip!();
    common::seed_selection_data(&pool).await;
    let rows = selection_repo::list_campuses(&pool).await.unwrap();
    assert!(rows.len() >= 2);
}

#[tokio::test]
async fn list_faculties() {
    let pool = pool_or_skip!();
    common::seed_selection_data(&pool).await;
    let rows = selection_repo::list_faculties(&pool).await.unwrap();
    assert!(!rows.is_empty());
}

#[tokio::test]
async fn list_grades() {
    let pool = pool_or_skip!();
    common::seed_selection_data(&pool).await;
    let grades = selection_repo::list_grades(&pool, 1).await.unwrap();
    assert!(grades.contains(&"2024".to_string()));
}

#[tokio::test]
async fn list_majors() {
    let pool = pool_or_skip!();
    common::seed_selection_data(&pool).await;
    let rows = selection_repo::list_majors(&pool, "2024").await.unwrap();
    assert!(!rows.is_empty());
}

#[tokio::test]
async fn list_course_natures() {
    let pool = pool_or_skip!();
    common::seed_selection_data(&pool).await;
    let rows = selection_repo::list_course_natures(&pool).await.unwrap();
    assert!(rows.len() >= 2);
}

#[tokio::test]
async fn list_courses_by_major() {
    let pool = pool_or_skip!();
    common::seed_selection_data(&pool).await;
    let rows = selection_repo::list_courses_by_major(&pool, 1, "2024").await.unwrap();
    assert!(!rows.is_empty());
    assert_eq!(rows[0].code, "SEL101");
}

#[tokio::test]
async fn list_courses_by_nature() {
    let pool = pool_or_skip!();
    common::seed_selection_data(&pool).await;
    let rows = selection_repo::list_courses_by_nature(&pool, 1).await.unwrap();
    assert!(!rows.is_empty());
}

#[tokio::test]
async fn find_selection_course_by_code() {
    let pool = pool_or_skip!();
    common::seed_selection_data(&pool).await;
    let row = selection_repo::find_selection_course_by_code(&pool, "SEL101").await.unwrap();
    assert!(row.is_some());
    assert_eq!(row.unwrap().name, "选课测试课");
}

// search_selection_courses was moved from DB ILIKE to Meilisearch
// (courses::meili::search_selection_courses). Integration tests for
// the Meilisearch path require a running Meilisearch instance and are
// covered by end-to-end testing.

#[tokio::test]
async fn list_timeslots() {
    let pool = pool_or_skip!();
    common::seed_selection_data(&pool).await;
    let rows = selection_repo::list_timeslots(&pool, 1).await.unwrap();
    assert!(!rows.is_empty());
    assert_eq!(rows[0].weekday, 1);
}

#[tokio::test]
async fn find_latest_update_none_when_empty() {
    let pool = pool_or_skip!();
    // Don't seed fetchlog — should return None
    let result = selection_repo::find_latest_update(&pool).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn handlers_return_the_selection_wire_contract() {
    let pool = pool_or_skip!();
    let unique_number = (uuid::Uuid::new_v4().as_u128() % 1_000_000_000) as i64 + 10_000;
    let course_code = format!("WIRE{unique_number}");
    let fetch_source = format!("wire-contract-{unique_number}");
    sqlx::query(
        "INSERT INTO selection.courses \
         (id, code, name, credit, nature_id, calendar_id, campus_id, teacher_name, teacher_names) \
         VALUES ($1, $2, '契约测试课', NULL, NULL, NULL, NULL, NULL, NULL)",
    )
    .bind(unique_number)
    .bind(&course_code)
    .execute(&pool)
    .await
    .expect("insert selection contract course");
    sqlx::query(
        "INSERT INTO selection.timeslots \
         (course_id, teacher_name, weekday, start_slot, end_slot, weeks, location) \
         VALUES ($1, NULL, 2, 5, 6, NULL, NULL)",
    )
    .bind(unique_number)
    .execute(&pool)
    .await
    .expect("insert selection contract timeslot");
    sqlx::query(
        "INSERT INTO selection.fetchlog (source, fetched_at) \
         VALUES ($1, TIMESTAMPTZ '9999-12-31 23:59:59+00')",
    )
    .bind(&fetch_source)
    .execute(&pool)
    .await
    .expect("insert selection contract fetchlog");

    let app = courses::routes(test_state(pool.clone()));
    let course = get_json(app.clone(), &format!("/api/v2/selection/courses/{course_code}")).await;
    let timeslots =
        get_json(app.clone(), &format!("/api/v2/selection/courses/{course_code}/timeslots")).await;
    let latest_update = get_json(app, "/api/v2/selection/latest-update").await;

    sqlx::query("DELETE FROM selection.timeslots WHERE course_id = $1")
        .bind(unique_number)
        .execute(&pool)
        .await
        .expect("delete selection contract timeslot");
    sqlx::query("DELETE FROM selection.courses WHERE id = $1")
        .bind(unique_number)
        .execute(&pool)
        .await
        .expect("delete selection contract course");
    sqlx::query("DELETE FROM selection.fetchlog WHERE source = $1")
        .bind(&fetch_source)
        .execute(&pool)
        .await
        .expect("delete selection contract fetchlog");

    assert_eq!(
        course,
        json!({
            "id": unique_number.to_string(),
            "code": course_code,
            "name": "契约测试课",
            "credit": null,
            "natureId": null,
            "campusId": null,
            "teacherName": null,
            "teacherNames": []
        })
    );
    assert_eq!(
        timeslots,
        json!([{
            "courseId": unique_number.to_string(),
            "teacherName": null,
            "weekday": 2,
            "startSlot": 5,
            "endSlot": 6,
            "weeks": null,
            "location": null
        }])
    );
    let updated_at = latest_update["updatedAt"].as_str().expect("latest update timestamp");
    chrono::DateTime::parse_from_rfc3339(updated_at).expect("latest update uses RFC 3339");
    assert_eq!(latest_update.as_object().expect("latest update object").len(), 1);
}
