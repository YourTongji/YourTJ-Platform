-- Private Ingest-to-Delivery media publication, retryable processing, and multi-object cleanup.

ALTER TABLE media.moderation_preview_grants
  ADD COLUMN self_review BOOLEAN NOT NULL DEFAULT FALSE;

ALTER TABLE media.moderation_evidence
  ADD COLUMN self_review BOOLEAN NOT NULL DEFAULT FALSE;

ALTER TABLE media.object_deletion_jobs
  ADD COLUMN self_review BOOLEAN NOT NULL DEFAULT FALSE,
  ADD CONSTRAINT media_object_deletion_jobs_self_review_shape CHECK (
    NOT self_review OR (
      request_source = 'moderation'
      AND requested_by IS NOT NULL
      AND requested_role = 'admin'
    )
  );

CREATE TABLE media.asset_publications (
  asset_id       BIGINT PRIMARY KEY REFERENCES media.uploads(id) ON DELETE CASCADE,
  policy_version INTEGER NOT NULL DEFAULT 1 CHECK (policy_version > 0),
  status         TEXT NOT NULL DEFAULT 'unpublished' CHECK (
    status IN ('unpublished', 'processing', 'published', 'failed', 'blocked')
  ),
  published_at   TIMESTAMPTZ,
  blocked_at     TIMESTAMPTZ,
  last_error_code TEXT,
  updated_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK (
    (status = 'published' AND published_at IS NOT NULL AND blocked_at IS NULL)
    OR (status = 'blocked' AND blocked_at IS NOT NULL)
    OR (status NOT IN ('published', 'blocked') AND published_at IS NULL AND blocked_at IS NULL)
  ),
  CHECK (last_error_code IS NULL OR char_length(last_error_code) BETWEEN 1 AND 100)
);

CREATE INDEX media_asset_publications_state_idx
  ON media.asset_publications (status, updated_at, asset_id);

CREATE TABLE media.asset_variants (
  id             BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  asset_id       BIGINT NOT NULL REFERENCES media.uploads(id) ON DELETE CASCADE,
  variant_kind   TEXT NOT NULL CHECK (
    variant_kind IN ('thumb_256', 'display_1280', 'full_2048')
  ),
  policy_version INTEGER NOT NULL CHECK (policy_version > 0),
  object_key     TEXT NOT NULL UNIQUE CHECK (
    object_key ~ '^assets/[1-9][0-9]*/[1-9][0-9]*/[a-z0-9_]+-[0-9a-f]{64}\.webp$'
  ),
  content_sha256 CHAR(64) NOT NULL CHECK (content_sha256 ~ '^[0-9a-f]{64}$'),
  mime           TEXT NOT NULL CHECK (mime = 'image/webp'),
  bytes          BIGINT NOT NULL CHECK (bytes BETWEEN 1 AND 20971520),
  width          INTEGER NOT NULL CHECK (width BETWEEN 1 AND 20000),
  height         INTEGER NOT NULL CHECK (height BETWEEN 1 AND 20000),
  status         TEXT NOT NULL DEFAULT 'processing' CHECK (
    status IN ('processing', 'published', 'quarantined', 'deleted')
  ),
  published_at   TIMESTAMPTZ,
  deleted_at     TIMESTAMPTZ,
  CHECK (status <> 'published' OR published_at IS NOT NULL),
  CHECK (status <> 'processing' OR published_at IS NULL),
  CHECK ((status = 'deleted') = (deleted_at IS NOT NULL)),
  UNIQUE (asset_id, policy_version, variant_kind)
);

CREATE INDEX media_asset_variants_asset_state_idx
  ON media.asset_variants (asset_id, policy_version, status, variant_kind);

CREATE TABLE media.variant_processing_jobs (
  id              BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  asset_id        BIGINT NOT NULL REFERENCES media.uploads(id) ON DELETE CASCADE,
  policy_version  INTEGER NOT NULL CHECK (policy_version > 0),
  status          TEXT NOT NULL DEFAULT 'queued' CHECK (
    status IN ('queued', 'leased', 'succeeded', 'dead_letter')
  ),
  attempt_count   INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count BETWEEN 0 AND 8),
  available_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
  lease_token     UUID,
  lease_expires_at TIMESTAMPTZ,
  last_error_code TEXT,
  completed_at    TIMESTAMPTZ,
  created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK (
    (status = 'leased' AND lease_token IS NOT NULL AND lease_expires_at IS NOT NULL)
    OR (status <> 'leased' AND lease_token IS NULL AND lease_expires_at IS NULL)
  ),
  CHECK (
    (status = 'succeeded' AND completed_at IS NOT NULL)
    OR (status <> 'succeeded' AND completed_at IS NULL)
  ),
  CHECK (last_error_code IS NULL OR char_length(last_error_code) BETWEEN 1 AND 100),
  UNIQUE (asset_id, policy_version)
);

CREATE INDEX media_variant_processing_jobs_ready_idx
  ON media.variant_processing_jobs (available_at, id)
  WHERE status IN ('queued', 'leased');

