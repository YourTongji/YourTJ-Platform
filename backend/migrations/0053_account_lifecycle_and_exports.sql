-- Durable account lifecycle, purpose-bound recovery, first-run onboarding, and owner exports.
-- Additive owner tables preserve rolling compatibility; the account enum replacement is required
-- because PostgreSQL cannot safely consume a newly-added enum value in the same migration transaction.

ALTER TABLE identity.accounts ALTER COLUMN status DROP DEFAULT;
ALTER TYPE identity.account_status RENAME TO account_status_legacy;
CREATE TYPE identity.account_status AS ENUM (
  'active', 'suspended', 'deactivated', 'deletion_requested', 'deleted', 'purged'
);
ALTER TABLE identity.accounts
  ALTER COLUMN status TYPE identity.account_status
  USING status::text::identity.account_status,
  ALTER COLUMN status SET DEFAULT 'active'::identity.account_status;
DROP TYPE identity.account_status_legacy;

ALTER TABLE identity.accounts
  ADD COLUMN lifecycle_version BIGINT NOT NULL DEFAULT 1,
  ADD COLUMN credential_version BIGINT NOT NULL DEFAULT 1,
  ADD COLUMN deactivated_at TIMESTAMPTZ,
  ADD COLUMN deletion_requested_at TIMESTAMPTZ,
  ADD COLUMN deletion_recover_until TIMESTAMPTZ,
  ADD COLUMN deleted_at TIMESTAMPTZ,
  ADD COLUMN purge_started_at TIMESTAMPTZ,
  ADD COLUMN purged_at TIMESTAMPTZ,
  ADD COLUMN tombstone_id UUID,
  ADD CONSTRAINT accounts_lifecycle_version_positive CHECK (lifecycle_version > 0),
  ADD CONSTRAINT accounts_credential_version_positive CHECK (credential_version > 0),
  ADD CONSTRAINT accounts_tombstone_unique UNIQUE (tombstone_id),
  ADD CONSTRAINT accounts_deletion_window_valid CHECK (
    deletion_recover_until IS NULL
    OR (
      deletion_requested_at IS NOT NULL
      AND deletion_recover_until > deletion_requested_at
    )
  );

-- Legacy `deleted` rows predate a recovery window. Give them the same bounded safety window from
-- migration time rather than purging identity data immediately during rollout.
UPDATE identity.accounts
SET deletion_requested_at = now(),
    deletion_recover_until = now() + interval '30 days',
    deleted_at = now(),
    lifecycle_version = lifecycle_version + 1
WHERE status = 'deleted';

ALTER TABLE identity.accounts
  ADD CONSTRAINT accounts_lifecycle_shape CHECK (
    (status IN ('active', 'suspended')
      AND deactivated_at IS NULL
      AND deletion_requested_at IS NULL
      AND deletion_recover_until IS NULL
      AND deleted_at IS NULL
      AND purge_started_at IS NULL
      AND purged_at IS NULL
      AND tombstone_id IS NULL)
    OR (status = 'deactivated'
      AND deactivated_at IS NOT NULL
      AND deletion_requested_at IS NULL
      AND deletion_recover_until IS NULL
      AND deleted_at IS NULL
      AND purge_started_at IS NULL
      AND purged_at IS NULL
      AND tombstone_id IS NULL)
    OR (status = 'deletion_requested'
      AND deletion_requested_at IS NOT NULL
      AND deletion_recover_until IS NOT NULL
      AND deleted_at IS NULL
      AND purge_started_at IS NULL
      AND purged_at IS NULL
      AND tombstone_id IS NULL)
    OR (status = 'deleted'
      AND deletion_requested_at IS NOT NULL
      AND deletion_recover_until IS NOT NULL
      AND deleted_at IS NOT NULL
      AND purged_at IS NULL
      AND tombstone_id IS NULL)
    OR (status = 'purged'
      AND deletion_requested_at IS NOT NULL
      AND deletion_recover_until IS NOT NULL
      AND deleted_at IS NOT NULL
      AND purge_started_at IS NOT NULL
      AND purged_at IS NOT NULL
      AND tombstone_id IS NOT NULL)
  );

CREATE INDEX accounts_lifecycle_purge_due_idx
  ON identity.accounts (deletion_recover_until, id)
  WHERE status IN ('deletion_requested', 'deleted');

ALTER TABLE identity.sessions
  ADD COLUMN recent_auth_credential_version BIGINT;

