//! Populated upgrade coverage for account-lifecycle lease fencing migration 0058.

use std::borrow::Cow;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use sqlx::migrate::Migrator;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{Connection, PgConnection, PgPool};

static MIGRATOR: Migrator = sqlx::migrate!("../../migrations");

#[derive(Debug, sqlx::FromRow)]
struct LegacyJobRow {
    status: String,
    attempts: i16,
    next_attempt_at: DateTime<Utc>,
    locked_at: Option<DateTime<Utc>>,
    last_error_code: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct LeasedJobRow {
    status: String,
    attempts: i16,
    next_attempt_at: DateTime<Utc>,
    locked_at: Option<DateTime<Utc>>,
    lease_token: Option<uuid::Uuid>,
    last_error_code: Option<String>,
}

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

async fn insert_account(pool: &PgPool, suffix: &str, label: &str) -> i64 {
    sqlx::query_scalar("INSERT INTO identity.accounts (email, handle) VALUES ($1, $2) RETURNING id")
        .bind(format!("lease-upgrade-{label}-{suffix}@tongji.edu.cn"))
        .bind(format!("lease-upgrade-{label}-{suffix}"))
        .fetch_one(pool)
        .await
        .expect("insert lifecycle lease upgrade account")
}

#[tokio::test]
async fn populated_0058_upgrade_recovers_tokenless_running_jobs_and_fences_new_leases() {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL for lifecycle migration upgrade");
    let base_options =
        PgConnectOptions::from_str(&database_url).expect("parse lifecycle migration DB URL");
    let admin_options = base_options.clone().database("postgres");
    let mut admin = PgConnection::connect_with(&admin_options)
        .await
        .expect("connect lifecycle migration database administrator");
    let database_name = format!("yourtj_identity_0058_{}_test", uuid::Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE DATABASE \"{database_name}\""))
        .execute(&mut admin)
        .await
        .expect("create isolated lifecycle migration database");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(base_options.database(&database_name))
        .await
        .expect("connect isolated lifecycle migration database");

