-- 0050_activity_and_mention_privacy.sql — authored-activity and mention controls
--
-- Additive, non-PII policy columns. Existing rows backfill to conservative
-- activity visibility and the established open mention default. Old writers
-- leave both columns unchanged because they only update the earlier fields.

ALTER TABLE identity.profile_privacy
  ADD COLUMN activity_visibility TEXT NOT NULL DEFAULT 'only_me'
    CHECK (activity_visibility IN ('public', 'campus', 'only_me')),
  ADD COLUMN mention_policy TEXT NOT NULL DEFAULT 'everyone'
    CHECK (mention_policy IN ('everyone', 'following', 'nobody'));
