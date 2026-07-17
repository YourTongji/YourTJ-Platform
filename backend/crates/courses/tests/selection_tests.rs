//! Integration tests for the selection (选课) domain — exercises repo functions
//! directly. These tests require `DATABASE_URL` to be set. When it is not, every
//! test is skipped.

mod common;

use std::str::FromStr;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use courses::{selection_repo, sync};
use serde_json::{json, Value};
use shared::AppState;
use sqlx::migrate::Migrator;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{Connection, PgConnection, PgPool};
use tower::ServiceExt;

static MIGRATOR: Migrator = sqlx::migrate!("../../migrations");

async fn isolated_database_pool(prefix: &str) -> Option<PgPool> {
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        return None;
    };
    let base_options = PgConnectOptions::from_str(&database_url).expect("parse selection DB URL");
    let mut admin = PgConnection::connect_with(&base_options.clone().database("postgres"))
        .await
        .expect("connect selection database administrator");
    let database_name = format!("{prefix}_{}_test", uuid::Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE DATABASE \"{database_name}\""))
        .execute(&mut admin)
        .await
        .expect("create isolated selection database");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(base_options.database(&database_name))
        .await
        .expect("connect isolated selection database");
    MIGRATOR.run(&pool).await.expect("apply isolated selection migrations");
    Some(pool)
}

async fn record_validated_import_run(pool: &PgPool, snapshot_sha256: String) -> (i64, Value) {
    let counts: Value = sqlx::query_scalar(
        "SELECT jsonb_build_object(\
           'calendar', (SELECT COUNT(*) FROM selection.pk_calendars), \
           'language', (SELECT COUNT(*) FROM selection.pk_languages), \
           'coursenature', (SELECT COUNT(*) FROM selection.pk_course_natures), \
           'coursenature_by_calendar', (\
             SELECT COUNT(*) FROM selection.pk_course_natures_by_calendar\
           ), \
           'assessment', (SELECT COUNT(*) FROM selection.pk_assessments), \
           'campus', (SELECT COUNT(*) FROM selection.pk_campuses), \
           'faculty', (SELECT COUNT(*) FROM selection.pk_faculties), \
           'major', (SELECT COUNT(*) FROM selection.pk_majors), \
           'coursedetail', (SELECT COUNT(*) FROM selection.pk_course_details), \
           'teacher', (SELECT COUNT(*) FROM selection.pk_teachers_raw), \
           'teacher_timeslots', (SELECT COUNT(*) FROM selection.pk_teacher_timeslots), \
           'majorandcourse', (SELECT COUNT(*) FROM selection.pk_major_courses), \
           'fetchlog', (SELECT COUNT(*) FROM selection.pk_fetch_logs)\
         )",
    )
    .fetch_one(pool)
    .await
    .expect("count raw selection import fixture");
    let validation = json!({
        "rowCountsMatched": true,
        "sourceSchemaValidated": true,
        "completenessApproved": true,
        "approvalMode": "unbaselined",
        "approvalReason": "Reviewed isolated integration-test snapshot",
        "approvedCoreCounts": {
            "calendar": counts["calendar"],
            "coursenature": counts["coursenature"],
            "coursenature_by_calendar": counts["coursenature_by_calendar"],
            "campus": counts["campus"],
            "faculty": counts["faculty"],
            "major": counts["major"],
            "coursedetail": counts["coursedetail"],
            "teacher": counts["teacher"],
            "teacher_timeslots": counts["teacher_timeslots"],
            "majorandcourse": counts["majorandcourse"],
            "fetchlog": counts["fetchlog"],
        }
    });
    let import_run_id: i64 = sqlx::query_scalar(
        "INSERT INTO selection.import_runs (\
           snapshot_sha256, snapshot_bytes, source_database, imported_by, \
           source_table_counts, target_table_counts, validation\
         ) VALUES ($1, 1, 'jcourse-db-backup', 'selection-test', $2, $2, $3) \
         RETURNING id",
    )
    .bind(&snapshot_sha256)
    .bind(counts)
    .bind(&validation)
    .fetch_one(pool)
    .await
    .expect("record validated selection import fixture");
    let legacy_counts: Value = sqlx::query_scalar(
        "SELECT jsonb_build_object(\
           'teachers', (SELECT COUNT(*) FROM courses.pk_legacy_teachers), \
           'courses', (SELECT COUNT(*) FROM courses.pk_legacy_courses), \
           'course_aliases', (SELECT COUNT(*) FROM courses.pk_legacy_course_aliases)\
         )",
    )
    .fetch_one(pool)
    .await
    .expect("count raw legacy course import fixture");
    let mut legacy_validation = validation.clone();
    legacy_validation["legacyCourseApprovalMode"] = json!("unbaselined");
    legacy_validation["approvedLegacyCourseCounts"] = legacy_counts.clone();
    sqlx::query(
        "INSERT INTO courses.legacy_import_runs (\
           snapshot_sha256, source_database, source_table_counts, target_table_counts, validation\
         ) VALUES ($1, 'jcourse-db-backup', $2, $2, $3)",
    )
    .bind(&snapshot_sha256)
    .bind(legacy_counts)
    .bind(&legacy_validation)
    .execute(pool)
    .await
    .expect("record validated legacy course import fixture");
    (import_run_id, validation)
}

