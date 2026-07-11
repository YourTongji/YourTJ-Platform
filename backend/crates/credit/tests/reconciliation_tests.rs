//! Real-database coverage for read-only credit reconciliation and its admin boundary.

mod helpers;

use axum::body::Body;
use axum::http::{Request, Response, StatusCode};
use helpers::{create_test_account, create_test_app, create_token, mint_to_account, read_json};
use sqlx::PgPool;
use tower::ServiceExt;

async fn staff_account(pool: &PgPool, email: &str, handle: &str, role: &str) -> (i64, String) {
    let account_id = create_test_account(pool, email, handle).await;
    sqlx::query("UPDATE identity.accounts SET role = $2::identity.account_role WHERE id = $1")
        .bind(account_id)
        .bind(role)
        .execute(pool)
        .await
        .expect("set staff role");
    (account_id, create_token(pool, email).await)
}

async fn post_run(
    app: &axum::Router,
    token: &str,
    idempotency_key: &str,
    reason: &str,
) -> Response<Body> {
    app.clone()
        .oneshot(
            Request::builder()
                .uri("/api/v2/admin/credit/reconciliations")
                .method("POST")
                .header("Authorization", format!("Bearer {token}"))
                .header("Idempotency-Key", idempotency_key)
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::json!({ "reason": reason }).to_string()))
                .expect("build reconciliation request"),
        )
        .await
        .expect("reconciliation response")
}

async fn post_resume(
    app: &axum::Router,
    token: &str,
    run_id: &str,
    reason: &str,
) -> Response<Body> {
    app.clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v2/admin/credit/reconciliations/{run_id}/resume"))
                .method("POST")
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::json!({ "reason": reason }).to_string()))
                .expect("build reconciliation resume request"),
        )
        .await
        .expect("reconciliation resume response")
}

#[tokio::test]
async fn dedicated_capability_rejects_user_moderator_suspended_and_revoked_admin() {
    let (pool, app) = create_test_app().await;
    create_test_account(&pool, "recon-user@tongji.edu.cn", "recon-user").await;
    let user_token = create_token(&pool, "recon-user@tongji.edu.cn").await;
    let (_, moderator_token) =
        staff_account(&pool, "recon-mod@tongji.edu.cn", "recon-mod", "mod").await;
    let (revoked_admin_id, revoked_admin_token) =
        staff_account(&pool, "recon-revoked@tongji.edu.cn", "recon-revoked", "admin").await;
    sqlx::query("UPDATE identity.accounts SET legacy_access_revoked_before = now() WHERE id = $1")
        .bind(revoked_admin_id)
        .execute(&pool)
        .await
        .expect("revoke legacy admin access");
    let (suspended_admin_id, suspended_admin_token) =
        staff_account(&pool, "recon-suspended@tongji.edu.cn", "recon-suspended", "admin").await;
    sqlx::query(
        "INSERT INTO identity.sanctions (account_id, kind, reason, issued_by) \
         VALUES ($1, 'suspend', 'test suspension', $1)",
    )
    .bind(suspended_admin_id)
    .execute(&pool)
    .await
    .expect("suspend test admin");

    for (token, key) in [(&user_token, "reconcile-user-1"), (&moderator_token, "reconcile-mod-1")] {
        let response = post_run(&app, token, key, "verify the wallet projection").await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let list_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v2/admin/credit/reconciliations")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .expect("build unauthorized reconciliation list request"),
            )
            .await
            .expect("unauthorized reconciliation list response");
        assert_eq!(list_response.status(), StatusCode::FORBIDDEN);
    }
    let revoked =
        post_run(&app, &revoked_admin_token, "reconcile-revoked-1", "verify the wallet projection")
            .await;
    assert_eq!(revoked.status(), StatusCode::UNAUTHORIZED);
    let suspended = post_run(
        &app,
        &suspended_admin_token,
        "reconcile-suspended-1",
        "verify the wallet projection",
    )
    .await;
    assert_eq!(suspended.status(), StatusCode::FORBIDDEN);
    let run_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM credit.reconciliation_runs")
        .fetch_one(&pool)
        .await
        .expect("count runs");
    assert_eq!(run_count, 0);
}

