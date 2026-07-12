-- 0047_governance_appeals.sql — appeal cases, transition history, and user notices.
-- Append-only: never edit an applied migration.

ALTER TABLE identity.email_codes
  DROP CONSTRAINT email_codes_purpose_check,
  ADD CONSTRAINT email_codes_purpose_check
    CHECK (purpose IN ('login', 'registration', 'password_reset', 'appeal'));

CREATE TABLE governance.appeals (
  id                    BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  original_event_id     BIGINT NOT NULL REFERENCES governance.audit_events(id),
  appellant_account_id  BIGINT NOT NULL REFERENCES identity.accounts(id),
  original_actor_id     BIGINT REFERENCES identity.accounts(id),
  original_action       TEXT NOT NULL CHECK (char_length(original_action) BETWEEN 1 AND 200),
  target_kind           TEXT NOT NULL
    CHECK (target_kind IN ('sanction', 'forum_thread', 'forum_comment', 'review')),
  target_id             TEXT NOT NULL CHECK (char_length(target_id) BETWEEN 1 AND 200),
  disposition_kind      TEXT NOT NULL
    CHECK (disposition_kind IN ('silence', 'suspend', 'hide', 'delete')),
  status                TEXT NOT NULL DEFAULT 'submitted'
    CHECK (status IN ('submitted', 'in_review', 'upheld', 'overturned', 'amended', 'withdrawn')),
  submission_reason     TEXT NOT NULL CHECK (char_length(submission_reason) BETWEEN 3 AND 1000),
  idempotency_key       TEXT NOT NULL CHECK (char_length(idempotency_key) BETWEEN 8 AND 128),
  request_hash          TEXT NOT NULL CHECK (char_length(request_hash) = 64),
  submitted_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
  appealable_until      TIMESTAMPTZ NOT NULL,
  reviewer_account_id   BIGINT REFERENCES identity.accounts(id),
  review_started_at     TIMESTAMPTZ,
  decision_reason       TEXT CHECK (decision_reason IS NULL OR char_length(decision_reason) BETWEEN 3 AND 1000),
  amendment             JSONB,
  decided_at            TIMESTAMPTZ,
  version               BIGINT NOT NULL DEFAULT 1 CHECK (version > 0),
  UNIQUE (original_event_id, appellant_account_id),
  UNIQUE (appellant_account_id, idempotency_key),
  CHECK (appealable_until > submitted_at),
  CHECK (
    (status = 'submitted' AND reviewer_account_id IS NULL AND review_started_at IS NULL
      AND decision_reason IS NULL AND decided_at IS NULL AND amendment IS NULL)
    OR (status = 'in_review' AND reviewer_account_id IS NOT NULL AND review_started_at IS NOT NULL
      AND decision_reason IS NULL AND decided_at IS NULL AND amendment IS NULL)
    OR (status IN ('upheld', 'overturned') AND reviewer_account_id IS NOT NULL
      AND review_started_at IS NOT NULL AND decision_reason IS NOT NULL
      AND decided_at IS NOT NULL AND amendment IS NULL)
    OR (status = 'withdrawn' AND reviewer_account_id IS NULL AND review_started_at IS NULL
      AND decision_reason IS NOT NULL AND decided_at IS NOT NULL AND amendment IS NULL)
    OR (status = 'amended' AND reviewer_account_id IS NOT NULL AND review_started_at IS NOT NULL
      AND decision_reason IS NOT NULL AND decided_at IS NOT NULL AND amendment IS NOT NULL)
  )
);

CREATE INDEX governance_appeals_appellant_idx
  ON governance.appeals (appellant_account_id, id DESC);
CREATE INDEX governance_appeals_queue_idx
  ON governance.appeals (status, submitted_at, id)
  WHERE status IN ('submitted', 'in_review');

CREATE TABLE governance.appeal_events (
  id               BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  appeal_id        BIGINT NOT NULL REFERENCES governance.appeals(id),
  actor_kind       TEXT NOT NULL CHECK (actor_kind IN ('account', 'system')),
  actor_account_id BIGINT REFERENCES identity.accounts(id),
  from_status      TEXT CHECK (from_status IS NULL OR from_status IN (
    'submitted', 'in_review', 'upheld', 'overturned', 'amended', 'withdrawn'
  )),
  to_status        TEXT NOT NULL
    CHECK (to_status IN ('submitted', 'in_review', 'upheld', 'overturned', 'amended', 'withdrawn')),
  reason           TEXT NOT NULL CHECK (char_length(reason) BETWEEN 3 AND 1000),
  metadata         JSONB,
  created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK ((actor_kind = 'account') = (actor_account_id IS NOT NULL)),
  CHECK (
    (from_status IS NULL AND to_status = 'submitted')
    OR (from_status = 'submitted' AND to_status IN ('in_review', 'withdrawn'))
    OR (from_status = 'in_review' AND to_status IN ('upheld', 'overturned', 'amended'))
  )
);

CREATE INDEX governance_appeal_events_case_idx
  ON governance.appeal_events (appeal_id, id);

CREATE TABLE governance.notices (
  id               BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  account_id       BIGINT NOT NULL REFERENCES identity.accounts(id),
  notice_type      TEXT NOT NULL CHECK (notice_type IN (
    'sanction_applied', 'content_restricted', 'appeal_submitted', 'appeal_in_review',
    'appeal_upheld', 'appeal_overturned', 'appeal_amended', 'appeal_withdrawn'
  )),
  dedupe_key       TEXT NOT NULL CHECK (char_length(dedupe_key) BETWEEN 3 AND 200),
  governance_event_id BIGINT REFERENCES governance.audit_events(id),
  appeal_id        BIGINT REFERENCES governance.appeals(id),
  subject_kind     TEXT NOT NULL CHECK (subject_kind IN ('sanction', 'forum_thread', 'forum_comment', 'review', 'appeal')),
  subject_id       TEXT NOT NULL CHECK (char_length(subject_id) BETWEEN 1 AND 200),
  summary          TEXT NOT NULL CHECK (char_length(summary) BETWEEN 1 AND 500),
  read_at          TIMESTAMPTZ,
  created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (account_id, dedupe_key),
  CHECK ((governance_event_id IS NOT NULL) <> (appeal_id IS NOT NULL))
);

CREATE INDEX governance_notices_account_idx
  ON governance.notices (account_id, id DESC);
CREATE INDEX governance_notices_unread_idx
  ON governance.notices (account_id, id DESC) WHERE read_at IS NULL;

CREATE OR REPLACE FUNCTION governance.reject_audit_event_mutation()
RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  RAISE EXCEPTION 'governance.audit_events is append-only';
END;
$$;

CREATE TRIGGER governance_audit_events_no_update
  BEFORE UPDATE OR DELETE ON governance.audit_events
  FOR EACH ROW EXECUTE FUNCTION governance.reject_audit_event_mutation();

CREATE OR REPLACE FUNCTION governance.reject_appeal_event_mutation()
RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  RAISE EXCEPTION 'governance.appeal_events is append-only';
END;
$$;

CREATE TRIGGER governance_appeal_events_no_update
  BEFORE UPDATE OR DELETE ON governance.appeal_events
  FOR EACH ROW EXECUTE FUNCTION governance.reject_appeal_event_mutation();
