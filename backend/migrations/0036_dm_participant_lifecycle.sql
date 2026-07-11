-- 0036_dm_participant_lifecycle.sql — participant-local mute and inbox lifecycle support.
-- Append-only: never edit an applied migration.

ALTER TABLE forum.dm_participants
  ADD COLUMN muted_at TIMESTAMPTZ;

CREATE INDEX dm_participants_lifecycle_idx
  ON forum.dm_participants (account_id, deleted_at, archived_at, conversation_id);
