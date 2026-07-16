//! Fresh and populated-upgrade coverage for wallet-claim privacy migration 0070.

use std::borrow::Cow;
use std::str::FromStr;

use sqlx::migrate::Migrator;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{Connection, PgConnection, PgPool};

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

struct IsolatedDatabase {
    admin: PgConnection,
    name: String,
    pool: PgPool,
}

impl IsolatedDatabase {
    async fn create(label: &str) -> Self {
        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL for wallet claim privacy migration");
        let base_options =
            PgConnectOptions::from_str(&database_url).expect("parse migration database URL");
        let mut admin = PgConnection::connect_with(&base_options.clone().database("postgres"))
            .await
            .expect("connect migration database administrator");
        let name = format!("ytj_0070_{label}_{}_test", uuid::Uuid::new_v4().simple());
        sqlx::query(&format!("CREATE DATABASE \"{name}\""))
            .execute(&mut admin)
            .await
            .expect("create isolated wallet claim migration database");
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect_with(base_options.database(&name))
            .await
            .expect("connect isolated wallet claim migration database");
        Self { admin, name, pool }
    }

    async fn finish(mut self) {
        self.pool.close().await;
        sqlx::query(&format!("DROP DATABASE \"{}\"", self.name))
            .execute(&mut self.admin)
            .await
            .expect("drop isolated wallet claim migration database");
    }
}

async fn insert_account(pool: &PgPool, suffix: &str, label: &str) -> i64 {
    sqlx::query_scalar("INSERT INTO identity.accounts (email, handle) VALUES ($1, $2) RETURNING id")
        .bind(format!("claim-privacy-{label}-{suffix}@tongji.edu.cn"))
        .bind(format!("claim-{label}-{}", &suffix[..12]))
        .fetch_one(pool)
        .await
        .expect("insert wallet claim migration account")
}

async fn insert_course(pool: &PgPool, suffix: &str) -> i64 {
    sqlx::query_scalar(
        "INSERT INTO courses.courses (code, name) VALUES ($1, 'Claim privacy migration') \
         RETURNING id",
    )
    .bind(format!("CLAIM-PRIVACY-{suffix}"))
    .fetch_one(pool)
    .await
    .expect("insert wallet claim migration course")
}

