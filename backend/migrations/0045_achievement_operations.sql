-- 0045_achievement_operations.sql — versioned definitions and revocable manual awards.

ALTER TABLE platform.badges
  ADD COLUMN icon_token TEXT NOT NULL DEFAULT 'award',
  ADD COLUMN status TEXT NOT NULL DEFAULT 'active',
  ADD COLUMN version BIGINT NOT NULL DEFAULT 1,
  ADD COLUMN created_by BIGINT REFERENCES identity.accounts(id) ON DELETE SET NULL,
  ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  ADD CONSTRAINT badges_icon_token_check
    CHECK (icon_token IN ('award', 'book-open-check', 'message-circle-heart', 'star')),
  ADD CONSTRAINT badges_status_check CHECK (status IN ('active', 'retired')),
  ADD CONSTRAINT badges_version_positive CHECK (version > 0),
  ADD CONSTRAINT badges_name_length CHECK (char_length(name) BETWEEN 1 AND 100) NOT VALID,
  ADD CONSTRAINT badges_description_length
    CHECK (description IS NULL OR char_length(description) <= 240) NOT VALID;

UPDATE platform.badges
SET icon_token = CASE slug
  WHEN 'quality-author' THEN 'star'
  WHEN 'first-comment' THEN 'message-circle-heart'
  ELSE 'award'
END
WHERE slug IN ('first-thread', 'quality-author', 'first-comment');

ALTER TABLE platform.account_badges
  ADD COLUMN award_reason TEXT,
  ADD COLUMN revoked_at TIMESTAMPTZ,
  ADD COLUMN revoked_by BIGINT REFERENCES identity.accounts(id) ON DELETE SET NULL,
  ADD COLUMN revoke_reason TEXT,
  ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  ADD CONSTRAINT account_badges_award_reason_length
    CHECK (award_reason IS NULL OR char_length(award_reason) BETWEEN 3 AND 500),
  ADD CONSTRAINT account_badges_revocation_shape CHECK (
    (revoked_at IS NULL AND revoked_by IS NULL AND revoke_reason IS NULL)
    OR
    (revoked_at IS NOT NULL AND revoke_reason IS NOT NULL
      AND char_length(revoke_reason) BETWEEN 3 AND 500)
  );

CREATE TABLE platform.achievement_events (
  id             BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  account_id     BIGINT NOT NULL REFERENCES identity.accounts(id) ON DELETE CASCADE,
  badge_id       BIGINT NOT NULL REFERENCES platform.badges(id),
  action         TEXT NOT NULL CHECK (action IN ('awarded', 'revoked')),
  source         TEXT NOT NULL CHECK (source IN ('automatic', 'manual')),
  actor_id       BIGINT REFERENCES identity.accounts(id) ON DELETE SET NULL,
  reason         TEXT NOT NULL CHECK (char_length(reason) BETWEEN 3 AND 500),
  created_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX achievement_events_account_cursor_idx
  ON platform.achievement_events (account_id, id DESC);

CREATE INDEX account_badges_active_account_idx
  ON platform.account_badges (account_id, awarded_at DESC)
  WHERE revoked_at IS NULL;
