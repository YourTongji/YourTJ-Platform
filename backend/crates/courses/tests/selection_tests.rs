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

async fn get_status(app: axum::Router, uri: &str) -> StatusCode {
    app.oneshot(Request::builder().uri(uri).body(Body::empty()).expect("build selection request"))
        .await
        .expect("send selection request")
        .status()
}

async fn get_bad_request(app: axum::Router, uri: &str) -> Value {
    let response = app
        .oneshot(Request::builder().uri(uri).body(Body::empty()).expect("build selection request"))
        .await
        .expect("send invalid selection request");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body =
        to_bytes(response.into_body(), 1024 * 1024).await.expect("read invalid selection response");
    serde_json::from_slice(&body).expect("parse invalid selection response")
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
    let rows = selection_repo::list_majors(&pool, 1, "2024").await.unwrap();
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
    let rows = selection_repo::list_courses_by_major(&pool, 1, 1, "2024").await.unwrap();
    assert!(!rows.is_empty());
    assert_eq!(rows[0].code, "SEL101");
}

#[tokio::test]
async fn list_courses_by_nature() {
    let pool = pool_or_skip!();
    common::seed_selection_data(&pool).await;
    let rows = selection_repo::list_courses_by_nature(&pool, 1, 1).await.unwrap();
    assert!(!rows.is_empty());
}

#[tokio::test]
async fn finds_selection_course_by_teaching_class_id() {
    let pool = pool_or_skip!();
    common::seed_selection_data(&pool).await;
    let row = selection_repo::find_selection_course_by_id(&pool, 1).await.unwrap();
    assert!(row.is_some());
    assert_eq!(row.unwrap().name, "选课测试课");
}

