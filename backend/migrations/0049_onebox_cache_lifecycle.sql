-- 0049_onebox_cache_lifecycle.sql — bounded success and failure caching for link previews.
--
-- Policy-v2 cache keys are unreachable after URL normalization changes, so the
-- derived cache is cleared. New successes and failures receive explicit expiry.

ALTER TABLE platform.link_previews
  ADD COLUMN status TEXT NOT NULL DEFAULT 'ready'
    CHECK (status IN ('ready', 'error')),
  ADD COLUMN error_category TEXT,
  ADD COLUMN expires_at TIMESTAMPTZ NOT NULL DEFAULT (now() + interval '7 days'),
  ADD CONSTRAINT link_previews_cache_state CHECK (
    (status = 'ready' AND error_category IS NULL)
    OR (status = 'error' AND error_category IS NOT NULL AND expires_at IS NOT NULL)
  );

DELETE FROM platform.link_previews;

ALTER TABLE platform.link_previews
  ADD CONSTRAINT link_previews_query_free_url
    CHECK (position('?' IN url) = 0 AND position('#' IN url) = 0),
  ADD CONSTRAINT link_previews_no_remote_image CHECK (image_url IS NULL);

CREATE INDEX link_previews_expired_idx
  ON platform.link_previews (expires_at)
  WHERE expires_at IS NOT NULL;