#[tokio::test]
async fn reconciliation_persists_drift_without_changing_wallet_or_ledger() {
    let (pool, app) = create_test_app().await;
    let (_, admin_token) =
        staff_account(&pool, "recon-admin@tongji.edu.cn", "recon-admin", "admin").await;
    let account_id = create_test_account(&pool, "recon-target@tongji.edu.cn", "recon-target").await;
    let missing_wallet_account_id =
        create_test_account(&pool, "recon-missing@tongji.edu.cn", "recon-missing").await;
    mint_to_account(&pool, account_id, 100).await;
    mint_to_account(&pool, missing_wallet_account_id, 40).await;
    let ledger_count_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM credit.ledger")
        .fetch_one(&pool)
        .await
        .expect("ledger count before");
    sqlx::query("UPDATE credit.wallets SET balance = 91 WHERE account_id = $1")
        .bind(account_id)
        .execute(&pool)
        .await
        .expect("introduce projection drift");
    sqlx::query("DELETE FROM credit.wallets WHERE account_id = $1")
        .bind(missing_wallet_account_id)
        .execute(&pool)
        .await
        .expect("remove wallet projection fixture");

    let response =
        post_run(&app, &admin_token, "reconcile-drift-1", "investigate projection alert").await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let run = read_json(response).await;
    assert_eq!(run["status"], "succeeded");
    assert_eq!(run["ledgerOk"], true);
    assert_eq!(run["driftedWallets"], 2);
    assert_eq!(run["missingWallets"], 1);
    assert_eq!(run["balanceDriftedWallets"], 2);
    assert_eq!(run["totalAbsoluteDrift"], "49");

    let run_id = run["id"].as_str().expect("run id");
    let wallets_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/v2/admin/credit/reconciliations/{run_id}/wallets?driftOnly=true"
                ))
                .header("Authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .expect("wallet result request"),
        )
        .await
        .expect("wallet result response");
    assert_eq!(wallets_response.status(), StatusCode::OK);
    let wallets = read_json(wallets_response).await;
    let findings = wallets["items"].as_array().expect("wallet items");
    assert_eq!(findings.len(), 2);
    let account_id_string = account_id.to_string();
    let finding = findings
        .iter()
        .find(|finding| finding["accountId"].as_str() == Some(account_id_string.as_str()))
        .expect("balance drift finding");
    assert_eq!(finding["accountId"], account_id.to_string());
    assert_eq!(finding["expectedBalance"], "100");
    assert_eq!(finding["actualBalance"], "91");
    assert_eq!(finding["delta"], "-9");
    let missing_wallet_account_id_string = missing_wallet_account_id.to_string();
    let missing_finding = findings
        .iter()
        .find(|finding| {
            finding["accountId"].as_str() == Some(missing_wallet_account_id_string.as_str())
        })
        .expect("missing wallet finding");
    assert_eq!(missing_finding["expectedBalance"], "40");
    assert!(missing_finding["actualBalance"].is_null());
    assert_eq!(missing_finding["delta"], "-40");
    assert_eq!(missing_finding["walletExists"], false);

    let wallet_after: i64 =
        sqlx::query_scalar("SELECT balance FROM credit.wallets WHERE account_id = $1")
            .bind(account_id)
            .fetch_one(&pool)
            .await
            .expect("wallet after reconciliation");
    let ledger_count_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM credit.ledger")
        .fetch_one(&pool)
        .await
        .expect("ledger count after");
    assert_eq!(wallet_after, 91);
    assert_eq!(ledger_count_after, ledger_count_before);
    let missing_wallet_after: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM credit.wallets WHERE account_id = $1)")
            .bind(missing_wallet_account_id)
            .fetch_one(&pool)
            .await
            .expect("missing wallet remains absent");
    assert!(!missing_wallet_after);

    let audit_actions: Vec<String> = sqlx::query_scalar(
        "SELECT action FROM governance.audit_events \
         WHERE target_type = 'credit_reconciliation' AND target_id = $1 ORDER BY id ASC",
    )
    .bind(run_id)
    .fetch_all(&pool)
    .await
    .expect("reconciliation audit events");
    assert_eq!(
        audit_actions,
        vec![
            "credit.reconciliation.requested",
            "credit.reconciliation.started",
            "credit.reconciliation.succeeded",
        ]
    );
}

