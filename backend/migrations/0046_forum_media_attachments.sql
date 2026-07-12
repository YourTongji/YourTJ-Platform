-- Media-owned, version-aware bindings for clean Forum images.

ALTER TABLE media.upload_intents
  DROP CONSTRAINT upload_intents_usage_check,
  DROP CONSTRAINT media_upload_intents_profile_usage_kind_check,
  ADD CONSTRAINT media_upload_intents_usage_check
    CHECK (usage IN ('profile_avatar', 'profile_banner', 'forum_thread', 'forum_comment')),
  ADD CONSTRAINT media_upload_intents_image_usage_kind_check
    CHECK (usage IS NULL OR kind = 'image');

ALTER TABLE media.uploads
  DROP CONSTRAINT uploads_usage_check,
  DROP CONSTRAINT media_uploads_profile_usage_kind_check,
  ADD COLUMN image_width INTEGER,
  ADD COLUMN image_height INTEGER,
  ADD CONSTRAINT media_uploads_usage_check
    CHECK (usage IN ('profile_avatar', 'profile_banner', 'forum_thread', 'forum_comment')),
  ADD CONSTRAINT media_uploads_image_usage_kind_check
    CHECK (usage IS NULL OR kind = 'image'),
  ADD CONSTRAINT media_uploads_image_dimensions_check CHECK (
    (image_width IS NULL AND image_height IS NULL)
    OR (
      kind = 'image'
      AND image_width BETWEEN 1 AND 20000
      AND image_height BETWEEN 1 AND 20000
    )
  );

CREATE TABLE media.asset_usages (
  id                       BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  asset_id                 BIGINT NOT NULL REFERENCES media.uploads(id),
  owner_account_id         BIGINT NOT NULL REFERENCES identity.accounts(id),
  target_type              TEXT NOT NULL CHECK (target_type IN ('forum_thread', 'forum_comment')),
  target_id                BIGINT NOT NULL CHECK (target_id > 0),
  position                 SMALLINT NOT NULL CHECK (position BETWEEN 0 AND 7),
  alt_text                 TEXT NOT NULL CHECK (
    char_length(btrim(alt_text)) BETWEEN 1 AND 300
    AND alt_text = btrim(alt_text)
  ),
  bound_content_version    BIGINT NOT NULL CHECK (bound_content_version > 0),
  detached_content_version BIGINT CHECK (detached_content_version > 0),
  bound_at                 TIMESTAMPTZ NOT NULL DEFAULT now(),
  detached_at              TIMESTAMPTZ,
  detached_reason          TEXT CHECK (detached_reason IN ('content_edit', 'target_deleted')),
  gc_eligible_at           TIMESTAMPTZ,
  CHECK (
    (detached_at IS NULL AND detached_reason IS NULL AND detached_content_version IS NULL
      AND gc_eligible_at IS NULL)
    OR
    (detached_at IS NOT NULL AND detached_reason IS NOT NULL AND gc_eligible_at IS NOT NULL
      AND (
        (detached_reason = 'content_edit' AND detached_content_version IS NOT NULL)
        OR
        (detached_reason = 'target_deleted' AND detached_content_version IS NULL)
      ))
  )
);

CREATE UNIQUE INDEX media_asset_usages_active_position_idx
  ON media.asset_usages (target_type, target_id, position)
  WHERE detached_at IS NULL;

CREATE UNIQUE INDEX media_asset_usages_active_asset_idx
  ON media.asset_usages (target_type, target_id, asset_id)
  WHERE detached_at IS NULL;

CREATE INDEX media_asset_usages_asset_active_idx
  ON media.asset_usages (asset_id, target_type, target_id)
  WHERE detached_at IS NULL;

CREATE INDEX media_asset_usages_gc_idx
  ON media.asset_usages (gc_eligible_at, asset_id)
  WHERE detached_at IS NOT NULL;

CREATE TABLE media.moderation_preview_grants (
  id                   BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  token_hash           CHAR(64) NOT NULL UNIQUE CHECK (token_hash ~ '^[0-9a-f]{64}$'),
  upload_id            BIGINT NOT NULL REFERENCES media.uploads(id) ON DELETE CASCADE,
  moderator_account_id BIGINT NOT NULL REFERENCES identity.accounts(id),
  reason               TEXT NOT NULL CHECK (
    char_length(btrim(reason)) BETWEEN 3 AND 500
    AND reason = btrim(reason)
  ),
  expires_at           TIMESTAMPTZ NOT NULL,
  consumed_at          TIMESTAMPTZ,
  created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK (expires_at > created_at),
  CHECK (consumed_at IS NULL OR consumed_at >= created_at)
);

CREATE INDEX media_moderation_preview_grants_expiry_idx
  ON media.moderation_preview_grants (expires_at)
  WHERE consumed_at IS NULL;

ALTER TABLE forum.post_revisions
  ADD COLUMN old_content_version BIGINT NOT NULL DEFAULT 1,
  ADD CONSTRAINT forum_post_revisions_content_version_positive
    CHECK (old_content_version > 0);