async fn assert_materializer_rejects_source(pool: &PgPool, script: &str, expected_error: &str) {
    let mut connection = pool.acquire().await.expect("acquire materializer connection");
    let error = sqlx::raw_sql(script)
        .execute(&mut *connection)
        .await
        .expect_err("unvalidated source must stop materialization");
    assert!(error.to_string().contains(expected_error), "unexpected materializer failure: {error}");
    sqlx::query("ROLLBACK")
        .execute(&mut *connection)
        .await
        .expect("rollback rejected materialization");
}

async fn enqueue_committed(
    pool: PgPool,
    account_id: i64,
    key_hash: String,
    fingerprint: String,
) -> shared::AppResult<(uuid::Uuid, bool)> {
    let mut tx = pool.begin().await?;
    let (job, created) = sync::enqueue_sync_job_tx(
        &mut tx,
        account_id,
        "selection sync integration test",
        &key_hash,
        &fingerprint,
    )
    .await?;
    tx.commit().await?;
    Ok((job.id, created))
}

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
    let rows = selection_repo::list_course_natures(&pool, 1).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, 1);
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
         (course_id, teacher_name, weekday, start_slot, end_slot, weeks, \
          week_numbers, weeks_unknown, location, location_unknown) \
         VALUES ($1, '甲老师', 1, 1, 2, '1-16', \
                 ARRAY(SELECT generate_series(1, 16)), false, 'A101', false), \
                ($2, '乙老师', 2, 3, 4, '1-16', \
                 ARRAY(SELECT generate_series(1, 16)), false, 'B202', false)",
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
    let scoped_majors = get_json(
        app.clone(),
        &format!("/api/v2/selection/majors?calendarId={first_calendar_id}&grade={grade}"),
    )
    .await;
    let scoped_offerings = get_json(
        app.clone(),
        &format!(
            "/api/v2/selection/offerings?calendarId={first_calendar_id}&courseCode={shared_code}"
        ),
    )
    .await;
    let missing_calendar_error =
        get_bad_request(app.clone(), &format!("/api/v2/selection/majors?grade={grade}")).await;
    let missing_natures_calendar_error =
        get_bad_request(app.clone(), "/api/v2/selection/course-natures").await;
    let missing_offerings_calendar_error = get_bad_request(
        app.clone(),
        &format!("/api/v2/selection/offerings?courseCode={shared_code}"),
    )
    .await;
    let malformed_calendar_error = get_bad_request(
        app.clone(),
        "/api/v2/selection/courses-by-nature?calendarId=not-a-number&natureId=1",
    )
    .await;
    let missing_search_calendar_error =
        get_bad_request(app.clone(), "/api/v2/selection/courses/search?q=test").await;
    let invalid_offering_id_error =
        get_bad_request(app.clone(), "/api/v2/selection/offerings/not-a-teaching-class").await;
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
    assert_eq!(scoped_majors.as_array().expect("scoped majors").len(), 1);
    assert_eq!(scoped_majors[0]["id"], first_major_id.to_string());
    assert_eq!(scoped_offerings["items"].as_array().expect("scoped offerings").len(), 1);
    assert_eq!(scoped_offerings["items"][0]["offeringId"], first_teaching_class_id.to_string());
    let invalid_query_error =
        json!({ "error": { "code": "BAD_REQUEST", "message": "invalid selection query" } });
    assert_eq!(missing_calendar_error, invalid_query_error);
    assert_eq!(missing_natures_calendar_error, invalid_query_error);
    assert_eq!(missing_offerings_calendar_error, invalid_query_error);
    assert_eq!(malformed_calendar_error, invalid_query_error);
    assert_eq!(missing_search_calendar_error, invalid_query_error);
    assert_eq!(
        invalid_offering_id_error,
        json!({ "error": { "code": "BAD_REQUEST", "message": "invalid offeringId" } })
    );
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
    // Don't seed either freshness clock — both should be explicitly absent.
    let result = selection_repo::find_latest_update(&pool).await.unwrap();
    assert!(result.updated_at.is_none());
    assert!(result.imported_at.is_none());
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
            "offeringId": unique_number.to_string(),
            "code": course_code,
            "teachingClassCode": null,
            "name": "契约测试课",
            "credit": null,
            "natureId": null,
            "calendarId": "1",
            "campusId": null,
            "facultyName": null,
            "teachingLanguage": null,
            "teacherName": null,
            "teacherNames": [],
            "startWeek": null,
            "endWeek": null,
            "weeksUnknown": true,
            "scheduleUnknown": true,
            "status": "unknown",
            "catalogueCourseId": null,
            "reviewCount": 0,
            "reviewAvg": null,
            "reviewScope": "none"
        })
    );
    assert_eq!(
        timeslots,
        json!([{
            "offeringId": unique_number.to_string(),
            "courseId": unique_number.to_string(),
            "teacherName": null,
            "weekday": 2,
            "startSlot": 5,
            "endSlot": 6,
            "weeks": null,
            "weekNumbers": [],
            "weeksUnknown": true,
            "location": null,
            "locationUnknown": true
        }])
    );
    let updated_at = latest_update["updatedAt"].as_str().expect("latest update timestamp");
    chrono::DateTime::parse_from_rfc3339(updated_at).expect("latest update uses RFC 3339");
    assert_eq!(latest_update["stale"], false);
    assert_eq!(latest_update["staleAfterHours"], 168);
    assert!(latest_update.get("importedAt").is_some());
    assert_eq!(latest_update.as_object().expect("latest update object").len(), 4);
}

