-- 0041_forum_draft_versions.sql — optimistic concurrency for cross-device drafts.

ALTER TABLE forum.drafts
  ADD COLUMN version BIGINT NOT NULL DEFAULT 1,
  ADD CONSTRAINT drafts_version_positive CHECK (version > 0);