#[tokio::test]
async fn populated_0070_upgrade_bounds_challenges_and_retires_claimed_credentials() {
    let database = IsolatedDatabase::create("upgrade").await;
    migrations_matching(|version| version < 70)
        .run(&database.pool)
        .await
        .expect("migrate populated fixture through 0069");
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let claimed_account_id = insert_account(&database.pool, &suffix, "claimed").await;
    let other_account_id = insert_account(&database.pool, &suffix, "other").await;
    let purge_account_id = insert_account(&database.pool, &suffix, "purge").await;
    let course_id = insert_course(&database.pool, &suffix).await;
    let claimed_review_id: i64 = sqlx::query_scalar(
        "INSERT INTO reviews.reviews \
         (course_id, account_id, rating, comment, reviewer_name, wallet_user_hash, edit_token, \
          is_legacy) \
         VALUES ($1, $2, 5, 'claimed', 'legacy', $3, $4, 1) RETURNING id",
    )
    .bind(course_id)
    .bind(claimed_account_id)
    .bind(format!("claimed-hash-{suffix}"))
    .bind(format!("claimed-token-{suffix}"))
    .fetch_one(&database.pool)
    .await
    .expect("insert claimed legacy review");
    let unclaimed_hash = format!("unclaimed-hash-{suffix}");
    let unclaimed_token = format!("unclaimed-token-{suffix}");
    let unclaimed_review_id: i64 = sqlx::query_scalar(
        "INSERT INTO reviews.reviews \
         (course_id, rating, comment, reviewer_name, wallet_user_hash, edit_token, is_legacy) \
         VALUES ($1, 4, 'unclaimed', 'legacy', $2, $3, 1) RETURNING id",
    )
    .bind(course_id)
    .bind(&unclaimed_hash)
    .bind(&unclaimed_token)
    .fetch_one(&database.pool)
    .await
    .expect("insert unclaimed legacy review");
    let purge_review_id: i64 = sqlx::query_scalar(
        "INSERT INTO reviews.reviews \
         (course_id, account_id, rating, comment, reviewer_name, wallet_user_hash, edit_token, \
          is_legacy) \
         VALUES ($1, $2, 3, 'purge fallback', 'legacy', $3, $4, 1) RETURNING id",
    )
    .bind(course_id)
    .bind(purge_account_id)
    .bind(format!("purge-hash-{suffix}"))
    .bind(format!("purge-token-{suffix}"))
    .fetch_one(&database.pool)
    .await
    .expect("insert purge fallback legacy review");
    reviews::data_export::purge_account_private_data(&database.pool, purge_account_id)
        .await
        .expect("purge claimed legacy review credentials");
    let purged_credentials: (Option<String>, Option<String>) =
        sqlx::query_as("SELECT wallet_user_hash, edit_token FROM reviews.reviews WHERE id = $1")
            .bind(purge_review_id)
            .fetch_one(&database.pool)
            .await
            .expect("read purged legacy review credentials");
    assert_eq!(purged_credentials, (None, None));

    sqlx::query(
        "INSERT INTO identity.wallet_claim_challenges \
         (id, account_id, nonce, expires_at, used_at, created_at) VALUES \
         ('expired', $1, 'expired-nonce', now() - interval '1 minute', NULL, \
          now() - interval '4 minutes'), \
         ('used', $1, 'used-nonce', now() + interval '10 minutes', now(), \
          now() - interval '3 minutes'), \
         ('active-old', $1, 'old-nonce', now() + interval '10 minutes', NULL, \
          now() - interval '2 minutes'), \
         ('active-new', $1, 'new-nonce', now() + interval '10 minutes', NULL, \
          now() - interval '1 minute'), \
         ('other-active', $2, 'other-nonce', now() + interval '10 minutes', NULL, now())",
    )
    .bind(claimed_account_id)
    .bind(other_account_id)
    .execute(&database.pool)
    .await
    .expect("seed legacy wallet claim challenges");

    migrations_matching(|version| version == 70)
        .run(&database.pool)
        .await
        .expect("apply wallet claim privacy migration 0070");

    let claimed_credentials: (Option<String>, Option<String>) =
        sqlx::query_as("SELECT wallet_user_hash, edit_token FROM reviews.reviews WHERE id = $1")
            .bind(claimed_review_id)
            .fetch_one(&database.pool)
            .await
            .expect("read migrated claimed review");
    assert_eq!(claimed_credentials, (None, None));
    let unclaimed_credentials: (Option<String>, Option<String>) =
        sqlx::query_as("SELECT wallet_user_hash, edit_token FROM reviews.reviews WHERE id = $1")
            .bind(unclaimed_review_id)
            .fetch_one(&database.pool)
            .await
            .expect("read migrated unclaimed review");
    assert_eq!(unclaimed_credentials, (Some(unclaimed_hash), Some(unclaimed_token)));

    let challenges: Vec<(i64, String)> = sqlx::query_as(
        "SELECT account_id, id FROM identity.wallet_claim_challenges ORDER BY account_id, id",
    )
    .fetch_all(&database.pool)
    .await
    .expect("read bounded wallet claim challenges");
    assert_eq!(
        challenges,
        vec![(claimed_account_id, "active-new".into()), (other_account_id, "other-active".into()),]
    );
    let duplicate_challenge = sqlx::query(
        "INSERT INTO identity.wallet_claim_challenges (id, account_id, nonce, expires_at) \
         VALUES ('duplicate', $1, 'duplicate-nonce', now() + interval '10 minutes')",
    )
    .bind(claimed_account_id)
    .execute(&database.pool)
    .await;
    assert!(duplicate_challenge.is_err(), "database must reject a second account challenge");
    let leaked_claimed_credentials = sqlx::query(
        "UPDATE reviews.reviews SET wallet_user_hash = 'leaked', edit_token = 'leaked' \
         WHERE id = $1",
    )
    .bind(claimed_review_id)
    .execute(&database.pool)
    .await;
    assert!(
        leaked_claimed_credentials.is_err(),
        "claimed reviews must reject legacy identifier or edit credential writes"
    );

    database.finish().await;
}

#[tokio::test]
async fn fresh_0070_schema_enforces_wallet_claim_privacy_constraints() {
    let database = IsolatedDatabase::create("fresh").await;
    MIGRATOR.run(&database.pool).await.expect("apply all migrations to a fresh database");
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let account_id = insert_account(&database.pool, &suffix, "fresh").await;
    sqlx::query(
        "INSERT INTO identity.wallet_claim_challenges (id, account_id, nonce, expires_at) \
         VALUES ('fresh-first', $1, 'fresh-nonce', now() + interval '10 minutes')",
    )
    .bind(account_id)
    .execute(&database.pool)
    .await
    .expect("insert first fresh-schema challenge");
    let second = sqlx::query(
        "INSERT INTO identity.wallet_claim_challenges (id, account_id, nonce, expires_at) \
         VALUES ('fresh-second', $1, 'fresh-nonce-2', now() + interval '10 minutes')",
    )
    .bind(account_id)
    .execute(&database.pool)
    .await;
    assert!(second.is_err(), "fresh schema must bound each account to one challenge");

    let course_id = insert_course(&database.pool, &suffix).await;
    let invalid_claimed_review = sqlx::query(
        "INSERT INTO reviews.reviews \
         (course_id, account_id, rating, comment, reviewer_name, wallet_user_hash, edit_token, \
          is_legacy) \
         VALUES ($1, $2, 5, 'invalid', 'legacy', 'stable-hash', 'legacy-token', 1)",
    )
    .bind(course_id)
    .bind(account_id)
    .execute(&database.pool)
    .await;
    assert!(
        invalid_claimed_review.is_err(),
        "fresh schema must reject credentials on an already claimed review"
    );

    database.finish().await;
}
