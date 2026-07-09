-- 0007_badges_feature.sql — Add featured_at column for thread featuring
--
-- Append-only: never edit an applied migration.
--
-- NOTE: `forum.threads.featured_at` is already added by 0006_forum_f2_f3.sql
-- (§2.9). On a fresh database 0006 runs first, so a plain `ADD COLUMN` here
-- aborts with "column already exists". The `IF NOT EXISTS` guard keeps this
-- migration idempotent regardless of which migration created the column.

ALTER TABLE forum.threads
  ADD COLUMN IF NOT EXISTS featured_at TIMESTAMPTZ;
