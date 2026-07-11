-- 0015_media_upload_intents.sql — server-issued OSS upload authorization and callback idempotency.

CREATE TABLE media.upload_intents (
  id              UUID PRIMARY KEY,
  account_id      BIGINT NOT NULL REFERENCES identity.accounts(id),
  kind            TEXT NOT NULL CHECK (kind IN ('image', 'file')),
  oss_key         TEXT NOT NULL UNIQUE,
  content_type    TEXT NOT NULL,
  max_bytes       BIGINT NOT NULL CHECK (max_bytes > 0 AND max_bytes <= 20971520),
  callback_token  TEXT NOT NULL UNIQUE,
  expires_at      TIMESTAMPTZ NOT NULL,
  consumed_at     TIMESTAMPTZ,
  upload_id       BIGINT UNIQUE REFERENCES media.uploads(id),
  created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX ON media.upload_intents (account_id, created_at DESC);
CREATE INDEX ON media.upload_intents (expires_at) WHERE consumed_at IS NULL;