#[tokio::test]
async fn canonical_offerings_filter_paginate_and_bind_the_cursor() {
    let pool = pool_or_skip!();
    common::seed_selection_data(&pool).await;
    let first_id = (uuid::Uuid::new_v4().as_u128() % 800_000_000) as i64 + 1_000_000_000;
    let second_id = first_id + 1;
    let unknown_id = first_id + 2;
    let mixed_id = first_id + 3;
    let unknown_week_id = first_id + 4;
    let course_code = format!("OFFER{first_id}");

    sqlx::query(
        "INSERT INTO selection.courses (\
           id, code, teaching_class_code, name, calendar_id, teacher_names, \
           start_week, end_week, weeks_unknown, schedule_unknown) \
         VALUES \
           ($1, $4, 'OFFER.01', '教学班一', 1, ARRAY['甲老师'], 1, 3, false, false), \
           ($2, $4, 'OFFER.02', '教学班二', 1, ARRAY['乙老师'], 1, 3, false, false), \
           ($3, $4, 'OFFER.03', '时段未知教学班', 1, ARRAY['丙老师'], NULL, NULL, true, true), \
           ($5, $4, 'OFFER.04', '部分未知教学班', 1, ARRAY['丁老师'], 1, 3, false, true), \
           ($6, $4, 'OFFER.05', '周次未知教学班', 1, ARRAY['戊老师'], NULL, NULL, true, false)",
    )
    .bind(first_id)
    .bind(second_id)
    .bind(unknown_id)
    .bind(&course_code)
    .bind(mixed_id)
    .bind(unknown_week_id)
    .execute(&pool)
    .await
    .expect("insert canonical offering fixtures");
    sqlx::query(
        "INSERT INTO selection.timeslots (\
           course_id, teacher_name, weekday, start_slot, end_slot, weeks, \
           week_numbers, weeks_unknown, location, location_unknown) \
         VALUES \
           ($1, '甲老师', 2, 5, 6, '1,2,3', ARRAY[1,2,3], false, 'A101', false), \
           ($2, '乙老师', 2, 6, 7, '1,2,3', ARRAY[1,2,3], false, 'A102', false), \
           ($3, '丁老师', 2, 6, 6, '1,2,3', ARRAY[1,2,3], false, 'A103', false), \
           ($4, '戊老师', 2, 6, 6, NULL, ARRAY[]::integer[], true, NULL, true)",
    )
    .bind(first_id)
    .bind(second_id)
    .bind(mixed_id)
    .bind(unknown_week_id)
    .execute(&pool)
    .await
    .expect("insert canonical timeslot fixtures");

    let app = courses::routes(test_state(pool.clone()));
    let query = format!(
        "/api/v2/selection/offerings?calendarId=1&courseCode={course_code}\
         &weekday=2&startSlot=6&endSlot=6&week=2&includeUnknownSchedule=false&limit=1"
    )
    .replace(' ', "");
    let first_page = get_json(app.clone(), &query).await;
    assert_eq!(first_page["items"].as_array().expect("first page items").len(), 1);
    assert_eq!(first_page["items"][0]["offeringId"], first_id.to_string());
    assert_eq!(first_page["hasMore"], true);
    let cursor = first_page["nextCursor"].as_str().expect("first page cursor");

    let second_page = get_json(app.clone(), &format!("{query}&cursor={cursor}")).await;
    assert_eq!(second_page["items"].as_array().expect("second page items").len(), 1);
    assert_eq!(second_page["items"][0]["offeringId"], second_id.to_string());
    assert_eq!(second_page["hasMore"], false);

    let strict_without_week = get_json(
        app.clone(),
        &format!(
            "/api/v2/selection/offerings?calendarId=1&courseCode={course_code}\
             &weekday=2&startSlot=6&endSlot=6&includeUnknownSchedule=false&limit=100"
        )
        .replace(' ', ""),
    )
    .await;
    let strict_ids: Vec<&str> = strict_without_week["items"]
        .as_array()
        .expect("strict no-week items")
        .iter()
        .filter_map(|item| item["offeringId"].as_str())
        .collect();
    assert_eq!(strict_ids, vec![first_id.to_string(), second_id.to_string()]);

    let with_unknown = get_json(
        app.clone(),
        &format!(
            "/api/v2/selection/offerings?calendarId=1&courseCode={course_code}\
             &weekday=2&startSlot=6&endSlot=6&week=2&includeUnknownSchedule=true&limit=100"
        )
        .replace(' ', ""),
    )
    .await;
    assert_eq!(with_unknown["items"].as_array().expect("unknown-inclusive items").len(), 5);
    let unknown_id_string = unknown_id.to_string();
    assert!(with_unknown["items"]
        .as_array()
        .expect("unknown-inclusive items")
        .iter()
        .any(|item| item["offeringId"].as_str() == Some(unknown_id_string.as_str())));

    let detail = get_json(app.clone(), &format!("/api/v2/selection/offerings/{first_id}")).await;
    assert_eq!(detail["offeringId"], first_id.to_string());
    let timeslots =
        get_json(app.clone(), &format!("/api/v2/selection/offerings/{first_id}/timeslots")).await;
    assert_eq!(timeslots[0]["weekNumbers"], json!([1, 2, 3]));

    let incomplete_time_filter = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/v2/selection/offerings?calendarId=1&courseCode={course_code}&weekday=2"
                ))
                .body(Body::empty())
                .expect("build incomplete time-filter request"),
        )
        .await
        .expect("send incomplete time-filter request");
    assert_eq!(incomplete_time_filter.status(), StatusCode::BAD_REQUEST);

    let stale_cursor = app
        .oneshot(
            Request::builder()
                .uri(
                    format!(
                        "/api/v2/selection/offerings?calendarId=1&courseCode={course_code}\
                         &weekday=2&startSlot=6&endSlot=6&week=3\
                         &includeUnknownSchedule=false&limit=1&cursor={cursor}"
                    )
                    .replace(' ', ""),
                )
                .body(Body::empty())
                .expect("build stale cursor request"),
        )
        .await
        .expect("send stale cursor request");
    assert_eq!(stale_cursor.status(), StatusCode::BAD_REQUEST);

    sqlx::query("DELETE FROM selection.timeslots WHERE course_id BETWEEN $1 AND $2")
        .bind(first_id)
        .bind(unknown_week_id)
        .execute(&pool)
        .await
        .expect("delete canonical timeslot fixtures");
    sqlx::query("DELETE FROM selection.courses WHERE id BETWEEN $1 AND $2")
        .bind(first_id)
        .bind(unknown_week_id)
        .execute(&pool)
        .await
        .expect("delete canonical offering fixtures");
}

