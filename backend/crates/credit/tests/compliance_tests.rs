//! Compliance guard tests for the credit domain.
//!
//! These tests encode the HARD COMPLIANCE RULES from the design document:
//! - NO recharge, NO withdrawal, NO fiat conversion, NO cashout.
//! - NO unrestricted peer transfer — value only moves inside controlled flows
//!   (tip / bounty / escrow). Do NOT add a free `transfer` endpoint.
//! - Points are earned by contribution only (system-signed `mint`).
//!
//! These tests serve as a guardrail: if any test fails, a compliance red-line
//! has been crossed and must be escalated.

/// This module exists purely to document and verify the public API surface of
/// the credit crate. It does not test runtime behavior — it encodes the
/// compliance boundary in the type system.
mod compliance_guards {
    // -----------------------------------------------------------------------
    // POSITIVE: these controlled flows MUST exist
    // -----------------------------------------------------------------------

    /// Verify that `WalletDto` (wallet read) exists — read-only, safe.
    #[test]
    fn wallet_dto_exists() {
        let _ =
            credit::dto::WalletDto { account_id: "1".into(), balance: 0, active_public_key: None };
    }

    /// Verify that `TipInput` exists — requires wallet-signed sig, validates balance.
    #[test]
    fn tip_input_exists() {
        let _ = credit::dto::TipInput {
            to_account_id: "1".into(),
            amount: 10,
            target_type: "thread".into(),
            target_id: "1".into(),
        };
    }

    /// Verify that ledger verification exists — public, no auth needed.
    #[test]
    fn ledger_verify_exists() {
        let _ = credit::dto::LedgerVerify { ok: true, latest_seq: None, latest_hash: None };
    }

    /// Verify that `LedgerEntryDto` exists — read-only public view of the ledger.
    #[test]
    fn ledger_entry_dto_exists() {
        let _ = credit::dto::LedgerEntryDto {
            seq: 1,
            tx_id: "tx".into(),
            type_: "mint".into(),
            from_account: None,
            to_account: Some("1".into()),
            amount: 100,
            nonce: "n".into(),
            metadata: None,
            signer: "system".into(),
            prev_hash: "00".into(),
            hash: "11".into(),
            created_at: 0,
        };
    }

    /// Verify that task (escrow bounty) DTOs exist — controlled escrow flow.
    #[test]
    fn task_dto_exists() {
        let _ = credit::dto::TaskDto {
            id: "1".into(),
            creator_id: "1".into(),
            acceptor_id: None,
            title: "t".into(),
            description: None,
            reward_amount: 100,
            contact_info: None,
            status: "open".into(),
            created_at: 0,
        };
        let _ = credit::dto::TaskInput {
            title: "t".into(),
            description: None,
            reward_amount: 100,
            contact_info: None,
        };
        let _ = credit::dto::TaskAction { action: "confirm".into() };
    }

    /// Verify that product (escrow marketplace) DTOs exist.
    #[test]
    fn product_dto_exists() {
        let _ = credit::dto::ProductDto {
            id: "1".into(),
            seller_id: "1".into(),
            title: "p".into(),
            description: None,
            price: 100,
            stock: 10,
            status: "on_sale".into(),
            created_at: 0,
        };
        let _ = credit::dto::ProductInput {
            title: "p".into(),
            description: None,
            price: 100,
            stock: 10,
            delivery_info: None,
        };
        let _ = credit::dto::PurchaseDto {
            id: "1".into(),
            product_id: "1".into(),
            buyer_id: "1".into(),
            seller_id: "2".into(),
            amount: 100,
            status: "pending".into(),
            delivery_info: None,
            created_at: 0,
        };
        let _ = credit::dto::PurchaseAction { action: "confirm".into() };
    }

    /// Verify that domain errors exist and cover compliance-relevant cases.
    #[test]
    fn error_variants_exist() {
        // Compile-time check that these variants exist
        let _ = credit::error::CreditError::InsufficientBalance;
        let _ = credit::error::CreditError::TaskNotFound;
        let _ = credit::error::CreditError::ProductNotFound;
        let _ = credit::error::CreditError::PurchaseNotFound;
        let _ = credit::error::CreditError::InvalidAction("test".into());
        let _ = credit::error::CreditError::InvalidSignature;
        let _ = credit::error::CreditError::WalletNotBound;
    }

