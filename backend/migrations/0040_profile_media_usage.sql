-- 0040_profile_media_usage.sql — persist the intended profile slot for resumable media review.

ALTER TABLE media.upload_intents
  ADD COLUMN usage TEXT
  CHECK (usage IN ('profile_avatar', 'profile_banner')),
  ADD CONSTRAINT media_upload_intents_profile_usage_kind_check
  CHECK (usage IS NULL OR kind = 'image');

ALTER TABLE media.uploads
  ADD COLUMN usage TEXT
  CHECK (usage IN ('profile_avatar', 'profile_banner')),
  ADD CONSTRAINT media_uploads_profile_usage_kind_check
  CHECK (usage IS NULL OR kind = 'image');

CREATE INDEX media_uploads_owner_usage_created_idx
  ON media.uploads (account_id, usage, created_at DESC, id DESC)
  WHERE usage IS NOT NULL;