#[tokio::test]
async fn materialization_preserves_uncertainty_and_is_idempotent() {
    let Some(pool) = isolated_database_pool("yourtj_selection_materialize").await else {
        return;
    };

    let retained_catalogue_id: i64 = sqlx::query_scalar(
        "INSERT INTO courses.courses (code, name, is_legacy) \
         VALUES ('RETAINED', '不在快照中的目录课程', 1) RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("insert retained catalogue fixture");
    sqlx::query(
        "INSERT INTO courses.course_aliases (course_id, alias) VALUES ($1, 'retained-alias')",
    )
    .bind(retained_catalogue_id)
    .execute(&pool)
    .await
    .expect("insert retained catalogue alias fixture");

    sqlx::query(
        "INSERT INTO courses.courses (\
           code, name, is_legacy, review_count, review_avg\
         ) VALUES ('C100', '门禁前目录名称', 1, 2, 5)",
    )
    .execute(&pool)
    .await
    .expect("insert guarded catalogue fixture");

    sqlx::raw_sql(
        "INSERT INTO selection.calendars (id, name, is_current) \
         VALUES (999, '门禁前学期', false); \
         INSERT INTO selection.courses (id, code, name, calendar_id) \
         VALUES (999, 'GUARDED', '门禁前教学班', 999)",
    )
    .execute(&pool)
    .await
    .expect("insert guarded selection fixture");

    sqlx::raw_sql(
        "INSERT INTO selection.pk_calendars (calendar_id, calendar_name) \
         VALUES (1, '测试学期'), (2, '下一测试学期'); \
         INSERT INTO selection.pk_course_natures (\
           course_label_id, course_label_name, calendar_id\
         ) VALUES (1, '必修', 1); \
         INSERT INTO selection.pk_course_natures_by_calendar (\
           calendar_id, course_label_id, course_label_name\
         ) VALUES (1, 1, '必修'); \
         INSERT INTO selection.pk_campuses (campus, campus_i18n, calendar_id) \
         VALUES ('四平路', '四平路', 1); \
         INSERT INTO selection.pk_faculties (faculty, faculty_i18n, calendar_id) \
         VALUES ('计算机学院', '计算机学院', 1); \
         INSERT INTO selection.pk_majors (id, code, grade, name, calendar_id) \
         VALUES (1, 'SE', 2026, '软件工程', 1); \
         INSERT INTO selection.pk_course_details (\
           id, code, name, course_label_id, campus, faculty, start_week, end_week, \
           course_code, course_name, credit, calendar_id) \
         VALUES \
           (1001, 'C100.01', '测试教学班', 1, '四平路', '计算机学院', \
             1, 3, 'C100', '测试课程', 2, 1), \
           (1002, 'C200.01', '未知周次教学班', 1, '四平路', '计算机学院', \
             NULL, NULL, 'C200', '未知周次课程', NULL, 1), \
           (1003, 'C300.01', '混合时段教学班', 1, '四平路', '计算机学院', \
             1, 3, 'C300', '混合时段课程', NULL, 1), \
           (2001, 'C100.02', '下一学期教学班', 1, '四平路', '计算机学院', \
             1, 3, 'C100', '下一学期课程名称', 3, 2); \
         INSERT INTO selection.pk_teachers_raw (\
           id, teaching_class_id, teacher_code, teacher_name, arrange_info_text) \
         VALUES \
           (1, 1001, 'T1', '甲老师', '甲老师(T1) 星期二5-6节 [1-3] A101'), \
           (2, 1001, 'T2', '乙老师', '甲老师(T1) 星期二5-6节 [1-3] A101'), \
           (3, 1002, 'T3', '丙老师', '无法解析的时段'), \
           (4, 1003, 'T4', '丁老师', E'丁老师(T4) 星期一1-2节 [1-3] C301\\n新版未知格式'); \
         INSERT INTO selection.pk_teacher_timeslots (\
           calendar_id, teaching_class_id, occupy_day, occupy_section, \
           teacher_code, teacher_name) \
         VALUES \
           (1, 1003, 2, 3, 'T4', '丁老师'), \
           (1, 1003, 2, 4, 'T4', '丁老师'); \
         INSERT INTO selection.pk_major_courses (major_id, course_id) \
         VALUES (1, 1001); \
         INSERT INTO selection.pk_fetch_logs (fetch_time, msg) \
         VALUES (extract(epoch from now())::bigint, 'integration fixture'); \
         INSERT INTO courses.pk_legacy_teachers (id, tid, name, department) \
         VALUES (10, 'T1', '甲老师', '计算机学院'); \
         INSERT INTO courses.pk_legacy_courses (\
           id, code, name, credit, department, teacher_id, review_count, review_avg\
         ) VALUES (50, 'C100', '测试课程', 2, '计算机学院', 10, 4, 4.5); \
         INSERT INTO courses.pk_legacy_course_aliases (system, alias, course_id, created_at) \
         VALUES ('onesystem', 'C100.01', 50, extract(epoch from now())::bigint)",
    )
    .execute(&pool)
    .await
    .expect("seed repeated teacher arrangement fixtures");

    assert_materializer_rejects_source(
        &pool,
        include_str!("../../../ops/materialize_courses.sql"),
        "no validated import run exists",
    )
    .await;
    assert_materializer_rejects_source(
        &pool,
        include_str!("../../../ops/materialize_selection.sql"),
        "no validated import run exists",
    )
    .await;
    let guarded_catalogue_name: String =
        sqlx::query_scalar("SELECT name FROM courses.courses WHERE code = 'C100'")
            .fetch_one(&pool)
            .await
            .expect("read guarded catalogue projection");
    assert_eq!(guarded_catalogue_name, "门禁前目录名称");
    let guarded_selection_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM selection.courses WHERE id = 999)")
            .fetch_one(&pool)
            .await
            .expect("read guarded selection projection");
    assert!(guarded_selection_exists, "preflight failure must preserve the old projection");

    let (import_run_id, validation) = record_validated_import_run(&pool, "a".repeat(64)).await;
    sqlx::query(
        "UPDATE selection.import_runs \
         SET validation = '{\"rowCountsMatched\":true,\"sourceSchemaValidated\":true}' \
         WHERE id = $1",
    )
    .bind(import_run_id)
    .execute(&pool)
    .await
    .expect("remove completeness approval from import fixture");
    assert_materializer_rejects_source(
        &pool,
        include_str!("../../../ops/materialize_selection.sql"),
        "latest import run is not validated",
    )
    .await;
    sqlx::query("UPDATE selection.import_runs SET validation = $2 WHERE id = $1")
        .bind(import_run_id)
        .bind(&validation)
        .execute(&pool)
        .await
        .expect("restore completeness approval on import fixture");

    let mut baseline_core_counts = validation["approvedCoreCounts"].clone();
    let current_teacher_count =
        baseline_core_counts["teacher"].as_i64().expect("teacher count is an integer");
    baseline_core_counts["teacher"] = json!(current_teacher_count + 1);
    let unapproved_decrease = json!({
        "rowCountsMatched": true,
        "sourceSchemaValidated": true,
        "completenessApproved": true,
        "approvalMode": "baselineCompared",
        "baselineSnapshotSha256": "b".repeat(64),
        "baselineCoreCounts": baseline_core_counts,
    });
    sqlx::query("UPDATE selection.import_runs SET validation = $2 WHERE id = $1")
        .bind(import_run_id)
        .bind(&unapproved_decrease)
        .execute(&pool)
        .await
        .expect("record semantically invalid baseline approval");
    assert_materializer_rejects_source(
        &pool,
        include_str!("../../../ops/materialize_selection.sql"),
        "snapshot completeness approval is invalid",
    )
    .await;

    let empty_decrease_override = json!({
        "rowCountsMatched": true,
        "sourceSchemaValidated": true,
        "completenessApproved": true,
        "approvalMode": "countDecreaseOverride",
        "approvalReason": "Reviewed expected integration fixture decrease",
        "baselineSnapshotSha256": "b".repeat(64),
        "baselineCoreCounts": baseline_core_counts,
        "countDecreases": {},
    });
    sqlx::query("UPDATE selection.import_runs SET validation = $2 WHERE id = $1")
        .bind(import_run_id)
        .bind(&empty_decrease_override)
        .execute(&pool)
        .await
        .expect("record empty count-decrease override");
    assert_materializer_rejects_source(
        &pool,
        include_str!("../../../ops/materialize_selection.sql"),
        "snapshot completeness approval is invalid",
    )
    .await;

    let valid_decrease_override = json!({
        "rowCountsMatched": true,
        "sourceSchemaValidated": true,
        "completenessApproved": true,
        "approvalMode": "countDecreaseOverride",
        "approvalReason": "Reviewed expected integration fixture decrease",
        "baselineSnapshotSha256": "b".repeat(64),
        "baselineCoreCounts": baseline_core_counts,
        "countDecreases": {
            "teacher": {
                "before": current_teacher_count + 1,
                "after": current_teacher_count,
            }
        },
    });
    sqlx::query("UPDATE selection.import_runs SET validation = $2 WHERE id = $1")
        .bind(import_run_id)
        .bind(&valid_decrease_override)
        .execute(&pool)
        .await
        .expect("record exact count-decrease override");
    sqlx::query("SELECT selection.assert_materialization_source()")
        .execute(&pool)
        .await
        .expect("accept exact count-decrease approval semantics");
    sqlx::query("UPDATE selection.import_runs SET validation = $2 WHERE id = $1")
        .bind(import_run_id)
        .bind(validation)
        .execute(&pool)
        .await
        .expect("restore unbaselined integration approval");

    for _ in 0..2 {
        sqlx::raw_sql(include_str!("../../../ops/materialize_courses.sql"))
            .execute(&pool)
            .await
            .expect("materialize catalogue fixture");
        sqlx::raw_sql(include_str!("../../../ops/materialize_selection.sql"))
            .execute(&pool)
            .await
            .expect("materialize selection fixture");
    }

    let canonical_course: (String, Option<f64>) =
        sqlx::query_as("SELECT name, credit FROM courses.courses WHERE code = 'C100'")
            .fetch_one(&pool)
            .await
            .expect("read deterministic catalogue course");
    assert_eq!(canonical_course, ("下一学期课程名称".into(), Some(3.0)));
    let catalogue_rating: (i32, f64, i32, f64) = sqlx::query_as(
        "SELECT review_count, review_avg, legacy_review_count, legacy_review_avg \
         FROM courses.courses WHERE code = 'C100'",
    )
    .fetch_one(&pool)
    .await
    .expect("read idempotent legacy and community rating aggregate");
    assert_eq!(catalogue_rating.0, 6);
    assert!((catalogue_rating.1 - 14.0 / 3.0).abs() < 1e-12);
    assert_eq!(catalogue_rating.2, 4);
    assert_eq!(catalogue_rating.3, 4.5);

    let fact: (i64, Option<String>, Vec<i32>, bool, Option<String>, bool) = sqlx::query_as(
        "SELECT course_id, teacher_name, week_numbers, weeks_unknown, location, location_unknown \
         FROM selection.timeslots WHERE course_id = 1001",
    )
    .fetch_one(&pool)
    .await
    .expect("read collapsed schedule fact");
    assert_eq!(fact.0, 1001);
    assert_eq!(fact.1.as_deref(), Some("甲老师"));
    assert_eq!(fact.2, vec![1, 2, 3]);
    assert!(!fact.3);
    assert_eq!(fact.4.as_deref(), Some("A101"));
    assert!(!fact.5);
    let rating: (i32, Option<f64>, String) = sqlx::query_as(
        "SELECT review_count, review_avg, review_scope \
         FROM selection.courses WHERE id = 1001",
    )
    .fetch_one(&pool)
    .await
    .expect("read teacher-matched legacy rating");
    assert_eq!(rating.0, 4);
    assert_eq!(rating.1, Some(4.5));
    assert_eq!(rating.2, "teacher");
    let fact_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM selection.timeslots WHERE course_id = 1001")
            .fetch_one(&pool)
            .await
            .expect("count idempotent schedule facts");
    assert_eq!(fact_count, 1);
    let unknown_flags: (bool, bool) = sqlx::query_as(
        "SELECT weeks_unknown, schedule_unknown FROM selection.courses WHERE id = 1002",
    )
    .fetch_one(&pool)
    .await
    .expect("read unknown schedule flags");
    assert_eq!(unknown_flags, (true, true));

    let mixed_slots: Vec<(i32, i32, i32)> = sqlx::query_as(
        "SELECT weekday, start_slot, end_slot FROM selection.timeslots \
         WHERE course_id = 1003 ORDER BY weekday, start_slot",
    )
    .fetch_all(&pool)
    .await
    .expect("read mixed-format schedule facts");
    assert_eq!(mixed_slots, vec![(1, 1, 2), (2, 3, 4)]);
    let mixed_unknown: bool =
        sqlx::query_scalar("SELECT schedule_unknown FROM selection.courses WHERE id = 1003")
            .fetch_one(&pool)
            .await
            .expect("read mixed-format uncertainty");
    assert!(mixed_unknown, "one unparsed arrangement line keeps the schedule unknown");

    sqlx::query(
        "INSERT INTO selection.pk_course_details \
         (id, code, course_code, course_name, calendar_id) \
         VALUES (1004, 'C400.01', 'C400', '漂移课程', 1)",
    )
    .execute(&pool)
    .await
    .expect("introduce raw count drift");
    assert_materializer_rejects_source(
        &pool,
        include_str!("../../../ops/materialize_courses.sql"),
        "raw row counts differ",
    )
    .await;
    assert_materializer_rejects_source(
        &pool,
        include_str!("../../../ops/materialize_selection.sql"),
        "raw row counts differ",
    )
    .await;
    let drifted_catalogue_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM courses.courses WHERE code = 'C400')")
            .fetch_one(&pool)
            .await
            .expect("check drifted catalogue projection");
    assert!(!drifted_catalogue_exists, "failed catalogue reconcile must not create rows");
    let drifted_selection_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM selection.courses WHERE id = 1004)")
            .fetch_one(&pool)
            .await
            .expect("check drifted selection projection");
    assert!(!drifted_selection_exists, "failed selection reconcile must preserve old rows");

    let over_limit_arrangement = (1..=101)
        .map(|index| format!("星期一1-1节 [1] L{index:03}"))
        .collect::<Vec<_>>()
        .join("\n");
    sqlx::query(
        "INSERT INTO selection.pk_course_details (\
           id, code, name, start_week, end_week, course_code, course_name, calendar_id\
         ) VALUES (1005, 'C500.01', '超量时段教学班', 1, 1, 'C500', '超量时段课程', 1)",
    )
    .execute(&pool)
    .await
    .expect("insert over-limit teaching class fixture");
    sqlx::query(
        "INSERT INTO selection.pk_teachers_raw (\
           id, teaching_class_id, teacher_code, teacher_name, arrange_info_text\
         ) VALUES (5, 1005, 'T5', '超量老师', $1)",
    )
    .bind(over_limit_arrangement)
    .execute(&pool)
    .await
    .expect("insert over-limit arrangement fixture");
    record_validated_import_run(&pool, "b".repeat(64)).await;
    assert_materializer_rejects_source(
        &pool,
        include_str!("../../../ops/materialize_selection.sql"),
        "offering exceeds 100 timeslots",
    )
    .await;
    let prior_projection_preserved: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM selection.courses WHERE id = 1001) \
           AND NOT EXISTS(SELECT 1 FROM selection.courses WHERE id = 1005)",
    )
    .fetch_one(&pool)
    .await
    .expect("verify over-limit materialization rollback");
    assert!(prior_projection_preserved);

    let retained_catalogue: bool = sqlx::query_scalar(
        "SELECT EXISTS(\
           SELECT 1 FROM courses.course_aliases \
           WHERE course_id = $1 AND alias = 'retained-alias'\
         )",
    )
    .bind(retained_catalogue_id)
    .fetch_one(&pool)
    .await
    .expect("verify conservative catalogue retention");
    assert!(retained_catalogue, "selection snapshots cannot retire catalogue rows or aliases");
}

