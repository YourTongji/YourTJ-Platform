-- Announcements gain lifecycle, revision, audience and per-account receipt state.
-- Promotions are first-party, scheduled platform content with controlled media references.

ALTER TABLE platform.announcements
  ADD COLUMN status TEXT NOT NULL DEFAULT 'published',
  ADD COLUMN presentation TEXT NOT NULL DEFAULT 'card',
  ADD COLUMN severity TEXT NOT NULL DEFAULT 'info',
  ADD COLUMN priority INTEGER NOT NULL DEFAULT 0,
  ADD COLUMN audience TEXT NOT NULL DEFAULT 'all',
  ADD COLUMN requires_ack BOOLEAN NOT NULL DEFAULT FALSE,
  ADD COLUMN version BIGINT NOT NULL DEFAULT 1,
  ADD COLUMN revision BIGINT NOT NULL DEFAULT 1,
  ADD COLUMN starts_at TIMESTAMPTZ,
  ADD COLUMN ends_at TIMESTAMPTZ,
  ADD COLUMN published_at TIMESTAMPTZ,
  ADD COLUMN archived_at TIMESTAMPTZ,
  ADD COLUMN created_by BIGINT REFERENCES identity.accounts(id) ON DELETE SET NULL,
  ADD COLUMN updated_by BIGINT REFERENCES identity.accounts(id) ON DELETE SET NULL,
  ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT now();

UPDATE platform.announcements
SET starts_at = created_at,
    published_at = created_at,
    updated_at = created_at;

ALTER TABLE platform.announcements
  ADD CONSTRAINT announcements_status_check
    CHECK (status IN ('draft', 'scheduled', 'published', 'archived')),
  ADD CONSTRAINT announcements_presentation_check
    CHECK (presentation IN ('card', 'banner')),
  ADD CONSTRAINT announcements_severity_check
    CHECK (severity IN ('info', 'success', 'warning', 'critical')),
  ADD CONSTRAINT announcements_audience_check
    CHECK (audience IN ('all', 'authenticated', 'staff')),
  ADD CONSTRAINT announcements_priority_check CHECK (priority BETWEEN -1000 AND 1000),
  ADD CONSTRAINT announcements_version_check CHECK (version >= 1),
  ADD CONSTRAINT announcements_revision_check CHECK (revision >= 1),
  ADD CONSTRAINT announcements_title_check CHECK (char_length(btrim(title)) BETWEEN 1 AND 200),
  ADD CONSTRAINT announcements_body_check CHECK (body IS NULL OR char_length(body) <= 20000),
  ADD CONSTRAINT announcements_schedule_check CHECK (ends_at IS NULL OR starts_at IS NULL OR ends_at > starts_at),
  ADD CONSTRAINT announcements_scheduled_start_check CHECK (status <> 'scheduled' OR starts_at IS NOT NULL),
  ADD CONSTRAINT announcements_archive_time_check CHECK (status <> 'archived' OR archived_at IS NOT NULL);

CREATE TABLE platform.announcement_revisions (
  announcement_id BIGINT NOT NULL REFERENCES platform.announcements(id) ON DELETE CASCADE,
  version BIGINT NOT NULL,
  revision BIGINT NOT NULL,
  title TEXT NOT NULL,
  body TEXT,
  status TEXT NOT NULL,
  presentation TEXT NOT NULL,
  severity TEXT NOT NULL,
  priority INTEGER NOT NULL,
  audience TEXT NOT NULL,
  requires_ack BOOLEAN NOT NULL,
  starts_at TIMESTAMPTZ,
  ends_at TIMESTAMPTZ,
  changed_by BIGINT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (announcement_id, version),
  CHECK (version >= 1),
  CHECK (revision >= 1),
  CHECK (status IN ('draft', 'scheduled', 'published', 'archived')),
  CHECK (presentation IN ('card', 'banner')),
  CHECK (severity IN ('info', 'success', 'warning', 'critical')),
  CHECK (priority BETWEEN -1000 AND 1000),
  CHECK (audience IN ('all', 'authenticated', 'staff')),
  CHECK (char_length(btrim(title)) BETWEEN 1 AND 200),
  CHECK (body IS NULL OR char_length(body) <= 20000),
  CHECK (ends_at IS NULL OR starts_at IS NULL OR ends_at > starts_at)
);

