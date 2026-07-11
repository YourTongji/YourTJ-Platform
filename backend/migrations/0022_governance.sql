-- 0022_governance.sql — cross-domain staff audit and account invitations.
-- Append-only: never edit an applied migration.

CREATE SCHEMA IF NOT EXISTS governance;

CREATE TABLE governance.audit_events (
  id               BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  actor_kind       TEXT NOT NULL CHECK (actor_kind IN ('account', 'system', 'service')),
  actor_account_id BIGINT REFERENCES identity.accounts(id),
  actor_role       TEXT,
  action           TEXT NOT NULL,
  target_type      TEXT NOT NULL,
  target_id        TEXT NOT NULL,
  reason           TEXT,
  metadata         JSONB,
  created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK ((actor_kind = 'account') = (actor_account_id IS NOT NULL))
);

CREATE INDEX governance_audit_events_created_idx
  ON governance.audit_events (created_at DESC, id DESC);
CREATE INDEX governance_audit_events_target_idx
  ON governance.audit_events (target_type, target_id, id DESC);

ALTER TABLE identity.accounts
  ADD COLUMN invited_by BIGINT REFERENCES identity.accounts(id),
  ADD COLUMN invited_at TIMESTAMPTZ;

