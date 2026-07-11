-- Track non-versioned asset bindings and make retention-aware object GC durable.

ALTER TABLE media.uploads
  ADD COLUMN cleaned_at TIMESTAMPTZ,
  ADD COLUMN redacted_at TIMESTAMPTZ,
  ADD COLUMN is_cleanup_tombstone BOOLEAN NOT NULL DEFAULT FALSE;

-- Historical approvals do not record their decision time. Start their orphan clock at rollout
-- rather than treating old upload creation time as approval time.
UPDATE media.uploads SET cleaned_at = now() WHERE status = 'clean';

CREATE FUNCTION media.stamp_upload_cleaned_at()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
  IF NEW.status = 'clean' AND NEW.cleaned_at IS NULL THEN
    NEW.cleaned_at := now();
  END IF;
  RETURN NEW;
END;
$$;

CREATE TRIGGER media_upload_cleaned_at_on_insert
BEFORE INSERT ON media.uploads
FOR EACH ROW EXECUTE FUNCTION media.stamp_upload_cleaned_at();

CREATE TRIGGER media_upload_cleaned_at_on_status_update
BEFORE UPDATE OF status ON media.uploads
FOR EACH ROW EXECUTE FUNCTION media.stamp_upload_cleaned_at();

ALTER TABLE media.uploads
  ADD CONSTRAINT media_uploads_cleaned_at_required
    CHECK (status <> 'clean' OR cleaned_at IS NOT NULL),
  ADD CONSTRAINT media_uploads_redaction_state
    CHECK (redacted_at IS NULL OR status = 'blocked'),
  ADD CONSTRAINT media_uploads_cleanup_tombstone_state
    CHECK (NOT is_cleanup_tombstone OR status IN ('quarantined', 'blocked'));

ALTER TABLE media.upload_intents
  ADD COLUMN revoked_at TIMESTAMPTZ,
  ADD COLUMN callback_token_hash BYTEA;

-- Callback tokens are bearer secrets. Preserve outstanding callbacks by hashing the existing
-- values before removing the plaintext column; the new API verifies only the digest.
UPDATE media.upload_intents
SET callback_token_hash = sha256(convert_to(callback_token, 'UTF8'));

ALTER TABLE media.upload_intents
  ALTER COLUMN callback_token_hash SET NOT NULL,
  ADD CONSTRAINT media_upload_intents_callback_token_hash_size
    CHECK (octet_length(callback_token_hash) = 32),
  ADD CONSTRAINT media_upload_intents_callback_token_hash_unique
    UNIQUE (callback_token_hash),
  DROP COLUMN callback_token;

CREATE INDEX media_upload_intents_orphan_cleanup_idx
  ON media.upload_intents (expires_at, id)
  WHERE upload_id IS NULL;