#[tokio::test]
async fn reconciliation_stops_wallet_comparison_when_ledger_is_tampered() {
    let (pool, app) = create_test_app().await;
    let (_, admin_token) =
        staff_account(&pool, "tamper-admin@tongji.edu.cn", "tamper-admin", "admin").await;
    let account_id =
        create_test_account(&pool, "tamper-target@tongji.edu.cn", "tamper-target").await;
    mint_to_account(&pool, account_id, 25).await;

    sqlx::query("ALTER TABLE credit.ledger DISABLE TRIGGER credit_ledger_reject_mutation")
        .execute(&pool)
        .await
        .expect("disable append-only trigger for tamper fixture");
    sqlx::query("UPDATE credit.ledger SET amount = amount + 1 WHERE seq = 1")
        .execute(&pool)
        .await
        .expect("tamper ledger fixture");
    sqlx::query("ALTER TABLE credit.ledger ENABLE TRIGGER credit_ledger_reject_mutation")
        .execute(&pool)
        .await
        .expect("restore append-only trigger");

    let response =
        post_run(&app, &admin_token, "reconcile-tamper-1", "investigate ledger verification").await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let run = read_json(response).await;
    assert_eq!(run["status"], "succeeded");
    assert_eq!(run["ledgerOk"], false);
    assert_eq!(run["ledgerFailureSeq"], 1);
    assert_eq!(run["walletsChecked"], 0);
    assert_eq!(run["driftedWallets"], 0);

    let wallet_balance: i64 =
        sqlx::query_scalar("SELECT balance FROM credit.wallets WHERE account_id = $1")
            .bind(account_id)
            .fetch_one(&pool)
            .await
            .expect("wallet remains unchanged");
    assert_eq!(wallet_balance, 25);
}

#[tokio::test]
async fn execution_failure_persists_only_a_bounded_error_code() {
    let (pool, app) = create_test_app().await;
    let (_, admin_token) =
        staff_account(&pool, "failure-admin@tongji.edu.cn", "failure-admin", "admin").await;
    let account_id =
        create_test_account(&pool, "failure-target@tongji.edu.cn", "failure-target").await;
    mint_to_account(&pool, account_id, 15).await;
    sqlx::raw_sql(
        "CREATE FUNCTION credit.test_reconciliation_failure() RETURNS trigger \
         LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'fixture-only database detail'; END $$; \
         CREATE TRIGGER test_reconciliation_failure \
         BEFORE INSERT ON credit.reconciliation_wallet_results \
         FOR EACH ROW EXECUTE FUNCTION credit.test_reconciliation_failure();",
    )
    .execute(&pool)
    .await
    .expect("install reconciliation failure fixture");

    let response =
        post_run(&app, &admin_token, "reconcile-failure-1", "exercise failure observability").await;
    sqlx::raw_sql(
        "DROP TRIGGER test_reconciliation_failure ON credit.reconciliation_wallet_results; \
         DROP FUNCTION credit.test_reconciliation_failure();",
    )
    .execute(&pool)
    .await
    .expect("remove reconciliation failure fixture");

    assert_eq!(response.status(), StatusCode::CREATED);
    let run = read_json(response).await;
    assert_eq!(run["status"], "failed");
    assert_eq!(run["errorCode"], "RECONCILIATION_DATABASE_ERROR");
    assert!(!run.to_string().contains("fixture-only database detail"));
    let failed_audit_metadata: serde_json::Value = sqlx::query_scalar(
        "SELECT metadata FROM governance.audit_events \
         WHERE action = 'credit.reconciliation.failed' AND target_id = $1",
    )
    .bind(run["id"].as_str().expect("failed run id"))
    .fetch_one(&pool)
    .await
    .expect("failed reconciliation audit");
    assert_eq!(failed_audit_metadata["errorCode"], "RECONCILIATION_DATABASE_ERROR");
    assert!(!failed_audit_metadata.to_string().contains("fixture-only database detail"));
    let wallet_balance: i64 =
        sqlx::query_scalar("SELECT balance FROM credit.wallets WHERE account_id = $1")
            .bind(account_id)
            .fetch_one(&pool)
            .await
            .expect("wallet remains unchanged after failed reconciliation");
    assert_eq!(wallet_balance, 15);
}

