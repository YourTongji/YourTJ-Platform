-- 0021_dm_moderation.sql — canonical 1:1 conversations, read state, and DM reports.
-- Append-only: never edit an applied migration.

ALTER TABLE forum.dm_conversations
  ADD COLUMN account_low_id BIGINT REFERENCES identity.accounts(id),
  ADD COLUMN account_high_id BIGINT REFERENCES identity.accounts(id);

-- Collapse any legacy duplicate 1:1 conversations before adding the canonical
-- pair constraint. Messages move to the oldest conversation for the pair.
WITH conversation_pairs AS (
  SELECT conversation_id,
         MIN(account_id) AS account_low_id,
         MAX(account_id) AS account_high_id
  FROM forum.dm_participants
  GROUP BY conversation_id
  HAVING COUNT(*) = 2
), ranked_pairs AS (
  SELECT conversation_id,
         MIN(conversation_id) OVER (PARTITION BY account_low_id, account_high_id) AS keeper_id
  FROM conversation_pairs
)
UPDATE forum.dm_messages AS message
SET conversation_id = ranked.keeper_id
FROM ranked_pairs AS ranked
WHERE message.conversation_id = ranked.conversation_id
  AND ranked.conversation_id <> ranked.keeper_id;

WITH conversation_pairs AS (
  SELECT conversation_id,
         MIN(account_id) AS account_low_id,
         MAX(account_id) AS account_high_id
  FROM forum.dm_participants
  GROUP BY conversation_id
  HAVING COUNT(*) = 2
), ranked_pairs AS (
  SELECT conversation_id,
         MIN(conversation_id) OVER (PARTITION BY account_low_id, account_high_id) AS keeper_id
  FROM conversation_pairs
)
DELETE FROM forum.dm_participants AS participant
USING ranked_pairs AS ranked
WHERE participant.conversation_id = ranked.conversation_id
  AND ranked.conversation_id <> ranked.keeper_id;

DELETE FROM forum.dm_conversations AS conversation
WHERE NOT EXISTS (
  SELECT 1
  FROM forum.dm_participants AS participant
  WHERE participant.conversation_id = conversation.id
);

UPDATE forum.dm_conversations AS conversation
SET account_low_id = pair.account_low_id,
    account_high_id = pair.account_high_id
FROM (
  SELECT conversation_id,
         MIN(account_id) AS account_low_id,
         MAX(account_id) AS account_high_id
  FROM forum.dm_participants
  GROUP BY conversation_id
  HAVING COUNT(*) = 2
) AS pair
WHERE conversation.id = pair.conversation_id;

ALTER TABLE forum.dm_conversations
  ALTER COLUMN account_low_id SET NOT NULL,
  ALTER COLUMN account_high_id SET NOT NULL,
  ADD CONSTRAINT dm_conversations_canonical_pair CHECK (account_low_id < account_high_id),
  ADD CONSTRAINT dm_conversations_unique_pair UNIQUE (account_low_id, account_high_id);

ALTER TABLE forum.dm_participants
  ADD COLUMN last_read_message_id BIGINT REFERENCES forum.dm_messages(id) ON DELETE SET NULL,
  ADD COLUMN archived_at TIMESTAMPTZ,
  ADD COLUMN deleted_at TIMESTAMPTZ;

ALTER TABLE forum.dm_messages
  ADD CONSTRAINT dm_messages_body_length
  CHECK (char_length(body) BETWEEN 1 AND 16000) NOT VALID;

CREATE TABLE forum.dm_message_reports (
  id              BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  message_id      BIGINT NOT NULL REFERENCES forum.dm_messages(id),
  conversation_id BIGINT NOT NULL REFERENCES forum.dm_conversations(id),
  reported_by     BIGINT NOT NULL REFERENCES identity.accounts(id),
  reason          TEXT NOT NULL
                  CHECK (reason IN ('spam', 'abuse', 'harassment', 'fraud', 'illegal', 'other')),
  note            TEXT,
  status          TEXT NOT NULL DEFAULT 'open'
                  CHECK (status IN ('open', 'upheld', 'rejected')),
  handled_by      BIGINT REFERENCES identity.accounts(id),
  handled_at      TIMESTAMPTZ,
  resolution_note TEXT,
  created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (message_id, reported_by)
);

CREATE INDEX dm_participants_inbox_idx
  ON forum.dm_participants (account_id, deleted_at, conversation_id);
CREATE INDEX dm_message_reports_queue_idx
  ON forum.dm_message_reports (status, id);
