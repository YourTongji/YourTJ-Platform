-- 0007_badges_feature.sql — Add featured_at column for thread featuring
--
-- Append-only: never edit an applied migration.

ALTER TABLE forum.threads
  ADD COLUMN featured_at TIMESTAMPTZ;