-- PostgreSQL is the fail-closed abuse-control authority when Redis is absent. Attempts survive
-- orphan cleanup long enough to enforce a rolling daily provider budget, then are purged.
CREATE TABLE media.upload_credential_attempts (
  id               BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  account_id       BIGINT NOT NULL REFERENCES identity.accounts(id) ON DELETE CASCADE,
  reserved_bytes   BIGINT NOT NULL CHECK (reserved_bytes BETWEEN 1 AND 20971520),
  created_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX media_upload_credential_attempts_account_idx
  ON media.upload_credential_attempts (account_id, created_at DESC);

CREATE INDEX media_upload_credential_attempts_cleanup_idx
  ON media.upload_credential_attempts (created_at, id);

CREATE TABLE media.asset_bindings (
  id                BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  asset_id          BIGINT NOT NULL REFERENCES media.uploads(id),
  owner_account_id  BIGINT NOT NULL REFERENCES identity.accounts(id),
  target_type       TEXT NOT NULL CHECK (
    target_type IN ('profile_avatar', 'profile_banner', 'platform_promotion')
  ),
  target_id         BIGINT NOT NULL CHECK (target_id > 0),
  bound_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
  detached_at       TIMESTAMPTZ,
  detached_reason   TEXT CHECK (
    detached_reason IN ('replaced', 'cleared', 'archived', 'account_purge')
  ),
  gc_eligible_at    TIMESTAMPTZ,
  CHECK (
    (detached_at IS NULL AND detached_reason IS NULL AND gc_eligible_at IS NULL)
    OR
    (detached_at IS NOT NULL AND detached_reason IS NOT NULL AND gc_eligible_at IS NOT NULL)
  )
);

CREATE UNIQUE INDEX media_asset_bindings_active_target_idx
  ON media.asset_bindings (target_type, target_id)
  WHERE detached_at IS NULL;

CREATE INDEX media_asset_bindings_asset_active_idx
  ON media.asset_bindings (asset_id, target_type, target_id)
  WHERE detached_at IS NULL;

CREATE INDEX media_asset_bindings_gc_idx
  ON media.asset_bindings (gc_eligible_at, asset_id)
  WHERE detached_at IS NOT NULL;

CREATE TABLE media.draft_asset_references (
  account_id   BIGINT NOT NULL,
  draft_key    TEXT NOT NULL,
  asset_id     BIGINT NOT NULL REFERENCES media.uploads(id),
  target_type  TEXT NOT NULL CHECK (target_type IN ('forum_thread', 'forum_comment')),
  position     SMALLINT NOT NULL CHECK (position BETWEEN 0 AND 7),
  referenced_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (account_id, draft_key, asset_id),
  UNIQUE (account_id, draft_key, position),
  FOREIGN KEY (account_id, draft_key)
    REFERENCES forum.drafts(account_id, draft_key) ON DELETE CASCADE
);

CREATE INDEX media_draft_asset_references_asset_idx
  ON media.draft_asset_references (asset_id, account_id, draft_key);

-- Database triggers keep facts complete while older API instances drain. New writers also write
-- these facts explicitly; the helper is idempotent and shares their advisory-lock namespace.
CREATE FUNCTION media.replace_nonversioned_asset_binding(
  binding_type TEXT,
  binding_target_id BIGINT,
  desired_asset_id BIGINT,
  desired_owner_account_id BIGINT,
  detach_reason TEXT
)
RETURNS VOID
LANGUAGE plpgsql
AS $$
DECLARE
  required_usage TEXT;
  asset_is_valid BOOLEAN;
BEGIN
  IF binding_type NOT IN ('profile_avatar', 'profile_banner', 'platform_promotion')
     OR binding_target_id <= 0
     OR detach_reason NOT IN ('replaced', 'cleared', 'archived', 'account_purge') THEN
    RAISE EXCEPTION 'invalid non-versioned media binding'
      USING ERRCODE = 'check_violation';
  END IF;

  PERFORM pg_advisory_xact_lock(
    hashtextextended('media-binding:' || binding_type || ':' || binding_target_id, 0)
  );
  required_usage := CASE binding_type
    WHEN 'profile_avatar' THEN 'profile_avatar'
    WHEN 'profile_banner' THEN 'profile_banner'
  END;
  IF desired_asset_id IS NOT NULL THEN
    SELECT upload.account_id = desired_owner_account_id
           AND upload.kind = 'image'
           AND upload.status = 'clean'
           AND (required_usage IS NULL OR upload.usage = required_usage)
      INTO asset_is_valid
    FROM media.uploads upload
    WHERE upload.id = desired_asset_id
    FOR SHARE;
    IF asset_is_valid IS DISTINCT FROM TRUE THEN
      RAISE EXCEPTION 'invalid asset for non-versioned media binding'
        USING ERRCODE = 'foreign_key_violation';
    END IF;
  END IF;

  UPDATE media.asset_bindings binding
  SET detached_at = now(), detached_reason = detach_reason,
      gc_eligible_at = CASE WHEN detach_reason = 'account_purge'
                            THEN now() ELSE now() + interval '30 days' END
  WHERE binding.target_type = binding_type
    AND binding.target_id = binding_target_id
    AND binding.detached_at IS NULL
    AND binding.asset_id IS DISTINCT FROM desired_asset_id;

  IF detach_reason = 'account_purge' THEN
    UPDATE media.asset_bindings binding
    SET gc_eligible_at = now()
    WHERE binding.target_type = binding_type
      AND binding.target_id = binding_target_id
      AND binding.detached_at IS NOT NULL
      AND binding.gc_eligible_at > now();
  END IF;

  IF desired_asset_id IS NOT NULL AND NOT EXISTS (
    SELECT 1 FROM media.asset_bindings binding
    WHERE binding.target_type = binding_type
      AND binding.target_id = binding_target_id
      AND binding.asset_id = desired_asset_id
      AND binding.detached_at IS NULL
  ) THEN
    INSERT INTO media.asset_bindings
      (asset_id, owner_account_id, target_type, target_id)
    VALUES
      (desired_asset_id, desired_owner_account_id, binding_type, binding_target_id);
  END IF;
END;
$$;

CREATE FUNCTION media.sync_profile_asset_bindings_from_source()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
  IF TG_OP = 'DELETE' THEN
    PERFORM media.replace_nonversioned_asset_binding(
      'profile_avatar', OLD.account_id, NULL, OLD.account_id, 'account_purge'
    );
    PERFORM media.replace_nonversioned_asset_binding(
      'profile_banner', OLD.account_id, NULL, OLD.account_id, 'account_purge'
    );
    RETURN OLD;
  END IF;
  IF TG_OP = 'INSERT' OR NEW.avatar_asset_id IS DISTINCT FROM OLD.avatar_asset_id THEN
    PERFORM media.replace_nonversioned_asset_binding(
      'profile_avatar', NEW.account_id, NEW.avatar_asset_id, NEW.account_id,
      CASE WHEN NEW.avatar_asset_id IS NULL THEN 'cleared' ELSE 'replaced' END
    );
  END IF;
  IF TG_OP = 'INSERT' OR NEW.banner_asset_id IS DISTINCT FROM OLD.banner_asset_id THEN
    PERFORM media.replace_nonversioned_asset_binding(
      'profile_banner', NEW.account_id, NEW.banner_asset_id, NEW.account_id,
      CASE WHEN NEW.banner_asset_id IS NULL THEN 'cleared' ELSE 'replaced' END
    );
  END IF;
  RETURN NEW;
END;
$$;

CREATE TRIGGER identity_profile_media_binding_sync
AFTER INSERT OR UPDATE OF avatar_asset_id, banner_asset_id ON identity.profiles
FOR EACH ROW EXECUTE FUNCTION media.sync_profile_asset_bindings_from_source();

CREATE TRIGGER identity_profile_media_binding_delete_sync
AFTER DELETE ON identity.profiles
FOR EACH ROW EXECUTE FUNCTION media.sync_profile_asset_bindings_from_source();

CREATE FUNCTION media.sync_promotion_asset_binding_from_source()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
DECLARE
  desired_asset_id BIGINT;
  desired_owner_account_id BIGINT;
  detach_reason TEXT;
BEGIN
  desired_asset_id := CASE WHEN NEW.status = 'archived' THEN NULL ELSE NEW.asset_id END;
  IF desired_asset_id IS NOT NULL THEN
    SELECT account_id INTO desired_owner_account_id
    FROM media.uploads WHERE id = desired_asset_id;
  END IF;
  detach_reason := CASE
    WHEN NEW.status = 'archived' THEN 'archived'
    WHEN desired_asset_id IS NULL THEN 'cleared'
    ELSE 'replaced'
  END;
  PERFORM media.replace_nonversioned_asset_binding(
    'platform_promotion', NEW.id, desired_asset_id,
    COALESCE(desired_owner_account_id, NEW.updated_by, NEW.created_by, 0), detach_reason
  );
  RETURN NEW;
END;
$$;

CREATE TRIGGER platform_promotion_media_binding_sync
AFTER INSERT OR UPDATE OF asset_id, status ON platform.promotions
FOR EACH ROW EXECUTE FUNCTION media.sync_promotion_asset_binding_from_source();

CREATE FUNCTION media.sync_draft_asset_references_from_source()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
DECLARE
  raw_reference_count INTEGER;
  unique_reference_count INTEGER;
  valid_reference_count INTEGER;
BEGIN
  IF NEW.payload->>'kind' NOT IN ('thread', 'comment') THEN
    RAISE EXCEPTION 'invalid forum draft media target'
      USING ERRCODE = 'check_violation';
  END IF;
  SELECT count(*)::integer, count(DISTINCT asset.value)::integer
    INTO raw_reference_count, unique_reference_count
  FROM jsonb_array_elements_text(
    CASE
      WHEN jsonb_typeof(NEW.payload->'attachmentAssetIds') = 'array'
        THEN NEW.payload->'attachmentAssetIds'
      ELSE '[]'::jsonb
    END
  ) AS asset(value);
  IF raw_reference_count > (CASE NEW.payload->>'kind' WHEN 'thread' THEN 8 ELSE 4 END)
     OR unique_reference_count <> raw_reference_count THEN
    RAISE EXCEPTION 'invalid forum draft media reference set'
      USING ERRCODE = 'check_violation';
  END IF;

  PERFORM upload.id
  FROM jsonb_array_elements_text(
    CASE
      WHEN jsonb_typeof(NEW.payload->'attachmentAssetIds') = 'array'
        THEN NEW.payload->'attachmentAssetIds'
      ELSE '[]'::jsonb
    END
  ) AS asset(value)
  JOIN media.uploads upload
    ON asset.value = upload.id::text
   AND upload.account_id = NEW.account_id
   AND upload.kind = 'image'
   AND upload.status IN ('pending', 'clean', 'blocked')
   AND upload.usage = CASE NEW.payload->>'kind'
         WHEN 'thread' THEN 'forum_thread'
         WHEN 'comment' THEN 'forum_comment'
       END
  ORDER BY upload.id
  FOR SHARE OF upload;

  SELECT count(*)::integer INTO valid_reference_count
  FROM jsonb_array_elements_text(
    CASE
      WHEN jsonb_typeof(NEW.payload->'attachmentAssetIds') = 'array'
        THEN NEW.payload->'attachmentAssetIds'
      ELSE '[]'::jsonb
    END
  ) AS asset(value)
  JOIN media.uploads upload
    ON asset.value = upload.id::text
   AND upload.account_id = NEW.account_id
   AND upload.kind = 'image'
   AND upload.status IN ('pending', 'clean', 'blocked')
   AND upload.usage = CASE NEW.payload->>'kind'
         WHEN 'thread' THEN 'forum_thread'
         WHEN 'comment' THEN 'forum_comment'
       END;
  IF valid_reference_count <> raw_reference_count THEN
    RAISE EXCEPTION 'forum draft references an unavailable media asset'
      USING ERRCODE = 'foreign_key_violation';
  END IF;

  DELETE FROM media.draft_asset_references
  WHERE account_id = NEW.account_id AND draft_key = NEW.draft_key;

  INSERT INTO media.draft_asset_references
    (account_id, draft_key, asset_id, target_type, position)
  SELECT NEW.account_id,
         NEW.draft_key,
         upload.id,
         CASE NEW.payload->>'kind'
           WHEN 'thread' THEN 'forum_thread'
           WHEN 'comment' THEN 'forum_comment'
         END,
         (asset.ordinality - 1)::smallint
  FROM jsonb_array_elements_text(
    CASE
      WHEN jsonb_typeof(NEW.payload->'attachmentAssetIds') = 'array'
        THEN NEW.payload->'attachmentAssetIds'
      ELSE '[]'::jsonb
    END
  ) WITH ORDINALITY AS asset(value, ordinality)
  JOIN media.uploads upload
    ON asset.value = upload.id::text
   AND upload.account_id = NEW.account_id
   AND upload.kind = 'image'
   AND upload.status IN ('pending', 'clean', 'blocked')
   AND upload.usage = CASE NEW.payload->>'kind'
         WHEN 'thread' THEN 'forum_thread'
         WHEN 'comment' THEN 'forum_comment'
       END
  WHERE NEW.payload->>'kind' IN ('thread', 'comment')
    AND asset.ordinality <= CASE NEW.payload->>'kind' WHEN 'thread' THEN 8 ELSE 4 END
  ON CONFLICT DO NOTHING;
  RETURN NEW;
END;
$$;

CREATE TRIGGER forum_draft_media_reference_sync
AFTER INSERT OR UPDATE OF payload ON forum.drafts
FOR EACH ROW EXECUTE FUNCTION media.sync_draft_asset_references_from_source();

-- Install source-table triggers before taking the backfill snapshot. CREATE TRIGGER waits for
-- already-running source writers; once it returns, every later write is synchronized and this
-- backfill sees every write that committed before trigger installation.
INSERT INTO media.draft_asset_references
  (account_id, draft_key, asset_id, target_type, position)
SELECT draft.account_id,
       draft.draft_key,
       upload.id,
       CASE draft.payload->>'kind'
         WHEN 'thread' THEN 'forum_thread'
         WHEN 'comment' THEN 'forum_comment'
       END,
       (asset.ordinality - 1)::smallint
FROM forum.drafts draft
CROSS JOIN LATERAL jsonb_array_elements_text(
  CASE
    WHEN jsonb_typeof(draft.payload->'attachmentAssetIds') = 'array'
      THEN draft.payload->'attachmentAssetIds'
    ELSE '[]'::jsonb
  END
) WITH ORDINALITY AS asset(value, ordinality)
JOIN media.uploads upload
  ON upload.id = CASE
       WHEN asset.value ~ '^[1-9][0-9]{0,17}$' THEN asset.value::bigint
     END
 AND upload.account_id = draft.account_id
 AND upload.kind = 'image'
 AND upload.status IN ('pending', 'clean', 'blocked')
 AND upload.usage = CASE draft.payload->>'kind'
       WHEN 'thread' THEN 'forum_thread'
       WHEN 'comment' THEN 'forum_comment'
     END
WHERE draft.payload->>'kind' IN ('thread', 'comment')
  AND asset.ordinality <= CASE draft.payload->>'kind' WHEN 'thread' THEN 8 ELSE 4 END
ON CONFLICT DO NOTHING;

INSERT INTO media.asset_bindings (asset_id, owner_account_id, target_type, target_id)
SELECT profile.avatar_asset_id, profile.account_id, 'profile_avatar', profile.account_id
FROM identity.profiles profile
JOIN media.uploads upload ON upload.id = profile.avatar_asset_id
WHERE profile.avatar_asset_id IS NOT NULL AND upload.status = 'clean';

INSERT INTO media.asset_bindings (asset_id, owner_account_id, target_type, target_id)
SELECT profile.banner_asset_id, profile.account_id, 'profile_banner', profile.account_id
FROM identity.profiles profile
JOIN media.uploads upload ON upload.id = profile.banner_asset_id
WHERE profile.banner_asset_id IS NOT NULL AND upload.status = 'clean';

INSERT INTO media.asset_bindings (asset_id, owner_account_id, target_type, target_id)
SELECT promotion.asset_id, upload.account_id, 'platform_promotion', promotion.id
FROM platform.promotions promotion
JOIN media.uploads upload ON upload.id = promotion.asset_id
WHERE promotion.asset_id IS NOT NULL AND promotion.status <> 'archived' AND upload.status = 'clean';

CREATE TABLE media.asset_retention_holds (
  id                 BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  asset_id           BIGINT NOT NULL REFERENCES media.uploads(id),
  hold_kind          TEXT NOT NULL CHECK (hold_kind IN ('moderation', 'security')),
  reason             TEXT NOT NULL CHECK (
    char_length(btrim(reason)) BETWEEN 3 AND 500 AND reason = btrim(reason)
  ),
  placed_by          BIGINT NOT NULL REFERENCES identity.accounts(id),
  expires_at         TIMESTAMPTZ NOT NULL,
  released_at        TIMESTAMPTZ,
  released_by        BIGINT REFERENCES identity.accounts(id),
  release_reason     TEXT CHECK (
    release_reason IS NULL OR (
      char_length(btrim(release_reason)) BETWEEN 3 AND 500
      AND release_reason = btrim(release_reason)
    )
  ),
  created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK (expires_at > created_at),
  CHECK (
    (released_at IS NULL AND released_by IS NULL AND release_reason IS NULL)
    OR
    (released_at IS NOT NULL AND released_by IS NOT NULL AND release_reason IS NOT NULL)
  )
);

CREATE UNIQUE INDEX media_asset_retention_holds_active_idx
  ON media.asset_retention_holds (asset_id)
  WHERE released_at IS NULL;

CREATE INDEX media_asset_retention_holds_expiry_idx
  ON media.asset_retention_holds (expires_at, id) INCLUDE (asset_id)
  WHERE released_at IS NULL;

ALTER TABLE media.object_deletion_jobs
  ALTER COLUMN requested_by DROP NOT NULL,
  ALTER COLUMN requested_role DROP NOT NULL,
  ADD COLUMN request_source TEXT NOT NULL DEFAULT 'moderation' CHECK (
    request_source IN ('moderation', 'retention_gc', 'account_purge', 'intent_cleanup')
  ),
  ADD CONSTRAINT media_object_deletion_jobs_actor_shape CHECK (
    (request_source = 'moderation' AND requested_by IS NOT NULL AND requested_role IS NOT NULL)
    OR
    (request_source <> 'moderation' AND requested_by IS NULL AND requested_role IS NULL)
  );

CREATE TABLE media.object_deletion_job_retry_events (
  id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  job_id      BIGINT NOT NULL REFERENCES media.object_deletion_jobs(id) ON DELETE CASCADE,
  actor_id    BIGINT NOT NULL REFERENCES identity.accounts(id),
  reason      TEXT NOT NULL CHECK (
    char_length(btrim(reason)) BETWEEN 3 AND 500 AND reason = btrim(reason)
  ),
  created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX media_object_deletion_job_retry_events_retention_idx
  ON media.object_deletion_job_retry_events (created_at, id);

-- Rows whose provider deletion completed before this migration no longer need storage locators or
-- content fingerprints. The stable upload id remains for governance/audit references.
UPDATE media.uploads upload
SET oss_key = 'redacted/' || upload.id,
    url = '',
    bytes = 0,
    mime = 'application/octet-stream',
    sha256 = '',
    usage = NULL,
    image_width = NULL,
    image_height = NULL,
    cleaned_at = NULL,
    redacted_at = COALESCE(job.completed_at, now())
FROM media.object_deletion_jobs job
WHERE job.upload_id = upload.id
  AND job.status = 'succeeded'
  AND upload.status = 'blocked'
  AND upload.redacted_at IS NULL;

-- Before durable deletion jobs existed, the only supported transition to blocked happened after a
-- successful provider DELETE. Redact those legacy rows under that established invariant as well.
UPDATE media.uploads upload
SET oss_key = 'redacted/' || upload.id,
    url = '',
    bytes = 0,
    mime = 'application/octet-stream',
    sha256 = '',
    usage = NULL,
    image_width = NULL,
    image_height = NULL,
    cleaned_at = NULL,
    redacted_at = now()
WHERE upload.status = 'blocked'
  AND upload.redacted_at IS NULL;

DELETE FROM media.upload_intents intent
USING media.uploads upload
WHERE intent.upload_id = upload.id
  AND upload.status = 'blocked';

CREATE INDEX media_uploads_gc_candidates_idx
  ON media.uploads (cleaned_at, id)
  WHERE status = 'clean';
