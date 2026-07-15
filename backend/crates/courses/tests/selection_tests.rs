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
            "catalogueCourseId": null
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
    let course_code = format!("OFFER{first_id}");

    sqlx::query(
        "INSERT INTO selection.courses (\
           id, code, teaching_class_code, name, calendar_id, teacher_names, \
           start_week, end_week, weeks_unknown, schedule_unknown) \
         VALUES \
           ($1, $4, 'OFFER.01', '教学班一', 1, ARRAY['甲老师'], 1, 3, false, false), \
           ($2, $4, 'OFFER.02', '教学班二', 1, ARRAY['乙老师'], 1, 3, false, false), \
           ($3, $4, 'OFFER.03', '时段未知教学班', 1, ARRAY['丙老师'], NULL, NULL, true, true)",
    )
    .bind(first_id)
    .bind(second_id)
    .bind(unknown_id)
    .bind(&course_code)
    .execute(&pool)
    .await
    .expect("insert canonical offering fixtures");
    sqlx::query(
        "INSERT INTO selection.timeslots (\
           course_id, teacher_name, weekday, start_slot, end_slot, weeks, \
           week_numbers, weeks_unknown, location, location_unknown) \
         VALUES \
           ($1, '甲老师', 2, 5, 6, '1,2,3', ARRAY[1,2,3], false, 'A101', false), \
           ($2, '乙老师', 2, 6, 7, '1,2,3', ARRAY[1,2,3], false, 'A102', false)",
    )
    .bind(first_id)
    .bind(second_id)
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

    let with_unknown = get_json(
        app.clone(),
        &format!(
            "/api/v2/selection/offerings?calendarId=1&courseCode={course_code}\
             &weekday=2&startSlot=6&endSlot=6&week=2&includeUnknownSchedule=true&limit=100"
        )
        .replace(' ', ""),
    )
    .await;
    assert_eq!(with_unknown["items"].as_array().expect("unknown-inclusive items").len(), 3);
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
        .bind(unknown_id)
        .execute(&pool)
        .await
        .expect("delete canonical timeslot fixtures");
    sqlx::query("DELETE FROM selection.courses WHERE id BETWEEN $1 AND $2")
        .bind(first_id)
        .bind(unknown_id)
        .execute(&pool)
        .await
        .expect("delete canonical offering fixtures");
}

#[tokio::test]
async fn materialization_collapses_repeated_teacher_schedule_facts_and_is_idempotent() {
    let Some(pool) = isolated_database_pool("yourtj_selection_materialize").await else {
        return;
    };

    sqlx::raw_sql(
        "INSERT INTO selection.pk_calendars (calendar_id, calendar_name) \
         VALUES (1, '测试学期'); \
         INSERT INTO selection.pk_course_details (\
           id, code, name, start_week, end_week, course_code, course_name, calendar_id) \
         VALUES \
           (1001, 'C100.01', '测试教学班', 1, 3, 'C100', '测试课程', 1), \
           (1002, 'C200.01', '未知时段教学班', 0, 0, 'C200', '未知时段课程', 1); \
         INSERT INTO selection.pk_teachers_raw (\
           id, teaching_class_id, teacher_code, teacher_name, arrange_info_text) \
         VALUES \
           (1, 1001, 'T1', '甲老师', '星期二5-6节 [1-3] A101'), \
           (2, 1001, 'T2', '乙老师', '星期二5-6节 [1-3] A101'), \
           (3, 1002, 'T3', '丙老师', '无法解析的时段')",
    )
    .execute(&pool)
    .await
    .expect("seed repeated teacher arrangement fixtures");

    for _ in 0..2 {
        sqlx::raw_sql(include_str!("../../../ops/materialize_selection.sql"))
            .execute(&pool)
            .await
            .expect("materialize selection fixture");
    }

    let fact: (i64, Option<String>, Vec<i32>, bool, Option<String>, bool) = sqlx::query_as(
        "SELECT course_id, teacher_name, week_numbers, weeks_unknown, location, location_unknown \
         FROM selection.timeslots WHERE course_id = 1001",
    )
    .fetch_one(&pool)
    .await
    .expect("read collapsed schedule fact");
    assert_eq!(fact.0, 1001);
    assert_eq!(fact.1, None, "a schedule repeated by multiple teachers is not teacher-owned");
    assert_eq!(fact.2, vec![1, 2, 3]);
    assert!(!fact.3);
    assert_eq!(fact.4.as_deref(), Some("A101"));
    assert!(!fact.5);
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