CREATE TABLE media.object_cleanup_steps (
  id               BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  deletion_job_id  BIGINT NOT NULL REFERENCES media.object_deletion_jobs(id) ON DELETE CASCADE,
  step_kind        TEXT NOT NULL CHECK (
    step_kind IN ('cdn_purge', 'delivery_delete', 'ingest_delete')
  ),
  object_key       TEXT NOT NULL CHECK (
    object_key <> '' AND object_key !~ '(^/|\.\.)'
  ),
  status           TEXT NOT NULL DEFAULT 'queued' CHECK (
    status IN ('queued', 'leased', 'succeeded', 'dead_letter')
  ),
  attempt_count    INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count BETWEEN 0 AND 8),
  available_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
  lease_token      UUID,
  lease_expires_at TIMESTAMPTZ,
  last_error_code  TEXT,
  provider_task_id TEXT,
  provider_task_submitted_at TIMESTAMPTZ,
  completed_at     TIMESTAMPTZ,
  created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK (
    (status = 'leased' AND lease_token IS NOT NULL AND lease_expires_at IS NOT NULL)
    OR (status <> 'leased' AND lease_token IS NULL AND lease_expires_at IS NULL)
  ),
  CHECK (
    (provider_task_id IS NULL AND provider_task_submitted_at IS NULL)
    OR (
      step_kind = 'cdn_purge'
      AND provider_task_id ~ '^[0-9]+(,[0-9]+){0,9}$'
      AND char_length(provider_task_id) BETWEEN 1 AND 255
      AND provider_task_submitted_at IS NOT NULL
    )
  ),
  CHECK (
    (status = 'succeeded' AND completed_at IS NOT NULL)
    OR (status <> 'succeeded' AND completed_at IS NULL)
  ),
  CHECK (last_error_code IS NULL OR char_length(last_error_code) BETWEEN 1 AND 100),
  UNIQUE (deletion_job_id, step_kind, object_key)
);

CREATE INDEX media_object_cleanup_steps_ready_idx
  ON media.object_cleanup_steps (available_at, id)
  WHERE status IN ('queued', 'leased');

CREATE FUNCTION media.ensure_asset_publication()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
  INSERT INTO media.asset_publications (asset_id)
  VALUES (NEW.id)
  ON CONFLICT (asset_id) DO NOTHING;
  IF NEW.status IN ('quarantined', 'blocked') THEN
    UPDATE media.asset_publications
    SET status = 'blocked', published_at = NULL, blocked_at = COALESCE(blocked_at, now()),
        last_error_code = NULL, updated_at = now()
    WHERE asset_id = NEW.id;
    UPDATE media.asset_variants
    SET status = 'quarantined'
    WHERE asset_id = NEW.id AND status IN ('processing', 'published');
    UPDATE media.variant_processing_jobs
    SET status = 'dead_letter', lease_token = NULL, lease_expires_at = NULL,
        last_error_code = 'asset_left_clean_state', updated_at = now()
    WHERE asset_id = NEW.id AND status = 'queued';
    UPDATE media.variant_processing_jobs
    SET last_error_code = 'asset_left_clean_state', updated_at = now()
    WHERE asset_id = NEW.id AND status = 'leased';
  END IF;
  RETURN NEW;
END;
$$;

CREATE TRIGGER media_upload_publication_on_insert
AFTER INSERT ON media.uploads
FOR EACH ROW EXECUTE FUNCTION media.ensure_asset_publication();

CREATE TRIGGER media_upload_publication_on_status_update
AFTER UPDATE OF status ON media.uploads
FOR EACH ROW EXECUTE FUNCTION media.ensure_asset_publication();

INSERT INTO media.asset_publications (asset_id, status, blocked_at)
SELECT upload.id,
       CASE WHEN upload.status IN ('quarantined', 'blocked') THEN 'blocked' ELSE 'unpublished' END,
       CASE WHEN upload.status IN ('quarantined', 'blocked') THEN now() END
FROM media.uploads upload
ON CONFLICT (asset_id) DO NOTHING;

CREATE FUNCTION media.require_complete_asset_publication()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
DECLARE
  published_variant_count INTEGER;
  upload_is_clean BOOLEAN;
BEGIN
  IF NEW.status <> 'published' THEN
    RETURN NEW;
  END IF;
  SELECT upload.status = 'clean' INTO upload_is_clean
  FROM media.uploads upload
  WHERE upload.id = NEW.asset_id
  FOR SHARE;
  SELECT count(*)::integer INTO published_variant_count
  FROM media.asset_variants variant
  WHERE variant.asset_id = NEW.asset_id
    AND variant.policy_version = NEW.policy_version
    AND variant.status = 'published'
    AND variant.variant_kind IN ('thumb_256', 'display_1280', 'full_2048');
  IF upload_is_clean IS DISTINCT FROM TRUE OR published_variant_count <> 3 THEN
    RAISE EXCEPTION 'asset publication requires one complete clean variant set'
      USING ERRCODE = 'check_violation';
  END IF;
  RETURN NEW;
