-- Fence account lifecycle workers with an explicit per-claim lease token.

ALTER TABLE identity.account_lifecycle_jobs
  ADD COLUMN lease_token UUID;

-- Operators must drain and stop every token-unaware lifecycle worker before applying this
-- migration. Once that prerequisite is satisfied, return any crash-remnant running row to the
-- retryable state before the new invariant makes tokenless running jobs invalid.
UPDATE identity.account_lifecycle_jobs
SET status = 'failed',
    locked_at = NULL,
    lease_token = NULL,
    next_attempt_at = now(),
    last_error_code = 'lease_fencing_migration_recovery',
    updated_at = now()
WHERE status = 'running';

ALTER TABLE identity.account_lifecycle_jobs
  DROP CONSTRAINT account_lifecycle_jobs_check,
  ADD CONSTRAINT account_lifecycle_jobs_running_lease CHECK (
    (status = 'running' AND locked_at IS NOT NULL AND lease_token IS NOT NULL)
    OR
    (status <> 'running' AND locked_at IS NULL AND lease_token IS NULL)
  );

CREATE UNIQUE INDEX account_lifecycle_jobs_lease_token_idx
  ON identity.account_lifecycle_jobs (lease_token)
  WHERE lease_token IS NOT NULL;