#[tokio::test]
async fn arrangement_parser_accepts_real_identity_prefix_variants() {
    let Some(pool) = isolated_database_pool("yourtj_arrangement_parser").await else {
        return;
    };

    let named: (Option<String>, Option<String>, i32, Vec<i32>) = sqlx::query_as(
        "SELECT teacher_name, teacher_code, weekday, week_numbers \
         FROM selection.parse_arrangement_line(\
           '张老师(T100) 星期二3-4节 [1-5单] 教学楼 A101'\
         )",
    )
    .fetch_one(&pool)
    .await
    .expect("parse named teacher prefix");
    assert_eq!(named, (Some("张老师".into()), Some("T100".into()), 2, vec![1, 3, 5]));

    let code_only: (Option<String>, Option<String>, i32, Option<String>) = sqlx::query_as(
        "SELECT teacher_name, teacher_code, start_slot, location \
         FROM selection.parse_arrangement_line('(2500036) 星期三7-8节 [2-8双]')",
    )
    .fetch_one(&pool)
    .await
    .expect("parse code-only teacher prefix");
    assert_eq!(code_only, (None, Some("2500036".into()), 7, None));

    let ambiguous: (Option<String>, Option<String>, i32) = sqlx::query_as(
        "SELECT teacher_name, teacher_code, weekday \
         FROM selection.parse_arrangement_line(\
           '朱静宇(05072),(2401015) 星期一1-2节 [1-16] A101'\
         )",
    )
    .fetch_one(&pool)
    .await
    .expect("parse ambiguous multi-identity prefix");
    assert_eq!(ambiguous, (None, None, 1));
}

