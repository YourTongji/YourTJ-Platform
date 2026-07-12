-- 0048_recent_auth.sql — purpose-bound recent authentication on revocable sessions.
-- Append-only: never edit an applied migration.

ALTER TABLE identity.email_codes
  DROP CONSTRAINT email_codes_purpose_check,
  ADD CONSTRAINT email_codes_purpose_check
    CHECK (purpose IN ('login', 'registration', 'password_reset', 'recent_auth'));

ALTER TABLE identity.sessions
  ADD COLUMN recent_authenticated_at TIMESTAMPTZ,
  ADD COLUMN recent_auth_method TEXT,
  ADD CONSTRAINT sessions_recent_auth_method_check
    CHECK (recent_auth_method IS NULL OR recent_auth_method IN ('password', 'email_code')),
  ADD CONSTRAINT sessions_recent_auth_shape_check
    CHECK ((recent_authenticated_at IS NULL) = (recent_auth_method IS NULL));

COMMENT ON COLUMN identity.sessions.recent_authenticated_at IS
  'Server-written step-up time for this revocable session family; never derived from JWT iat.';
COMMENT ON COLUMN identity.sessions.recent_auth_method IS
  'Bounded method label only; passwords, codes, and email addresses are never stored here.';
