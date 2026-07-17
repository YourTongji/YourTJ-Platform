//! Populated-upgrade coverage for courses data-pipeline migration 0071.

use std::borrow::Cow;
use std::str::FromStr;

use sqlx::migrate::Migrator;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{Connection, PgConnection};

static MIGRATOR: Migrator = sqlx::migrate!("../../migrations");

fn migrations_matching(predicate: impl Fn(i64) -> bool) -> Migrator {
    Migrator {
        migrations: Cow::Owned(
            MIGRATOR.iter().filter(|migration| predicate(migration.version)).cloned().collect(),
        ),
        ignore_missing: true,
        locking: true,
        no_tx: false,
    }
}

#[tokio::test]
async fn populated_0071_upgrade_preserves_community_metrics_and_stales_search() {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL for courses migration upgrade");
    let base_options =
        PgConnectOptions::from_str(&database_url).expect("parse courses migration DB URL");
    let mut admin = PgConnection::connect_with(&base_options.clone().database("postgres"))
        .await
        .expect("connect courses migration database administrator");
    let database_name = format!("yourtj_courses_0071_{}_test", uuid::Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE DATABASE \"{database_name}\""))
        .execute(&mut admin)
        .await
        .expect("create isolated courses migration database");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(base_options.database(&database_name))
        .await
        .expect("connect isolated courses migration database");

    migrations_matching(|version| version < 71)
        .run(&pool)
        .await
        .expect("migrate populated fixture through 0070");
    let course_id: i64 = sqlx::query_scalar(
        "INSERT INTO courses.courses (code, name, review_count, review_avg) \
         VALUES ('UPGRADE-0071', '升级课程', 2, 4.5) RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("seed pre-0071 catalogue aggregate");
    sqlx::raw_sql(
        "INSERT INTO selection.calendars (id, name, is_current) \
         VALUES (71, '升级学期', true); \
         INSERT INTO selection.courses (id, code, name, calendar_id) \
         VALUES (71001, 'UPGRADE-0071', '升级教学班', 71)",
    )
    .execute(&pool)
    .await
    .expect("seed pre-0071 selection offering");

    migrations_matching(|version| version == 71)
        .run(&pool)
        .await
        .expect("apply courses data-pipeline migration 0071");

    let catalogue: (i32, f64, i32, f64) = sqlx::query_as(
        "SELECT review_count, review_avg, legacy_review_count, legacy_review_avg \
         FROM courses.courses WHERE id = $1",
    )
    .bind(course_id)
    .fetch_one(&pool)
    .await
    .expect("read preserved catalogue aggregate");
    assert_eq!(catalogue, (2, 4.5, 0, 0.0));
    let offering: (i32, Option<f64>, String) = sqlx::query_as(
        "SELECT review_count, review_avg, review_scope \
         FROM selection.courses WHERE id = 71001",
    )
    .fetch_one(&pool)
    .await
    .expect("read backfilled selection aggregate");
    assert_eq!(offering, (0, None, "none".into()));
    let readiness: Vec<(String, i64, String)> = sqlx::query_as(
        "SELECT projection, source_rows, status \
         FROM courses.search_projection_state ORDER BY projection",
    )
    .fetch_all(&pool)
    .await
    .expect("read initialized search readiness");
    assert_eq!(
        readiness,
        vec![("catalogue".into(), 1, "stale".into()), ("selection".into(), 1, "stale".into())]
    );

    let contradictory_review = sqlx::query(
        "UPDATE selection.courses \
         SET review_count = 1, review_avg = NULL, review_scope = 'teacher' \
         WHERE id = 71001",
    )
    .execute(&pool)
    .await;
    assert!(contradictory_review.is_err(), "selection rating facts must remain consistent");

    pool.close().await;
    sqlx::query(&format!("DROP DATABASE \"{database_name}\""))
        .execute(&mut admin)
        .await
        .expect("drop isolated courses migration database");
}