UPDATE identity.sessions session
SET recent_auth_credential_version = account.credential_version
FROM identity.accounts account
WHERE account.id = session.account_id AND session.recent_auth_method = 'password';

ALTER TABLE identity.sessions
  ADD CONSTRAINT sessions_recent_auth_credential_version_shape CHECK (
    (recent_auth_method = 'password' AND recent_auth_credential_version IS NOT NULL)
    OR (recent_auth_method IS DISTINCT FROM 'password' AND recent_auth_credential_version IS NULL)
  );

ALTER TABLE identity.accounts DROP CONSTRAINT accounts_email_storage_check;
ALTER TABLE identity.accounts ADD CONSTRAINT accounts_email_storage_check CHECK (
  status = 'purged'
  OR email IS NOT NULL
  OR (
    email_ciphertext IS NOT NULL
    AND email_key_version IS NOT NULL
    AND email_blind_index IS NOT NULL
  )
);

CREATE TABLE identity.account_onboarding (
  account_id              BIGINT PRIMARY KEY REFERENCES identity.accounts(id) ON DELETE CASCADE,
  required_terms_version  TEXT NOT NULL CHECK (char_length(required_terms_version) BETWEEN 1 AND 64),
  accepted_terms_version  TEXT CHECK (
    accepted_terms_version IS NULL OR char_length(accepted_terms_version) BETWEEN 1 AND 64
  ),
  accepted_at             TIMESTAMPTZ,
  completed_at            TIMESTAMPTZ,
  updated_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK ((accepted_terms_version IS NULL) = (accepted_at IS NULL)),
  CHECK (completed_at IS NULL OR accepted_at IS NOT NULL)
);

INSERT INTO identity.account_onboarding (
  account_id, required_terms_version, accepted_terms_version, accepted_at, completed_at
)
SELECT id, '2026-07-12', 'legacy-v1', created_at, now()
FROM identity.accounts;

CREATE FUNCTION identity.create_account_onboarding()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
  INSERT INTO identity.account_onboarding (
    account_id, required_terms_version, accepted_terms_version, accepted_at, completed_at
  )
  VALUES (NEW.id, '2026-07-12', 'legacy-v1', now(), now())
  ON CONFLICT (account_id) DO NOTHING;
  RETURN NEW;
END;
$$;

CREATE TRIGGER accounts_create_onboarding
AFTER INSERT ON identity.accounts
FOR EACH ROW EXECUTE FUNCTION identity.create_account_onboarding();

ALTER TABLE identity.email_codes
  DROP CONSTRAINT email_codes_purpose_check,
  ADD CONSTRAINT email_codes_purpose_check
    CHECK (purpose IN (
      'login', 'registration', 'password_reset', 'recent_auth', 'appeal', 'recovery'
    ));

CREATE TABLE identity.account_recovery_credentials (
  id                 BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  account_id         BIGINT NOT NULL REFERENCES identity.accounts(id),
  token_hash         CHAR(64) NOT NULL UNIQUE CHECK (token_hash ~ '^[0-9a-f]{64}$'),
  proof_method       TEXT NOT NULL CHECK (proof_method IN ('password', 'email_code', 'session')),
  lifecycle_version  BIGINT NOT NULL CHECK (lifecycle_version > 0),
  expires_at         TIMESTAMPTZ NOT NULL,
  consumed_at        TIMESTAMPTZ,
  created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK (expires_at > created_at),
  CHECK (consumed_at IS NULL OR consumed_at >= created_at)
);

CREATE INDEX account_recovery_credentials_owner_idx
  ON identity.account_recovery_credentials (account_id, id DESC);
CREATE INDEX account_recovery_credentials_expiry_idx
  ON identity.account_recovery_credentials (expires_at);

CREATE TABLE identity.account_lifecycle_events (
  id               BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  account_id       BIGINT NOT NULL REFERENCES identity.accounts(id),
  actor_kind       TEXT NOT NULL CHECK (actor_kind IN ('account', 'system')),
  from_state       TEXT NOT NULL CHECK (from_state IN (
    'active', 'suspended', 'deactivated', 'deletion_requested', 'deleted', 'purged'
  )),
  to_state         TEXT NOT NULL CHECK (to_state IN (
    'active', 'suspended', 'deactivated', 'deletion_requested', 'deleted', 'purged'
  )),
  idempotency_key  TEXT CHECK (
    idempotency_key IS NULL OR char_length(idempotency_key) BETWEEN 8 AND 128
  ),
  request_hash     CHAR(64) CHECK (request_hash IS NULL OR request_hash ~ '^[0-9a-f]{64}$'),
  created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (account_id, idempotency_key),
  CHECK ((idempotency_key IS NULL) = (request_hash IS NULL))
);