#[tokio::test]
async fn advisory_lock_and_idempotency_prevent_concurrent_runs_and_allow_safe_retry() {
    let (pool, app) = create_test_app().await;
    let (_, admin_token) =
        staff_account(&pool, "lock-admin@tongji.edu.cn", "lock-admin", "admin").await;
    let mut lock_connection = pool.acquire().await.expect("lock connection");
    let locked: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock($1)")
        .bind(credit::reconciliation::RECONCILIATION_ADVISORY_LOCK_ID)
        .fetch_one(&mut *lock_connection)
        .await
        .expect("take reconciliation lock");
    assert!(locked);

    let first =
        post_run(&app, &admin_token, "reconcile-lock-1", "scheduled integrity verification").await;
    assert_eq!(first.status(), StatusCode::CREATED);
    let first = read_json(first).await;
    assert_eq!(first["status"], "queued");
    let run_id = first["id"].as_str().expect("queued run id");

    let competing =
        post_run(&app, &admin_token, "reconcile-lock-2", "competing integrity verification").await;
    assert_eq!(competing.status(), StatusCode::CONFLICT);

    let replay =
        post_run(&app, &admin_token, "reconcile-lock-1", "scheduled integrity verification").await;
    assert_eq!(replay.status(), StatusCode::OK);
    assert_eq!(read_json(replay).await["status"], "queued");

    let changed_payload =
        post_run(&app, &admin_token, "reconcile-lock-1", "a different reason").await;
    assert_eq!(changed_payload.status(), StatusCode::CONFLICT);

    let unlocked: bool = sqlx::query_scalar("SELECT pg_advisory_unlock($1)")
        .bind(credit::reconciliation::RECONCILIATION_ADVISORY_LOCK_ID)
        .fetch_one(&mut *lock_connection)
        .await
        .expect("release reconciliation lock");
    assert!(unlocked);

    let resumed =
        post_resume(&app, &admin_token, run_id, "recover interrupted read-only job").await;
    assert_eq!(resumed.status(), StatusCode::OK);
    assert_eq!(read_json(resumed).await["status"], "succeeded");
    let terminal_replay =
        post_run(&app, &admin_token, "reconcile-lock-1", "scheduled integrity verification").await;
    assert_eq!(terminal_replay.status(), StatusCode::OK);
    assert_eq!(read_json(terminal_replay).await["status"], "succeeded");
    let run_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM credit.reconciliation_runs")
        .fetch_one(&pool)
        .await
        .expect("count idempotent runs");
    assert_eq!(run_count, 1);
    let resume_audits: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM governance.audit_events \
         WHERE action = 'credit.reconciliation.resume_requested' AND target_id = $1",
    )
    .bind(run_id)
    .fetch_one(&pool)
    .await
    .expect("count resume audits");
    assert_eq!(resume_audits, 1);
}