END;
$$;

CREATE TRIGGER media_asset_publication_completeness_guard
BEFORE INSERT OR UPDATE OF status, policy_version ON media.asset_publications
FOR EACH ROW EXECUTE FUNCTION media.require_complete_asset_publication();

CREATE FUNCTION media.enqueue_ingest_cleanup_step()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
  INSERT INTO media.object_cleanup_steps (deletion_job_id, step_kind, object_key)
  SELECT NEW.id, cleanup.step_kind, variant.object_key
  FROM media.asset_variants variant
  CROSS JOIN (VALUES ('cdn_purge'::text), ('delivery_delete'::text)) cleanup(step_kind)
  WHERE variant.asset_id = NEW.upload_id AND variant.status <> 'deleted'
  ON CONFLICT DO NOTHING;

  INSERT INTO media.object_cleanup_steps (deletion_job_id, step_kind, object_key)
  SELECT NEW.id, 'ingest_delete', upload.oss_key
  FROM media.uploads upload
  WHERE upload.id = NEW.upload_id
  ON CONFLICT DO NOTHING;
  RETURN NEW;
END;
$$;

CREATE TRIGGER media_object_deletion_job_ingest_step
AFTER INSERT ON media.object_deletion_jobs
FOR EACH ROW EXECUTE FUNCTION media.enqueue_ingest_cleanup_step();

INSERT INTO media.object_cleanup_steps (deletion_job_id, step_kind, object_key, status, completed_at)
SELECT job.id,
       'ingest_delete',
       upload.oss_key,
       CASE WHEN job.status = 'succeeded' THEN 'succeeded' ELSE 'queued' END,
       CASE WHEN job.status = 'succeeded' THEN COALESCE(job.completed_at, now()) END
FROM media.object_deletion_jobs job
JOIN media.uploads upload ON upload.id = job.upload_id
ON CONFLICT DO NOTHING;

INSERT INTO media.object_cleanup_steps (deletion_job_id, step_kind, object_key)
SELECT job.id, cleanup.step_kind, variant.object_key
FROM media.object_deletion_jobs job
JOIN media.asset_variants variant ON variant.asset_id = job.upload_id
CROSS JOIN (VALUES ('cdn_purge'::text), ('delivery_delete'::text)) cleanup(step_kind)
WHERE job.status <> 'succeeded' AND variant.status <> 'deleted'
ON CONFLICT DO NOTHING;

CREATE FUNCTION media.require_cleanup_before_deletion_completion()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
  IF NEW.status = 'succeeded' AND OLD.status <> 'succeeded' AND EXISTS (
    SELECT 1 FROM media.object_cleanup_steps step
    WHERE step.deletion_job_id = NEW.id AND step.status <> 'succeeded'
  ) THEN
    RAISE EXCEPTION 'media deletion cannot complete before every cleanup step succeeds'
      USING ERRCODE = 'check_violation';
  END IF;
  RETURN NEW;
END;
$$;

CREATE TRIGGER media_object_deletion_cleanup_completion_guard
BEFORE UPDATE OF status ON media.object_deletion_jobs
FOR EACH ROW EXECUTE FUNCTION media.require_cleanup_before_deletion_completion();

CREATE OR REPLACE FUNCTION media.replace_nonversioned_asset_binding(
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
           AND publication.status = 'published'
           AND variant.status = 'published'
           AND (required_usage IS NULL OR upload.usage = required_usage)
      INTO asset_is_valid
    FROM media.uploads upload
    JOIN media.asset_publications publication ON publication.asset_id = upload.id
    JOIN media.asset_variants variant
      ON variant.asset_id = upload.id
     AND variant.policy_version = publication.policy_version
     AND variant.variant_kind = 'display_1280'
    WHERE upload.id = desired_asset_id
    FOR SHARE OF upload, publication, variant;
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

INSERT INTO media.variant_processing_jobs (asset_id, policy_version)
SELECT upload.id, publication.policy_version
FROM media.uploads upload
JOIN media.asset_publications publication ON publication.asset_id = upload.id
WHERE upload.kind = 'image'
  AND upload.status = 'clean'
  AND upload.mime IN ('image/jpeg', 'image/png', 'image/webp')
ON CONFLICT (asset_id, policy_version) DO NOTHING;

UPDATE media.asset_publications publication
SET status = 'processing', updated_at = now()
FROM media.uploads upload
WHERE upload.id = publication.asset_id
  AND upload.kind = 'image'
  AND upload.status = 'clean'
  AND upload.mime IN ('image/jpeg', 'image/png', 'image/webp')
  AND publication.status = 'unpublished';

UPDATE media.asset_publications publication
SET status = 'failed', last_error_code = 'legacy_animated_format_requires_reupload',
    updated_at = now()
FROM media.uploads upload
WHERE upload.id = publication.asset_id
  AND upload.kind = 'image'
  AND upload.status = 'clean'
  AND upload.mime NOT IN ('image/jpeg', 'image/png', 'image/webp')
  AND publication.status = 'unpublished';