#[tokio::test]
async fn sync_queue_is_concurrently_idempotent_and_allows_only_one_active_job() {
    let Some(pool) = isolated_database_pool("yourtj_selection_queue").await else {
        return;
    };
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let account_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle, role) \
         VALUES ($1, $2, 'admin') RETURNING id",
    )
    .bind(format!("selection-queue-{suffix}@tongji.edu.cn"))
    .bind(format!("selection-queue-{suffix}"))
    .fetch_one(&pool)
    .await
    .expect("insert selection queue operator");

    let key_hash = "a".repeat(64);
    let fingerprint = "d".repeat(64);
    let (first, replay) = tokio::join!(
        enqueue_committed(pool.clone(), account_id, key_hash.clone(), fingerprint.clone()),
        enqueue_committed(pool.clone(), account_id, key_hash.clone(), fingerprint.clone()),
    );
    let first = first.expect("first concurrent idempotent enqueue");
    let replay = replay.expect("second concurrent idempotent enqueue");
    assert_eq!(first.0, replay.0);
    assert_eq!(usize::from(first.1) + usize::from(replay.1), 1);

    let mismatch = enqueue_committed(pool.clone(), account_id, key_hash, "e".repeat(64)).await;
    assert!(mismatch.is_err(), "same idempotency key with another payload must conflict");

    sqlx::query(
        "UPDATE selection.sync_jobs \
         SET status = 'succeeded', step = 'complete', progress_current = progress_total, \
             completed_at = now(), updated_at = now() \
         WHERE id = $1",
    )
    .bind(first.0)
    .execute(&pool)
    .await
    .expect("complete the first queue fixture");

    let (left, right) = tokio::join!(
        enqueue_committed(pool.clone(), account_id, "b".repeat(64), fingerprint.clone()),
        enqueue_committed(pool.clone(), account_id, "c".repeat(64), fingerprint),
    );
    assert_eq!(usize::from(left.is_ok()) + usize::from(right.is_ok()), 1);
    assert_eq!(usize::from(left.is_err()) + usize::from(right.is_err()), 1);
    let active_jobs: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM selection.sync_jobs WHERE status IN ('queued', 'running')",
    )
    .fetch_one(&pool)
    .await
    .expect("count active selection sync jobs");
    assert_eq!(active_jobs, 1);

    let mismatched_cursor = sync::list_sync_jobs(&pool, Some("queued"), Some(first.0), 10).await;
    assert!(matches!(mismatched_cursor, Err(shared::AppError::NotFound)));
}

