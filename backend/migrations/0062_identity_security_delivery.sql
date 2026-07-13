-- Durable identity security facts and retryable email notifications.
-- Delivery rows deliberately reference only an account and a bounded template kind: recipients,
-- subjects, message bodies, verification codes, and provider responses are never persisted here.

ALTER TABLE identity.email_codes ADD COLUMN credential_version BIGINT;

-- Reset codes created before version binding cannot safely survive a concurrent credential change.
UPDATE identity.email_codes
SET used_at = COALESCE(used_at, now()), attempts = GREATEST(attempts, 99)
WHERE purpose = 'password_reset';

ALTER TABLE identity.sessions ADD COLUMN issued_auth_version BIGINT;
UPDATE identity.sessions session
SET issued_auth_version = account.auth_version
FROM identity.accounts account
WHERE account.id = session.account_id;

CREATE TABLE identity.security_events (
  id                  BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  account_id          BIGINT NOT NULL REFERENCES identity.accounts(id),
  event_type          TEXT NOT NULL CHECK (event_type IN (
    'password_set', 'password_changed', 'password_reset', 'refresh_replay_detected'
  )),
  subject_session_id  BIGINT,
  created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
  expires_at          TIMESTAMPTZ NOT NULL DEFAULT (now() + interval '365 days'),
  CHECK (expires_at > created_at)
);

CREATE INDEX security_events_account_idx
  ON identity.security_events (account_id, id DESC);
CREATE INDEX security_events_expiry_idx
  ON identity.security_events (expires_at);
CREATE UNIQUE INDEX security_events_refresh_replay_once_idx
  ON identity.security_events (account_id, subject_session_id)
  WHERE event_type = 'refresh_replay_detected' AND subject_session_id IS NOT NULL;

CREATE FUNCTION identity.reject_live_security_event_mutation()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
  IF TG_OP = 'DELETE' AND OLD.expires_at <= now() THEN
    RETURN OLD;
  END IF;
  RAISE EXCEPTION 'identity.security_events is append-only until retention expiry';
END;
$$;

CREATE TRIGGER security_events_append_only
BEFORE UPDATE OR DELETE ON identity.security_events
FOR EACH ROW EXECUTE FUNCTION identity.reject_live_security_event_mutation();

CREATE TRIGGER security_events_reject_truncate
BEFORE TRUNCATE ON identity.security_events
FOR EACH STATEMENT EXECUTE FUNCTION identity.reject_live_security_event_mutation();

CREATE TABLE identity.email_delivery_jobs (
  id               BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  account_id       BIGINT NOT NULL REFERENCES identity.accounts(id),
  kind             TEXT NOT NULL CHECK (kind IN (
    'password_set', 'password_changed', 'password_reset', 'admin_invitation'
  )),
  status           TEXT NOT NULL DEFAULT 'queued'
                     CHECK (status IN ('queued', 'running', 'succeeded', 'dead')),
  attempts         SMALLINT NOT NULL DEFAULT 0 CHECK (attempts BETWEEN 0 AND 8),
  next_attempt_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  locked_at        TIMESTAMPTZ,
  lease_token      UUID,
  last_error_code  TEXT CHECK (
    last_error_code IS NULL OR last_error_code IN (
      'provider_unavailable', 'identity_unavailable', 'recipient_unavailable', 'template_unavailable',
      'worker_lease_expired'
    )
  ),
  accepted_at      TIMESTAMPTZ,
  created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK ((status = 'running') = (locked_at IS NOT NULL AND lease_token IS NOT NULL)),
  CHECK ((status = 'succeeded') = (accepted_at IS NOT NULL)),
  CHECK (status = 'running' OR (locked_at IS NULL AND lease_token IS NULL)),
  CHECK (status <> 'dead' OR (attempts > 0 AND last_error_code IS NOT NULL))
);

CREATE INDEX email_delivery_jobs_due_idx
  ON identity.email_delivery_jobs (next_attempt_at, id)
  WHERE status = 'queued';
CREATE INDEX email_delivery_jobs_account_idx
  ON identity.email_delivery_jobs (account_id, id DESC);
CREATE INDEX email_delivery_jobs_retention_idx
  ON identity.email_delivery_jobs (status, updated_at)
  WHERE status IN ('succeeded', 'dead');
