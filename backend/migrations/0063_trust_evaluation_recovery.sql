-- Make daily trust evaluation resumable in bounded batches and isolate a corrupt
-- account from every later account in the same Shanghai-day run.

ALTER TABLE activity.trust_evaluation_runs
  DROP CONSTRAINT trust_evaluation_runs_status_check;

ALTER TABLE activity.trust_evaluation_runs
  ADD CONSTRAINT trust_evaluation_runs_status_check
    CHECK (status IN ('queued', 'running', 'completed', 'failed', 'dead')),
  ADD COLUMN cursor_account_id BIGINT NOT NULL DEFAULT 0 CHECK (cursor_account_id >= 0),
  ADD COLUMN next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  ADD COLUMN failed_count INT NOT NULL DEFAULT 0 CHECK (failed_count >= 0);

CREATE INDEX trust_evaluation_runs_due_idx
  ON activity.trust_evaluation_runs (next_attempt_at, activity_date)
  WHERE status IN ('queued', 'failed', 'running');

CREATE TABLE activity.trust_evaluation_failures (
  activity_date   DATE NOT NULL REFERENCES activity.trust_evaluation_runs(activity_date)
                    ON DELETE CASCADE,
  account_id      BIGINT NOT NULL REFERENCES identity.accounts(id) ON DELETE CASCADE,
  attempts        SMALLINT NOT NULL DEFAULT 1 CHECK (attempts BETWEEN 1 AND 8),
  error_code      TEXT NOT NULL CHECK (error_code IN ('account_evaluation_failed')),
  first_failed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  last_failed_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (activity_date, account_id)
);

CREATE INDEX trust_evaluation_failures_account_idx
  ON activity.trust_evaluation_failures (account_id, activity_date DESC);
