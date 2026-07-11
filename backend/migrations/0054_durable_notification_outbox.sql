-- Durable notification and automatic-achievement delivery.
-- PostgreSQL remains the source of truth; Redis and SSE carry only refresh hints.

CREATE TABLE platform.outbox_events (
  id                    BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  topic                 TEXT NOT NULL,
  source_key            TEXT NOT NULL UNIQUE,
  recipient_account_id  BIGINT NOT NULL REFERENCES identity.accounts(id) ON DELETE CASCADE,
  actor_account_id      BIGINT REFERENCES identity.accounts(id) ON DELETE SET NULL,
  event_type            TEXT NOT NULL,
  payload               JSONB NOT NULL DEFAULT '{}'::jsonb,
  aggregation_key       TEXT,
  state                 TEXT NOT NULL DEFAULT 'queued',
  attempts              SMALLINT NOT NULL DEFAULT 0,
  max_attempts          SMALLINT NOT NULL DEFAULT 8,
  manual_retry_count    SMALLINT NOT NULL DEFAULT 0,
  available_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
  claimed_by            UUID,
  lease_expires_at      TIMESTAMPTZ,
  last_error_code       TEXT,
  completed_at          TIMESTAMPTZ,
  dead_at               TIMESTAMPTZ,
  created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
  CONSTRAINT outbox_events_topic_check
    CHECK (topic IN ('notification', 'achievement_award')),
  CONSTRAINT outbox_events_source_key_check
    CHECK (char_length(source_key) BETWEEN 1 AND 200),
  CONSTRAINT outbox_events_event_type_check
    CHECK (char_length(event_type) BETWEEN 1 AND 80),
  CONSTRAINT outbox_events_payload_check
    CHECK (jsonb_typeof(payload) = 'object'),
  CONSTRAINT outbox_events_aggregation_key_check
    CHECK (aggregation_key IS NULL OR char_length(aggregation_key) BETWEEN 1 AND 160),
  CONSTRAINT outbox_events_state_check
    CHECK (state IN ('queued', 'running', 'succeeded', 'dead', 'cancelled')),
  CONSTRAINT outbox_events_attempts_check
    CHECK (attempts >= 0 AND max_attempts BETWEEN 1 AND 20 AND manual_retry_count >= 0),
  CONSTRAINT outbox_events_lease_check
    CHECK (
      (state = 'running' AND claimed_by IS NOT NULL AND lease_expires_at IS NOT NULL)
      OR (state <> 'running' AND claimed_by IS NULL AND lease_expires_at IS NULL)
    ),
  CONSTRAINT outbox_events_terminal_check
    CHECK (
      (state = 'succeeded' AND completed_at IS NOT NULL AND dead_at IS NULL)
      OR (state = 'dead' AND dead_at IS NOT NULL AND completed_at IS NULL)
      OR (state = 'cancelled' AND completed_at IS NOT NULL AND dead_at IS NULL)
      OR (state IN ('queued', 'running') AND completed_at IS NULL AND dead_at IS NULL)
    )
);

CREATE INDEX outbox_events_claim_idx
  ON platform.outbox_events (available_at, id)
  WHERE state = 'queued';

CREATE INDEX outbox_events_expired_lease_idx
  ON platform.outbox_events (lease_expires_at, id)
  WHERE state = 'running';

CREATE INDEX outbox_events_admin_idx
  ON platform.outbox_events (state, id DESC);

CREATE INDEX outbox_events_retention_idx
  ON platform.outbox_events (completed_at)
  WHERE state IN ('succeeded', 'cancelled');

CREATE INDEX outbox_events_dead_retention_idx
  ON platform.outbox_events (dead_at)
  WHERE state = 'dead';

CREATE TABLE forum.notification_delivery_receipts (
  outbox_event_id  BIGINT PRIMARY KEY,
  notification_id  BIGINT REFERENCES forum.notifications(id) ON DELETE SET NULL,
  outcome          TEXT NOT NULL,
  created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
  CONSTRAINT notification_delivery_receipts_outcome_check
    CHECK (outcome IN ('delivered', 'preference_disabled', 'relationship_hidden',
                       'recipient_unavailable', 'actor_unavailable', 'conversation_muted',
                       'mention_disallowed', 'content_unavailable'))
);

CREATE INDEX notification_delivery_receipts_retention_idx
  ON forum.notification_delivery_receipts (created_at);
