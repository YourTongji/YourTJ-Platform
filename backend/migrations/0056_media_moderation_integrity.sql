-- Make media approval evidence-bound and object deletion durable.

ALTER TABLE media.uploads
  DROP CONSTRAINT uploads_status_check,
  ADD CONSTRAINT uploads_status_check
    CHECK (status IN ('pending', 'clean', 'quarantined', 'blocked'));

CREATE FUNCTION media.enforce_upload_status_transition()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
  IF NEW.status = OLD.status THEN
    RETURN NEW;
  END IF;
  IF (OLD.status = 'pending' AND NEW.status IN ('clean', 'quarantined'))
     OR (OLD.status = 'clean' AND NEW.status = 'quarantined')
     OR (OLD.status = 'quarantined' AND NEW.status = 'blocked') THEN
    RETURN NEW;
  END IF;
  RAISE EXCEPTION 'invalid media upload status transition: % -> %', OLD.status, NEW.status
    USING ERRCODE = 'check_violation';
END;
$$;

CREATE TRIGGER media_upload_status_transition_guard
BEFORE UPDATE OF status ON media.uploads
FOR EACH ROW EXECUTE FUNCTION media.enforce_upload_status_transition();

CREATE TABLE media.moderation_evidence (
  id               BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  upload_id        BIGINT NOT NULL REFERENCES media.uploads(id) ON DELETE CASCADE,
  evidence_kind    TEXT NOT NULL CHECK (
    evidence_kind IN ('trusted_image_preview', 'malware_scan', 'content_scan')
  ),
  verdict          TEXT NOT NULL CHECK (verdict IN ('observed', 'clean', 'blocked')),
  actor_account_id BIGINT REFERENCES identity.accounts(id),
  provider         TEXT,
  observed_mime    TEXT NOT NULL,
  image_width      INTEGER,
  image_height     INTEGER,
  created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK (actor_account_id IS NOT NULL OR provider IS NOT NULL),
  CHECK (
    (image_width IS NULL AND image_height IS NULL)
    OR (image_width BETWEEN 1 AND 20000 AND image_height BETWEEN 1 AND 20000)
  )
);

CREATE INDEX media_moderation_evidence_upload_idx
  ON media.moderation_evidence (upload_id, evidence_kind, created_at DESC);

CREATE TABLE media.object_deletion_jobs (
  id                BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  upload_id         BIGINT NOT NULL UNIQUE REFERENCES media.uploads(id) ON DELETE CASCADE,
  requested_by      BIGINT NOT NULL REFERENCES identity.accounts(id),
  requested_role    TEXT NOT NULL CHECK (requested_role IN ('mod', 'admin')),
  reason            TEXT NOT NULL CHECK (
    char_length(btrim(reason)) BETWEEN 3 AND 500
    AND reason = btrim(reason)
  ),
  previous_status   TEXT NOT NULL CHECK (previous_status IN ('pending', 'clean')),
  status            TEXT NOT NULL DEFAULT 'queued' CHECK (
    status IN ('queued', 'leased', 'succeeded', 'dead_letter')
  ),
  attempt_count     INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count BETWEEN 0 AND 8),
  available_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
  lease_token       UUID,
  lease_expires_at  TIMESTAMPTZ,
  last_error_code   TEXT,
  completed_at      TIMESTAMPTZ,
  created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK (
    (status = 'leased' AND lease_token IS NOT NULL AND lease_expires_at IS NOT NULL)
    OR (status <> 'leased' AND lease_token IS NULL AND lease_expires_at IS NULL)
  ),
  CHECK (
    (status = 'succeeded' AND completed_at IS NOT NULL)
    OR (status <> 'succeeded' AND completed_at IS NULL)
  )
);

CREATE INDEX media_object_deletion_jobs_ready_idx
  ON media.object_deletion_jobs (available_at, id)
  WHERE status IN ('queued', 'leased');