INSERT INTO platform.announcement_revisions (
  announcement_id, version, revision, title, body, status, presentation, severity, priority,
  audience, requires_ack, starts_at, ends_at, changed_by, created_at
)
SELECT id, version, revision, title, body, status, presentation, severity, priority,
       audience, requires_ack, starts_at, ends_at, updated_by, updated_at
FROM platform.announcements;

CREATE OR REPLACE FUNCTION platform.reject_announcement_revision_mutation()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
  RAISE EXCEPTION 'platform.announcement_revisions is append-only';
END;
$$;

CREATE TRIGGER announcement_revisions_append_only
BEFORE UPDATE OR DELETE ON platform.announcement_revisions
FOR EACH ROW EXECUTE FUNCTION platform.reject_announcement_revision_mutation();

CREATE TABLE platform.announcement_receipts (
  account_id BIGINT NOT NULL REFERENCES identity.accounts(id) ON DELETE CASCADE,
  announcement_id BIGINT NOT NULL REFERENCES platform.announcements(id) ON DELETE CASCADE,
  revision BIGINT NOT NULL,
  first_seen_at TIMESTAMPTZ,
  dismissed_at TIMESTAMPTZ,
  acknowledged_at TIMESTAMPTZ,
  PRIMARY KEY (account_id, announcement_id, revision),
  CHECK (revision >= 1),
  CHECK (dismissed_at IS NULL OR first_seen_at IS NOT NULL),
  CHECK (acknowledged_at IS NULL OR first_seen_at IS NOT NULL)
);

CREATE INDEX announcement_receipts_announcement_revision_idx
  ON platform.announcement_receipts (announcement_id, revision);

CREATE INDEX announcements_active_idx
  ON platform.announcements (priority DESC, published_at DESC, id)
  WHERE status IN ('scheduled', 'published');

CREATE TABLE platform.promotions (
  id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  placement TEXT NOT NULL,
  title TEXT NOT NULL,
  body TEXT,
  cta_label TEXT,
  target_url TEXT NOT NULL,
  asset_id BIGINT REFERENCES media.uploads(id) ON DELETE SET NULL,
  status TEXT NOT NULL DEFAULT 'draft',
  priority INTEGER NOT NULL DEFAULT 0,
  audience TEXT NOT NULL DEFAULT 'all',
  version BIGINT NOT NULL DEFAULT 1,
  starts_at TIMESTAMPTZ,
  ends_at TIMESTAMPTZ,
  created_by BIGINT REFERENCES identity.accounts(id) ON DELETE SET NULL,
  updated_by BIGINT REFERENCES identity.accounts(id) ON DELETE SET NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  archived_at TIMESTAMPTZ,
  CHECK (placement IN ('home-left-primary', 'home-left-secondary')),
  CHECK (char_length(btrim(title)) BETWEEN 1 AND 120),
  CHECK (body IS NULL OR char_length(body) <= 500),
  CHECK (cta_label IS NULL OR char_length(cta_label) <= 40),
  CHECK (char_length(target_url) BETWEEN 1 AND 2048),
  CHECK (left(target_url, 1) = '/' AND left(target_url, 2) <> '//' AND position(E'\\' in target_url) = 0),
  CHECK (status IN ('draft', 'scheduled', 'published', 'paused', 'archived')),
  CHECK (priority BETWEEN -1000 AND 1000),
  CHECK (audience IN ('all', 'authenticated', 'staff')),
  CHECK (version >= 1),
  CHECK (ends_at IS NULL OR starts_at IS NULL OR ends_at > starts_at),
  CHECK (status <> 'scheduled' OR starts_at IS NOT NULL),
  CHECK (status <> 'archived' OR archived_at IS NOT NULL)
);

CREATE INDEX promotions_active_placement_idx
  ON platform.promotions (placement, priority DESC, starts_at DESC, id)
  WHERE status IN ('scheduled', 'published');
CREATE INDEX promotions_admin_idx ON platform.promotions (id DESC);
