-- 0059_media_asset_variants.sql — track derived media variants and their lifecycle state.

CREATE TABLE media.asset_variants (
  id                  BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  asset_id            BIGINT NOT NULL REFERENCES media.uploads(id) ON DELETE CASCADE,
  variant             TEXT NOT NULL CHECK (
    variant IN ('original', 'thumbnail', 'small', 'medium', 'large', 'avif', 'webp')
  ),
  object_key          TEXT NOT NULL,
  content_hash        TEXT NOT NULL,
  mime                TEXT NOT NULL,
  bytes               BIGINT NOT NULL CHECK (bytes > 0),
  width               INTEGER,
  height              INTEGER,
  status              TEXT NOT NULL CHECK (status IN ('processing', 'published', 'quarantined', 'deleted')),
  processing_attempts INTEGER NOT NULL DEFAULT 0 CHECK (processing_attempts >= 0),
  created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
  published_at        TIMESTAMPTZ,
  quarantined_at      TIMESTAMPTZ,
  deleted_at          TIMESTAMPTZ,
  UNIQUE (asset_id, variant, content_hash)
);

CREATE INDEX media_asset_variants_asset_status_idx
  ON media.asset_variants (asset_id, status);