#[tokio::test]
async fn calendar_scoped_routes_keep_same_code_teaching_classes_separate() {
    let pool = pool_or_skip!();
    let unique_number = (uuid::Uuid::new_v4().as_u128() % 1_000_000_000) as i64 + 2_000_000_000;
    let first_calendar_id = unique_number;
    let second_calendar_id = unique_number + 1;
    let first_major_id = unique_number + 2;
    let second_major_id = unique_number + 3;
    let nature_id = unique_number + 4;
    let first_teaching_class_id = unique_number + 5;
    let second_teaching_class_id = unique_number + 6;
    let grade = unique_number.to_string();
    let shared_code = format!("SAME{unique_number}");

    sqlx::query(
        "INSERT INTO selection.calendars (id, name, is_current) \
         VALUES ($1, '测试学期一', false), ($2, '测试学期二', false)",
    )
    .bind(first_calendar_id)
    .bind(second_calendar_id)
    .execute(&pool)
    .await
    .expect("insert selection calendars");
    sqlx::query(
        "INSERT INTO selection.majors (id, name, grade) \
         VALUES ($1, '测试专业一', $3), ($2, '测试专业二', $3)",
    )
    .bind(first_major_id)
    .bind(second_major_id)
    .bind(&grade)
    .execute(&pool)
    .await
    .expect("insert selection majors");
    sqlx::query("INSERT INTO selection.course_natures (id, name) VALUES ($1, '测试性质')")
        .bind(nature_id)
        .execute(&pool)
        .await
        .expect("insert selection nature");
    sqlx::query(
        "INSERT INTO selection.courses \
         (id, code, name, nature_id, calendar_id, teacher_name, teacher_names) \
         VALUES \
         ($1, $3, '同课号教学班一', $4, $5, '甲老师', ARRAY['甲老师']), \
         ($2, $3, '同课号教学班二', $4, $6, '乙老师', ARRAY['乙老师'])",
    )
    .bind(first_teaching_class_id)
    .bind(second_teaching_class_id)
    .bind(&shared_code)
    .bind(nature_id)
    .bind(first_calendar_id)
    .bind(second_calendar_id)
    .execute(&pool)
    .await
    .expect("insert selection teaching classes");
    sqlx::query(
        "INSERT INTO selection.major_courses (major_id, course_id, grade) \
         VALUES ($1, $3, $5), ($2, $4, $5)",
    )
    .bind(first_major_id)
    .bind(second_major_id)
    .bind(first_teaching_class_id)
    .bind(second_teaching_class_id)
    .bind(&grade)
    .execute(&pool)
    .await
    .expect("insert selection major courses");
    sqlx::query(
        "INSERT INTO selection.timeslots \
         (course_id, teacher_name, weekday, start_slot, end_slot, weeks, location) \
         VALUES ($1, '甲老师', 1, 1, 2, '1-16', 'A101'), \
                ($2, '乙老师', 2, 3, 4, '1-16', 'B202')",
    )
    .bind(first_teaching_class_id)
    .bind(second_teaching_class_id)
    .execute(&pool)
    .await
    .expect("insert selection timeslots");

    let first_majors = selection_repo::list_majors(&pool, first_calendar_id, &grade)
        .await
        .expect("list first-calendar majors");
    let first_major_courses =
        selection_repo::list_courses_by_major(&pool, first_calendar_id, first_major_id, &grade)
            .await
            .expect("list first-calendar major courses");
    let first_nature_courses =
        selection_repo::list_courses_by_nature(&pool, first_calendar_id, nature_id)
            .await
            .expect("list first-calendar nature courses");
    let first_detail = selection_repo::find_selection_course_by_id(&pool, first_teaching_class_id)
        .await
        .expect("find first teaching class")
        .expect("first teaching class exists");
    let first_search_projection = selection_repo::find_selection_courses_by_ids(
        &pool,
        first_calendar_id,
        &[second_teaching_class_id, first_teaching_class_id],
    )
    .await
    .expect("revalidate first-calendar search candidates");

    let app = courses::routes(test_state(pool.clone()));
    let detail =
        get_json(app.clone(), &format!("/api/v2/selection/courses/{first_teaching_class_id}"))
            .await;
    let timeslots = get_json(
        app.clone(),
        &format!("/api/v2/selection/courses/{first_teaching_class_id}/timeslots"),
    )
    .await;
    let missing_calendar_error =
        get_bad_request(app.clone(), &format!("/api/v2/selection/majors?grade={grade}")).await;
    let malformed_calendar_error = get_bad_request(
        app.clone(),
        "/api/v2/selection/courses-by-nature?calendarId=not-a-number&natureId=1",
    )
    .await;
    let missing_search_calendar_error =
        get_bad_request(app.clone(), "/api/v2/selection/courses/search?q=test").await;
    let invalid_id_status = get_status(app, "/api/v2/selection/courses/not-a-teaching-class").await;

    sqlx::query("DELETE FROM selection.timeslots WHERE course_id IN ($1, $2)")
        .bind(first_teaching_class_id)
        .bind(second_teaching_class_id)
        .execute(&pool)
        .await
        .expect("delete selection timeslots");
    sqlx::query("DELETE FROM selection.major_courses WHERE course_id IN ($1, $2)")
        .bind(first_teaching_class_id)
        .bind(second_teaching_class_id)
        .execute(&pool)
        .await
        .expect("delete selection major courses");
    sqlx::query("DELETE FROM selection.courses WHERE id IN ($1, $2)")
        .bind(first_teaching_class_id)
        .bind(second_teaching_class_id)
        .execute(&pool)
        .await
        .expect("delete selection teaching classes");
    sqlx::query("DELETE FROM selection.majors WHERE id IN ($1, $2)")
        .bind(first_major_id)
        .bind(second_major_id)
        .execute(&pool)
        .await
        .expect("delete selection majors");
    sqlx::query("DELETE FROM selection.course_natures WHERE id = $1")
        .bind(nature_id)
        .execute(&pool)
        .await
        .expect("delete selection nature");
    sqlx::query("DELETE FROM selection.calendars WHERE id IN ($1, $2)")
        .bind(first_calendar_id)
        .bind(second_calendar_id)
        .execute(&pool)
        .await
        .expect("delete selection calendars");

    assert_eq!(first_majors.len(), 1);
    assert_eq!(first_majors[0].id, first_major_id);
    assert_eq!(first_major_courses.len(), 1);
    assert_eq!(first_major_courses[0].id, first_teaching_class_id);
    assert_eq!(first_nature_courses.len(), 1);
    assert_eq!(first_nature_courses[0].id, first_teaching_class_id);
    assert_eq!(first_detail.teacher_name.as_deref(), Some("甲老师"));
    assert_eq!(first_search_projection.len(), 1);
    assert_eq!(first_search_projection[0].id, first_teaching_class_id);
    assert_eq!(detail["id"], first_teaching_class_id.to_string());
    assert_eq!(detail["code"], shared_code);
    assert_eq!(detail["calendarId"], first_calendar_id.to_string());
    assert_eq!(timeslots[0]["courseId"], first_teaching_class_id.to_string());
    assert_eq!(timeslots[0]["weekday"], 1);
    let invalid_query_error =
        json!({ "error": { "code": "BAD_REQUEST", "message": "invalid selection query" } });
    assert_eq!(missing_calendar_error, invalid_query_error);
    assert_eq!(malformed_calendar_error, invalid_query_error);
    assert_eq!(missing_search_calendar_error, invalid_query_error);
    assert_eq!(invalid_id_status, StatusCode::BAD_REQUEST);
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
    common::seed_selection_data(&pool).await;
    let unique_number = (uuid::Uuid::new_v4().as_u128() % 1_000_000_000) as i64 + 10_000;
    let course_code = format!("WIRE{unique_number}");
    let fetch_source = format!("wire-contract-{unique_number}");
    sqlx::query(
        "INSERT INTO selection.courses \
         (id, code, name, credit, nature_id, calendar_id, campus_id, teacher_name, teacher_names) \
         VALUES ($1, $2, '契约测试课', NULL, NULL, 1, NULL, NULL, NULL)",
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
    let course = get_json(app.clone(), &format!("/api/v2/selection/courses/{unique_number}")).await;
    let timeslots =
        get_json(app.clone(), &format!("/api/v2/selection/courses/{unique_number}/timeslots"))
            .await;
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
            "calendarId": "1",
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