#[tokio::test]
async fn expired_terminal_sync_lease_records_an_immutable_audit_event() {
    let Some(pool) = isolated_database_pool("yourtj_selection_expired_lease").await else {
        return;
    };
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let account_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle, role) \
         VALUES ($1, $2, 'admin') RETURNING id",
    )
    .bind(format!("selection-lease-{suffix}@tongji.edu.cn"))
    .bind(format!("selection-lease-{suffix}"))
    .fetch_one(&pool)
    .await
    .expect("insert expired-lease operator");
    let (job_id, created) =
        enqueue_committed(pool.clone(), account_id, "f".repeat(64), "a".repeat(64))
            .await
            .expect("enqueue expired-lease fixture");
    assert!(created);
    sqlx::query(
        "UPDATE selection.sync_jobs \
         SET status = 'running', attempts = 8, locked_at = now() - interval '31 minutes', \
             lease_token = $2, lease_expires_at = now() - interval '1 minute' \
         WHERE id = $1",
    )
    .bind(job_id)
    .bind(uuid::Uuid::new_v4())
    .execute(&pool)
    .await
    .expect("expire terminal selection sync lease");

    assert!(!sync::process_one_selection_sync_job(&test_state(pool.clone()))
        .await
        .expect("recover expired terminal lease"));
    let event: (String, String, Value) = sqlx::query_as(
        "SELECT action, reason, metadata FROM governance.audit_events \
         WHERE target_type = 'selection_sync_job' AND target_id = $1 \
         ORDER BY id DESC LIMIT 1",
    )
    .bind(job_id.to_string())
    .fetch_one(&pool)
    .await
    .expect("read expired-lease audit event");
    assert_eq!(event.0, "selection.sync.dead");
    assert_eq!(event.1, "selection projection sync worker lease expired");
    assert_eq!(event.2["errorCode"], "worker_lease_expired");
    assert_eq!(event.2["nextStatus"], "dead");
}
