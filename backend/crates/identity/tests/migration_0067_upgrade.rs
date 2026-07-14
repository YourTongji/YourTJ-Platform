//! Populated upgrade coverage for the single-active-wallet-key migration 0067.

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
async fn populated_0067_upgrade_freezes_latest_key_and_preserves_history() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL for wallet migration");
    let base_options = PgConnectOptions::from_str(&database_url).expect("parse migration DB URL");
    let admin_options = base_options.clone().database("postgres");
    let mut admin = PgConnection::connect_with(&admin_options)
        .await
        .expect("connect migration database administrator");
    let database_name = format!("yourtj_identity_0067_{}_test", uuid::Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE DATABASE \"{database_name}\""))
        .execute(&mut admin)
        .await
        .expect("create isolated wallet migration database");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(base_options.database(&database_name))
        .await
        .expect("connect isolated wallet migration database");

    migrations_matching(|version| version < 67)
        .run(&pool)
        .await
        .expect("migrate populated fixture through 0066");
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let account_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle) VALUES ($1, $2) RETURNING id",
    )
    .bind(format!("wallet-upgrade-{suffix}@tongji.edu.cn"))
    .bind(format!("wallet-upgrade-{suffix}"))
    .fetch_one(&pool)
    .await
    .expect("insert wallet migration account");

    let historical_key = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
    let latest_lower_key = "AgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI=";
    let latest_winner_key = "AwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwM=";
    sqlx::query(
        "INSERT INTO identity.account_keys (account_id, public_key, created_at) VALUES \
         ($1, $2, '2026-07-13 00:00:00+00'), \
         ($1, $3, '2026-07-14 00:00:00+00'), \
         ($1, $4, '2026-07-14 00:00:00+00')",
    )
    .bind(account_id)
    .bind(historical_key)
    .bind(latest_lower_key)
    .bind(latest_winner_key)
    .execute(&pool)
    .await
    .expect("seed multiple active wallet keys");

    migrations_matching(|version| version == 67)
        .run(&pool)
        .await
        .expect("apply single-active-wallet-key migration 0067");

    let keys: Vec<(String, bool)> = sqlx::query_as(
        "SELECT public_key, revoked_at IS NULL AS is_active \
         FROM identity.account_keys WHERE account_id = $1 ORDER BY public_key",
    )
    .bind(account_id)
    .fetch_all(&pool)
    .await
    .expect("read migrated wallet keys");
    assert_eq!(keys.len(), 3, "migration must retain every historical public key");
    assert_eq!(keys.iter().filter(|(_, is_active)| *is_active).count(), 1);
    assert!(
        keys.iter()
            .any(|(public_key, is_active)| { public_key == latest_winner_key && *is_active }),
        "same-timestamp ties must use descending public-key order"
    );
    assert!(keys
        .iter()
        .any(|(public_key, is_active)| { public_key == historical_key && !*is_active }));

    let second_active =
        sqlx::query("INSERT INTO identity.account_keys (account_id, public_key) VALUES ($1, $2)")
            .bind(account_id)
            .bind("BAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQ=")
            .execute(&pool)
            .await;
    assert!(second_active.is_err(), "database must reject a second active wallet key");

    sqlx::query(
        "INSERT INTO identity.account_keys (account_id, public_key, revoked_at) \
         VALUES ($1, $2, now())",
    )
    .bind(account_id)
    .bind("BQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQU=")
    .execute(&pool)
    .await
    .expect("revoked historical keys remain representable");

    let index_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM pg_indexes \
         WHERE schemaname = 'identity' \
           AND indexname = 'account_keys_one_active_per_account_idx')",
    )
    .fetch_one(&pool)
    .await
    .expect("inspect wallet key index");
    assert!(index_exists);

    pool.close().await;
    sqlx::query(&format!("DROP DATABASE \"{database_name}\""))
        .execute(&mut admin)
        .await
        .expect("drop isolated wallet migration database");
}
