-- 0033_identity_auth_hardening.sql — purpose-bound email codes and revocable sessions.
-- Append-only: never edit an applied migration.

ALTER TABLE identity.email_codes
  ADD COLUMN id BIGINT GENERATED ALWAYS AS IDENTITY;

ALTER TABLE identity.email_codes
  ADD CONSTRAINT email_codes_pkey PRIMARY KEY (id),
  ADD COLUMN purpose TEXT,
  ADD COLUMN used_at TIMESTAMPTZ,
  ADD COLUMN request_id UUID,
  ADD COLUMN delivery_accepted_at TIMESTAMPTZ;

-- Pre-migration codes have no trustworthy purpose or provider-acceptance fact.
-- Invalidate them instead of guessing and allowing a cross-purpose replay.
UPDATE identity.email_codes
SET purpose = 'login',
    used_at = now(),
    attempts = GREATEST(attempts, 99),
    request_id = md5(id::text || ':' || code_hash)::uuid;

ALTER TABLE identity.email_codes
  ALTER COLUMN purpose SET NOT NULL,
  ALTER COLUMN request_id SET NOT NULL,
  ADD CONSTRAINT email_codes_purpose_check
    CHECK (purpose IN ('login', 'registration', 'password_reset')),
  ADD CONSTRAINT email_codes_request_id_key UNIQUE (request_id);

CREATE INDEX email_codes_email_purpose_live_idx
  ON identity.email_codes (email, purpose, created_at DESC, id DESC)
  WHERE email IS NOT NULL AND used_at IS NULL;

CREATE INDEX email_codes_blind_purpose_live_idx
  ON identity.email_codes (email_blind_index, purpose, created_at DESC, id DESC)
  WHERE email_blind_index IS NOT NULL AND used_at IS NULL;

ALTER TABLE identity.accounts
  ADD COLUMN auth_version BIGINT NOT NULL DEFAULT 1,
  ADD COLUMN legacy_access_revoked_before TIMESTAMPTZ NOT NULL DEFAULT to_timestamp(0),
  ADD CONSTRAINT accounts_auth_version_positive CHECK (auth_version > 0);

ALTER TABLE identity.sessions
  ADD COLUMN family_id UUID,
  ADD COLUMN rotated_from_id BIGINT,
  ADD COLUMN replaced_by_id BIGINT,
  ADD COLUMN last_used_at TIMESTAMPTZ;

UPDATE identity.sessions
SET family_id = md5(id::text || ':' || refresh_hash)::uuid,
    last_used_at = created_at;

ALTER TABLE identity.sessions
  ALTER COLUMN family_id SET NOT NULL,
  ALTER COLUMN family_id SET DEFAULT gen_random_uuid(),
  ALTER COLUMN last_used_at SET NOT NULL,
  ALTER COLUMN last_used_at SET DEFAULT now(),
  ADD CONSTRAINT sessions_rotated_from_fkey
    FOREIGN KEY (rotated_from_id) REFERENCES identity.sessions(id),
  ADD CONSTRAINT sessions_replaced_by_fkey
    FOREIGN KEY (replaced_by_id) REFERENCES identity.sessions(id),
  ADD CONSTRAINT sessions_not_self_rotated
    CHECK (rotated_from_id IS NULL OR rotated_from_id <> id),
  ADD CONSTRAINT sessions_not_self_replaced
    CHECK (replaced_by_id IS NULL OR replaced_by_id <> id);

CREATE INDEX sessions_family_idx ON identity.sessions (family_id, id);
CREATE UNIQUE INDEX sessions_one_successor_idx
  ON identity.sessions (rotated_from_id) WHERE rotated_from_id IS NOT NULL;
CREATE INDEX sessions_active_account_idx
  ON identity.sessions (account_id, last_used_at DESC, id DESC)
  WHERE revoked_at IS NULL;
