-- 0038_credit_reconciliation.sql
-- Persist read-only credit ledger reconciliation runs and per-wallet findings.

CREATE TABLE credit.reconciliation_runs (
  id                       BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  public_id                UUID NOT NULL UNIQUE,
  requested_by             BIGINT NOT NULL REFERENCES identity.accounts(id),
  reason                   TEXT NOT NULL CHECK (char_length(reason) BETWEEN 3 AND 500),
  idempotency_key_hash     CHAR(64) NOT NULL,
  request_fingerprint      CHAR(64) NOT NULL,
  status                   TEXT NOT NULL DEFAULT 'queued'
                           CHECK (status IN ('queued', 'running', 'succeeded', 'failed')),
  ledger_ok                BOOLEAN,
  ledger_latest_seq        BIGINT,
  ledger_latest_hash       TEXT,
  ledger_failure_seq       BIGINT,
  wallets_checked          BIGINT NOT NULL DEFAULT 0 CHECK (wallets_checked >= 0),
  drifted_wallets          BIGINT NOT NULL DEFAULT 0 CHECK (drifted_wallets >= 0),
  missing_wallets          BIGINT NOT NULL DEFAULT 0 CHECK (missing_wallets >= 0),
  balance_drifted_wallets  BIGINT NOT NULL DEFAULT 0 CHECK (balance_drifted_wallets >= 0),
  sequence_drifted_wallets BIGINT NOT NULL DEFAULT 0 CHECK (sequence_drifted_wallets >= 0),
  total_absolute_drift     NUMERIC NOT NULL DEFAULT 0 CHECK (total_absolute_drift >= 0),
  error_code               TEXT,
  created_at               TIMESTAMPTZ NOT NULL DEFAULT now(),
  started_at               TIMESTAMPTZ,
  completed_at             TIMESTAMPTZ,
  UNIQUE (requested_by, idempotency_key_hash),
  CHECK ((status = 'queued') = (started_at IS NULL)),
  CHECK ((status IN ('succeeded', 'failed')) = (completed_at IS NOT NULL)),
  CHECK (status <> 'succeeded' OR ledger_ok IS NOT NULL),
  CHECK (status <> 'failed' OR error_code IS NOT NULL)
);

CREATE UNIQUE INDEX credit_reconciliation_one_active_idx
  ON credit.reconciliation_runs ((status IN ('queued', 'running')))
  WHERE status IN ('queued', 'running');

CREATE INDEX credit_reconciliation_runs_created_idx
  ON credit.reconciliation_runs (id DESC);

CREATE TABLE credit.reconciliation_wallet_results (
  run_id                BIGINT NOT NULL REFERENCES credit.reconciliation_runs(id),
  account_id            BIGINT NOT NULL REFERENCES identity.accounts(id),
  expected_balance      NUMERIC NOT NULL,
  actual_balance        BIGINT,
  delta                 NUMERIC NOT NULL,
  expected_last_seq     BIGINT NOT NULL,
  actual_last_seq       BIGINT,
  wallet_exists         BOOLEAN NOT NULL,
  has_balance_drift     BOOLEAN NOT NULL,
  has_sequence_drift    BOOLEAN NOT NULL,
  PRIMARY KEY (run_id, account_id)
);

CREATE INDEX credit_reconciliation_wallet_drift_idx
  ON credit.reconciliation_wallet_results (run_id, account_id)
  WHERE NOT wallet_exists OR has_balance_drift OR has_sequence_drift;
