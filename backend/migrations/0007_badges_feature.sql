-- 0007_badges_feature.sql — Add featured_at column for thread featuring
--
-- Append-only: never edit an applied migration.
--
-- NOTE: This migration conflicts with 0006_forum_f2_f3.sql which already
-- adds `featured_at`. The IF NOT EXISTS guard keeps this idempotent when
-- both migrations are applied on the same database.

ALTER TABLE forum.threads
  ADD COLUMN IF NOT EXISTS featured_at TIMESTAMPTZ;
