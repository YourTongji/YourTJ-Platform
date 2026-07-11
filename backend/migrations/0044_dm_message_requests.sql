-- 0044_dm_message_requests.sql — explicit, bounded 1:1 message-request lifecycle.
-- Append-only: never edit an applied migration.

ALTER TABLE forum.dm_conversations
  ADD COLUMN request_status TEXT NOT NULL DEFAULT 'accepted'
    CHECK (request_status IN ('accepted', 'pending', 'declined')),
  ADD COLUMN request_sender_id BIGINT REFERENCES identity.accounts(id),
  ADD COLUMN request_recipient_id BIGINT REFERENCES identity.accounts(id),
  ADD COLUMN requested_at TIMESTAMPTZ,
  ADD COLUMN responded_at TIMESTAMPTZ,
  ADD COLUMN request_cooldown_until TIMESTAMPTZ,
  ADD CONSTRAINT dm_conversations_request_direction CHECK (
    request_status = 'accepted'
    OR (
      request_sender_id IS NOT NULL
      AND request_recipient_id IS NOT NULL
      AND request_sender_id <> request_recipient_id
      AND request_sender_id IN (account_low_id, account_high_id)
      AND request_recipient_id IN (account_low_id, account_high_id)
    )
  );

CREATE INDEX dm_conversations_pending_recipient_idx
  ON forum.dm_conversations (request_recipient_id, requested_at DESC, id DESC)
  WHERE request_status = 'pending';

CREATE INDEX dm_conversations_pending_sender_idx
  ON forum.dm_conversations (request_sender_id, requested_at DESC, id DESC)
  WHERE request_status = 'pending';

CREATE TABLE forum.dm_request_idempotency (
  sender_id       BIGINT NOT NULL REFERENCES identity.accounts(id) ON DELETE CASCADE,
  idempotency_key TEXT NOT NULL CHECK (length(idempotency_key) BETWEEN 1 AND 128),
  request_hash    TEXT NOT NULL,
  conversation_id BIGINT NOT NULL REFERENCES forum.dm_conversations(id) ON DELETE CASCADE,
  created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (sender_id, idempotency_key)
);

CREATE INDEX dm_request_idempotency_created_idx
  ON forum.dm_request_idempotency (created_at);

CREATE TABLE forum.dm_request_attempts (
  id              BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  sender_id       BIGINT NOT NULL REFERENCES identity.accounts(id) ON DELETE CASCADE,
  recipient_id    BIGINT NOT NULL REFERENCES identity.accounts(id) ON DELETE CASCADE,
  conversation_id BIGINT NOT NULL REFERENCES forum.dm_conversations(id) ON DELETE CASCADE,
  created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK (sender_id <> recipient_id)
);

CREATE INDEX dm_request_attempts_sender_created_idx
  ON forum.dm_request_attempts (sender_id, created_at DESC);

CREATE FUNCTION forum.enforce_dm_request_delivery()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
DECLARE
  conversation_status TEXT;
  request_sender BIGINT;
BEGIN
  SELECT request_status, request_sender_id
  INTO conversation_status, request_sender
  FROM forum.dm_conversations
  WHERE id = NEW.conversation_id;

  IF conversation_status = 'accepted' THEN
    RETURN NEW;
  END IF;

  IF conversation_status = 'pending'
     AND request_sender = NEW.sender_id
     AND NOT EXISTS (
       SELECT 1 FROM forum.dm_messages WHERE conversation_id = NEW.conversation_id
     ) THEN
    RETURN NEW;
  END IF;

  RAISE EXCEPTION 'message delivery requires an accepted conversation'
    USING ERRCODE = '23514';
END;
$$;

CREATE TRIGGER dm_messages_enforce_request_delivery
BEFORE INSERT ON forum.dm_messages
FOR EACH ROW EXECUTE FUNCTION forum.enforce_dm_request_delivery();