    migrations_matching(|version| version < 58)
        .run(&pool)
        .await
        .expect("migrate populated fixture through 0057");
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let running_account_id = insert_account(&pool, &suffix, "running").await;
    let queued_account_id = insert_account(&pool, &suffix, "queued").await;
    let running_job_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.account_lifecycle_jobs \
         (account_id, job_type, status, attempts, next_attempt_at, locked_at, last_error_code) \
         VALUES ($1, 'purge', 'running', 7, now() + interval '2 hours', \
                 now() - interval '1 hour', 'legacy_worker_running') RETURNING id",
    )
    .bind(running_account_id)
    .fetch_one(&pool)
    .await
    .expect("insert tokenless running lifecycle job");
    let queued_job_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.account_lifecycle_jobs \
         (account_id, job_type, status, attempts, next_attempt_at, last_error_code) \
         VALUES ($1, 'purge', 'queued', 3, now() + interval '1 day', \
                 'queued_before_upgrade') RETURNING id",
    )
    .bind(queued_account_id)
    .fetch_one(&pool)
    .await
    .expect("insert queued lifecycle job");
    let queued_before = sqlx::query_as::<_, LegacyJobRow>(
        "SELECT status, attempts, next_attempt_at, locked_at, last_error_code \
         FROM identity.account_lifecycle_jobs WHERE id = $1",
    )
    .bind(queued_job_id)
    .fetch_one(&pool)
    .await
    .expect("read queued lifecycle job before upgrade");

    migrations_matching(|version| version == 58)
        .run(&pool)
        .await
        .expect("apply lifecycle lease fencing migration 0058");

    let recovered = sqlx::query_as::<_, LeasedJobRow>(
        "SELECT status, attempts, next_attempt_at, locked_at, lease_token, last_error_code \
         FROM identity.account_lifecycle_jobs WHERE id = $1",
    )
    .bind(running_job_id)
    .fetch_one(&pool)
    .await
    .expect("read recovered lifecycle job");
    assert_eq!(recovered.status, "failed");
    assert_eq!(recovered.attempts, 7);
    assert!(recovered.locked_at.is_none());
    assert!(recovered.lease_token.is_none());
    assert_eq!(recovered.last_error_code.as_deref(), Some("lease_fencing_migration_recovery"));
    assert!(
        recovered.next_attempt_at <= Utc::now(),
        "recovered lifecycle job must be immediately retryable"
    );

    let queued_after = sqlx::query_as::<_, LeasedJobRow>(
        "SELECT status, attempts, next_attempt_at, locked_at, lease_token, last_error_code \
         FROM identity.account_lifecycle_jobs WHERE id = $1",
    )
    .bind(queued_job_id)
    .fetch_one(&pool)
    .await
    .expect("read queued lifecycle job after upgrade");
    assert_eq!(queued_after.status, queued_before.status);
    assert_eq!(queued_after.attempts, queued_before.attempts);
    assert_eq!(queued_after.next_attempt_at, queued_before.next_attempt_at);
    assert_eq!(queued_after.locked_at, queued_before.locked_at);
    assert!(queued_after.lease_token.is_none());
    assert_eq!(queued_after.last_error_code, queued_before.last_error_code);

    let missing_running_token = sqlx::query(
        "UPDATE identity.account_lifecycle_jobs \
         SET status = 'running', locked_at = now(), lease_token = NULL WHERE id = $1",
    )
    .bind(running_job_id)
    .execute(&pool)
    .await;
    assert!(missing_running_token.is_err(), "running jobs must carry a lease token");

    let token_on_queued =
        sqlx::query("UPDATE identity.account_lifecycle_jobs SET lease_token = $2 WHERE id = $1")
            .bind(queued_job_id)
            .bind(uuid::Uuid::new_v4())
            .execute(&pool)
            .await;
    assert!(token_on_queued.is_err(), "non-running jobs must not carry a lease token");

    let lock_on_queued =
        sqlx::query("UPDATE identity.account_lifecycle_jobs SET locked_at = now() WHERE id = $1")
            .bind(queued_job_id)
            .execute(&pool)
            .await;
    assert!(lock_on_queued.is_err(), "non-running jobs must not retain a worker lock");

    let first_leased_account_id = insert_account(&pool, &suffix, "leased-a").await;
    let second_leased_account_id = insert_account(&pool, &suffix, "leased-b").await;
    let lease_token = uuid::Uuid::new_v4();
    let leased_job_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.account_lifecycle_jobs \
         (account_id, job_type, status, attempts, next_attempt_at, locked_at, lease_token) \
         VALUES ($1, 'purge', 'running', 1, now(), now(), $2) RETURNING id",
    )
    .bind(first_leased_account_id)
    .bind(lease_token)
    .fetch_one(&pool)
    .await
    .expect("insert valid leased lifecycle job");
    let persisted_lease: uuid::Uuid =
        sqlx::query_scalar("SELECT lease_token FROM identity.account_lifecycle_jobs WHERE id = $1")
            .bind(leased_job_id)
            .fetch_one(&pool)
            .await
            .expect("read valid lifecycle lease token");
    assert_eq!(persisted_lease, lease_token);

    let duplicate_lease = sqlx::query(
        "INSERT INTO identity.account_lifecycle_jobs \
         (account_id, job_type, status, attempts, next_attempt_at, locked_at, lease_token) \
         VALUES ($1, 'purge', 'running', 1, now(), now(), $2)",
    )
    .bind(second_leased_account_id)
    .bind(lease_token)
    .execute(&pool)
    .await;
    assert!(duplicate_lease.is_err(), "active lifecycle lease tokens must be unique");

    pool.close().await;
    sqlx::query(&format!("DROP DATABASE \"{database_name}\""))
        .execute(&mut admin)
        .await
        .expect("drop isolated lifecycle migration database");
}
