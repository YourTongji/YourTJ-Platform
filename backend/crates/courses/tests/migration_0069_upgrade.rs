//! Populated upgrade coverage for selection teaching-class alignment migration 0069.

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
async fn populated_0069_upgrade_backfills_calendar_and_preserves_unknown_facts() {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL for selection migration upgrade");
    let base_options =
        PgConnectOptions::from_str(&database_url).expect("parse selection migration DB URL");
    let mut admin = PgConnection::connect_with(&base_options.clone().database("postgres"))
        .await
        .expect("connect selection migration database administrator");
    let database_name = format!("yourtj_selection_0069_{}_test", uuid::Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE DATABASE \"{database_name}\""))
        .execute(&mut admin)
        .await
        .expect("create isolated selection migration database");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(base_options.database(&database_name))
        .await
        .expect("connect isolated selection migration database");

    migrations_matching(|version| version < 69)
        .run(&pool)
        .await
        .expect("migrate populated fixture through 0068");
    sqlx::raw_sql(
        "INSERT INTO selection.pk_calendars (calendar_id, calendar_name) \
         VALUES (42, '升级测试学期'); \
         INSERT INTO selection.calendars (id, name, is_current) \
         VALUES (42, '旧 normalized 学期', false); \
         INSERT INTO selection.pk_course_details (\
           id, code, name, start_week, end_week, course_code, course_name, calendar_id\
         ) VALUES (1001, 'C100.01', '升级教学班', NULL, NULL, 'C100', '升级课程', 42); \
         INSERT INTO selection.courses (id, code, name, calendar_id) \
         VALUES \
           (1001, 'C100', '升级课程', NULL), \
           (1002, 'C200', '缺失 raw 证据课程', 42); \
         INSERT INTO selection.timeslots (\
           course_id, weekday, start_slot, end_slot, location\
         ) VALUES \
           (1001, 1, 1, 2, 'A101'), \
           (1001, 1, 3, 4, '   '), \
           (1001, NULL, 0, 21, 'legacy-invalid')",
    )
    .execute(&pool)
    .await
    .expect("seed pre-0069 selection fixture");

    let unmapped_upgrade = migrations_matching(|version| version == 69).run(&pool).await;
    assert!(
        unmapped_upgrade.is_err(),
        "a legacy calendar value without same-id raw evidence must fail the migration"
    );
    sqlx::query(
        "INSERT INTO selection.pk_course_details (\
           id, code, name, start_week, end_week, course_code, course_name, calendar_id\
         ) VALUES (1002, 'C200.01', '补齐教学班', 1, 3, 'C200', '补齐课程', 42)",
    )
    .execute(&pool)
    .await
    .expect("repair raw calendar mapping after failed upgrade");

    migrations_matching(|version| version == 69)
        .run(&pool)
        .await
        .expect("apply selection teaching-class migration 0069");

    let offering: (i64, bool) =
        sqlx::query_as("SELECT calendar_id, weeks_unknown FROM selection.courses WHERE id = 1001")
            .fetch_one(&pool)
            .await
            .expect("read migrated selection offering");
    assert_eq!(offering, (42, true));
    let repaired_calendar: i64 =
        sqlx::query_scalar("SELECT calendar_id FROM selection.courses WHERE id = 1002")
            .fetch_one(&pool)
            .await
            .expect("read repaired calendar backfill");
    assert_eq!(repaired_calendar, 42);
    let location_unknown: bool = sqlx::query_scalar(
        "SELECT location_unknown FROM selection.timeslots \
         WHERE course_id = 1001 AND start_slot = 1",
    )
    .fetch_one(&pool)
    .await
    .expect("read migrated location certainty");
    assert!(!location_unknown);
    let normalized_blank_location: bool = sqlx::query_scalar(
        "SELECT EXISTS(\
           SELECT 1 FROM selection.timeslots \
           WHERE course_id = 1001 AND start_slot = 3 \
             AND location IS NULL AND location_unknown\
         )",
    )
    .fetch_one(&pool)
    .await
    .expect("check blank location normalization");
    assert!(normalized_blank_location);
    let invalid_legacy_timeslots: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM selection.timeslots \
         WHERE course_id = 1001 AND location = 'legacy-invalid'",
    )
    .fetch_one(&pool)
    .await
    .expect("check invalid legacy timeslot removal");
    assert_eq!(invalid_legacy_timeslots, 0);

    let calendar_is_nullable: String = sqlx::query_scalar(
        "SELECT is_nullable FROM information_schema.columns \
         WHERE table_schema = 'selection' AND table_name = 'courses' \
           AND column_name = 'calendar_id'",
    )
    .fetch_one(&pool)
    .await
    .expect("inspect migrated calendar constraint");
    assert_eq!(calendar_is_nullable, "NO");

    let prefixed_arrangement_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM selection.parse_arrangement_line(\
           'prefix星期一1-2节 [1-3] A101'\
         )",
    )
    .fetch_one(&pool)
    .await
    .expect("check strict arrangement parsing");
    assert_eq!(prefixed_arrangement_rows, 0);

    let half_null_weeks =
        sqlx::query("UPDATE selection.courses SET start_week = 1, end_week = NULL WHERE id = 1001")
            .execute(&pool)
            .await;
    assert!(half_null_weeks.is_err(), "half-null week ranges must be rejected");
    let false_unknown_without_range =
        sqlx::query("UPDATE selection.courses SET weeks_unknown = false WHERE id = 1001")
            .execute(&pool)
            .await;
    assert!(false_unknown_without_range.is_err(), "known course weeks require a complete range");
    let unknown_with_week_numbers = sqlx::query(
        "UPDATE selection.timeslots SET week_numbers = ARRAY[1], weeks_unknown = true \
         WHERE course_id = 1001 AND start_slot = 1",
    )
    .execute(&pool)
    .await;
    assert!(
        unknown_with_week_numbers.is_err(),
        "unknown timeslot weeks cannot retain week numbers"
    );
    let invalid_slot_range = sqlx::query(
        "UPDATE selection.timeslots SET weekday = 8, start_slot = 20, end_slot = 19 \
         WHERE course_id = 1001 AND start_slot = 1",
    )
    .execute(&pool)
    .await;
    assert!(invalid_slot_range.is_err(), "day and slot ranges must remain valid");
    let missing_known_location = sqlx::query(
        "UPDATE selection.timeslots SET location = NULL, location_unknown = false \
         WHERE course_id = 1001 AND start_slot = 1",
    )
    .execute(&pool)
    .await;
    assert!(missing_known_location.is_err(), "known locations require a value");
    let unknown_with_location = sqlx::query(
        "UPDATE selection.timeslots SET location_unknown = true \
         WHERE course_id = 1001 AND start_slot = 1",
    )
    .execute(&pool)
    .await;
    assert!(unknown_with_location.is_err(), "unknown locations cannot retain a location value");

    pool.close().await;
    sqlx::query(&format!("DROP DATABASE \"{database_name}\""))
        .execute(&mut admin)
        .await
        .expect("drop isolated selection migration database");
}
