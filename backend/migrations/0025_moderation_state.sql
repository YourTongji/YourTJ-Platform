-- 0025_moderation_state.sql — distinguish automated moderation transitions.
-- Append-only: never edit an applied migration.

ALTER TABLE identity.sanctions
  ALTER COLUMN issued_by DROP NOT NULL;

ALTER TABLE forum.flags
  ADD COLUMN auto_hidden_at TIMESTAMPTZ,
  ADD COLUMN resolution_note TEXT;