CREATE INDEX account_lifecycle_events_owner_idx
  ON identity.account_lifecycle_events (account_id, id DESC);

CREATE FUNCTION identity.reject_account_lifecycle_event_mutation()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
  RAISE EXCEPTION 'identity.account_lifecycle_events is append-only';
END;
$$;

CREATE TRIGGER account_lifecycle_events_append_only
BEFORE UPDATE OR DELETE ON identity.account_lifecycle_events
FOR EACH ROW EXECUTE FUNCTION identity.reject_account_lifecycle_event_mutation();

CREATE TRIGGER account_lifecycle_events_reject_truncate
BEFORE TRUNCATE ON identity.account_lifecycle_events
FOR EACH STATEMENT EXECUTE FUNCTION identity.reject_account_lifecycle_event_mutation();

CREATE TABLE identity.account_lifecycle_jobs (
  id               BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  account_id       BIGINT NOT NULL REFERENCES identity.accounts(id),
  job_type         TEXT NOT NULL CHECK (job_type IN ('mark_deleted', 'purge')),
  status           TEXT NOT NULL DEFAULT 'queued'
    CHECK (status IN ('queued', 'running', 'succeeded', 'failed')),
  attempts         SMALLINT NOT NULL DEFAULT 0 CHECK (attempts BETWEEN 0 AND 20),
  next_attempt_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  locked_at        TIMESTAMPTZ,
  last_error_code  TEXT CHECK (last_error_code IS NULL OR char_length(last_error_code) <= 80),
  created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (account_id, job_type),
  CHECK ((status = 'running') = (locked_at IS NOT NULL))
);

CREATE INDEX account_lifecycle_jobs_claim_idx
  ON identity.account_lifecycle_jobs (next_attempt_at, id)
  WHERE status IN ('queued', 'failed');

INSERT INTO identity.account_lifecycle_jobs (account_id, job_type, next_attempt_at)
SELECT id, 'purge', deletion_recover_until
FROM identity.accounts
WHERE status = 'deleted' AND deletion_recover_until IS NOT NULL
ON CONFLICT (account_id, job_type) DO NOTHING;

CREATE TABLE identity.account_export_jobs (
  id               UUID PRIMARY KEY,
  account_id       BIGINT NOT NULL REFERENCES identity.accounts(id),
  idempotency_hash CHAR(64) NOT NULL CHECK (idempotency_hash ~ '^[0-9a-f]{64}$'),
  status           TEXT NOT NULL DEFAULT 'queued'
    CHECK (status IN ('queued', 'running', 'ready', 'failed', 'expired')),
  attempts         SMALLINT NOT NULL DEFAULT 0 CHECK (attempts BETWEEN 0 AND 10),
  artifact         JSONB,
  error_code       TEXT CHECK (error_code IS NULL OR char_length(error_code) <= 80),
  locked_at        TIMESTAMPTZ,
  next_attempt_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  downloaded_at    TIMESTAMPTZ,
  created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
  expires_at       TIMESTAMPTZ NOT NULL,
  UNIQUE (account_id, idempotency_hash),
  CHECK (expires_at > created_at),
  CHECK ((status = 'running') = (locked_at IS NOT NULL)),
  CHECK ((status = 'ready') = (artifact IS NOT NULL))
);

CREATE INDEX account_export_jobs_claim_idx
  ON identity.account_export_jobs (next_attempt_at, created_at, id)
  WHERE status IN ('queued', 'failed');
CREATE INDEX account_export_jobs_expiry_idx
  ON identity.account_export_jobs (expires_at, id)
  WHERE status IN ('queued', 'running', 'ready', 'failed');

CREATE TABLE identity.account_export_download_grants (
  id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  export_id   UUID NOT NULL REFERENCES identity.account_export_jobs(id) ON DELETE CASCADE,
  account_id  BIGINT NOT NULL REFERENCES identity.accounts(id),
  token_hash  CHAR(64) NOT NULL UNIQUE CHECK (token_hash ~ '^[0-9a-f]{64}$'),
  expires_at  TIMESTAMPTZ NOT NULL,
  consumed_at TIMESTAMPTZ,
  created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK (expires_at > created_at),
  CHECK (consumed_at IS NULL OR consumed_at >= created_at)
);

CREATE INDEX account_export_download_grants_expiry_idx
  ON identity.account_export_download_grants (expires_at);
