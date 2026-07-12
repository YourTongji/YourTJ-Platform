-- Separate contribution achievements from staff-issued identity and special verifications.
-- Verification evidence remains an opaque private reference; public projections never expose it.

CREATE TABLE platform.verification_types (
  id                    BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  slug                  TEXT NOT NULL UNIQUE,
  category              TEXT NOT NULL,
  label                 TEXT NOT NULL,
  description           TEXT,
  icon                  TEXT NOT NULL,
  badge_variant         TEXT NOT NULL,
  allows_public_display BOOLEAN NOT NULL DEFAULT FALSE,
  created_by            BIGINT REFERENCES identity.accounts(id) ON DELETE SET NULL,
  created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
  CONSTRAINT verification_types_slug_format
    CHECK (slug ~ '^[a-z0-9]+(-[a-z0-9]+)*$' AND char_length(slug) BETWEEN 1 AND 64),
  CONSTRAINT verification_types_category
    CHECK (category IN ('identity', 'special')),
  CONSTRAINT verification_types_label_length
    CHECK (char_length(label) BETWEEN 1 AND 80),
  CONSTRAINT verification_types_description_length
    CHECK (description IS NULL OR char_length(description) <= 240),
  CONSTRAINT verification_types_icon
    CHECK (icon IN ('badge-check', 'building-2', 'shield-check', 'sparkles')),
  CONSTRAINT verification_types_badge_variant
    CHECK (badge_variant IN ('default', 'secondary', 'outline'))
);

CREATE TABLE platform.verification_grants (
  id                   BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  account_id           BIGINT NOT NULL REFERENCES identity.accounts(id) ON DELETE CASCADE,
  verification_type_id BIGINT NOT NULL REFERENCES platform.verification_types(id),
  display_on_profile   BOOLEAN NOT NULL DEFAULT FALSE,
  evidence_reference   TEXT,
  issue_reason         TEXT NOT NULL,
  issued_by            BIGINT REFERENCES identity.accounts(id) ON DELETE SET NULL,
  issued_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
  expires_at           TIMESTAMPTZ,
  revoked_by           BIGINT REFERENCES identity.accounts(id) ON DELETE SET NULL,
  revoked_at           TIMESTAMPTZ,
  revoke_reason        TEXT,
  CONSTRAINT verification_grants_evidence_reference
    CHECK (
      evidence_reference IS NULL OR (
        char_length(evidence_reference) BETWEEN 1 AND 128
        AND evidence_reference ~ '^[A-Za-z0-9][A-Za-z0-9._:/-]*$'
        AND position('://' IN evidence_reference) = 0
      )
    ),
  CONSTRAINT verification_grants_issue_reason_length
    CHECK (char_length(issue_reason) BETWEEN 3 AND 500),
  CONSTRAINT verification_grants_expiry
    CHECK (expires_at IS NULL OR expires_at > issued_at),
  CONSTRAINT verification_grants_revocation_shape
    CHECK (
      (revoked_at IS NULL AND revoked_by IS NULL AND revoke_reason IS NULL)
      OR
      (revoked_at IS NOT NULL AND revoke_reason IS NOT NULL AND char_length(revoke_reason) BETWEEN 3 AND 500)
    )
);

CREATE INDEX verification_types_created_cursor_idx
  ON platform.verification_types (id DESC);

CREATE INDEX verification_grants_account_cursor_idx
  ON platform.verification_grants (account_id, id DESC);

CREATE INDEX verification_grants_active_lookup_idx
  ON platform.verification_grants (account_id, verification_type_id, expires_at)
  WHERE revoked_at IS NULL;

CREATE INDEX verification_grants_public_profile_idx
  ON platform.verification_grants (account_id, issued_at DESC)
  WHERE display_on_profile AND revoked_at IS NULL;