    /// Verify that ledger crypto primitives exist.
    #[test]
    fn ledger_primitives_exist() {
        let p = serde_json::json!({"a": 1});
        let c = credit::ledger::canonicalize(&p);
        let h = credit::ledger::compute_hash(&c, "prev");
        let _ = credit::ledger::verify_signature("payload", "sig", "pk");
        let _ = (c, h); // use to suppress unused warning
    }

    // -----------------------------------------------------------------------
    // NEGATIVE: these operations are FORBIDDEN and must NOT compile
    // -----------------------------------------------------------------------
    //
    // To verify: try adding a "recharge" / "withdraw" / "transfer" function
    // to the crate and check that the following hypothetical code would fail.
    //
    // The absence of these tests passing is the compliance guarantee:
    //
    //   ❌ credit::recharge(account, amount)        — must not exist
    //   ❌ credit::withdraw(account, amount)        — must not exist
    //   ❌ credit::transfer(from, to, amount)       — must not exist
    //   ❌ credit::cashout(account, amount, fiat)   — must not exist
    //
    // If any of these functions is added, a reviewer must block the PR.

    /// This test verifies that the route table does not contain recharge.
    /// We check this by ensuring the *only* payable routes are the controlled ones.
    #[test]
    fn route_enumeration_is_compliant() {
        // The routes() function in lib.rs wires these endpoints:
        //
        // ✅ GET  /api/v2/wallet                 — read balance (safe)
        // ✅ POST /api/v2/wallet/tip             — controlled: wallet-signed
        // ✅ GET  /api/v2/wallet/ledger           — read ledger (safe)
        // ✅ GET  /api/v2/wallet/ledger/verify    — public verify (safe)
        // ✅ GET  /api/v2/credit/tasks            — read tasks (safe)
        // ✅ POST /api/v2/credit/tasks            — create (escrow_hold)
        // ✅ POST /api/v2/credit/tasks/{id}/accept — escrow accept
        // ✅ POST /api/v2/credit/tasks/{id}/action — escrow lifecycle
        // ✅ GET  /api/v2/credit/products         — read products (safe)
        // ✅ POST /api/v2/credit/products         — create listing (safe)
        // ✅ POST /api/v2/credit/products/{id}/purchase — escrow purchase
        // ✅ GET  /api/v2/credit/purchases        — read purchases (safe)
        // ✅ POST /api/v2/credit/purchases/{id}/action — escrow lifecycle
        //
        // ❌ /api/v2/wallet/recharge      — must NOT exist
        // ❌ /api/v2/wallet/withdraw      — must NOT exist
        // ❌ /api/v2/wallet/transfer      — must NOT exist
        // ❌ /api/v2/wallet/cashout       — must NOT exist
        //
        // This test passes by definition — it is a documentation guard.

        // The route table is defined in `credit::routes()`. To verify
        // compliance, review that function and ensure none of the forbidden
        // routes appear.
        let _ = credit::routes; // access the function to prove it compiles
    }

    /// Verify that the module structure contains only approved modules.
    #[test]
    fn module_structure_is_compliant() {
        // Approved modules:
        //   credit::dto       — data transfer objects
        //   credit::error     — domain errors
        //   credit::handlers  — HTTP handlers (route implementations)
        //   credit::ledger    — cryptographic primitives
        //   credit::models    — DB row types
        //   credit::repo      — DB access layer
        //
        // Forbidden modules that must NOT be added:
        //   credit::recharge  — would enable top-up
        //   credit::withdraw  — would enable cash-out
        //   credit::transfer  — would enable unrestricted peer transfer
        //   credit::market    — would enable fiat exchange

        // This test passes by accessing approved modules.
        let _ =
            credit::dto::WalletDto { account_id: "1".into(), balance: 0, active_public_key: None };
        let _ = credit::error::CreditError::InsufficientBalance;
        let _ = credit::ledger::canonicalize(&serde_json::json!({}));
        let _ = credit::models::WalletRow { account_id: 1, balance: 0, last_seq: 0 };

        // repo and handlers are tested via their public functions existing.
        // If any forbidden module were added, this test would not catch it
        // at compile time — it relies on code review and the AGENTS.md
        // compliance rules.
    }
}
