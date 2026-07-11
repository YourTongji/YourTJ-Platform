-- 0033_email_code_purpose_and_audit.sql — Add purpose, atomic consumption,
-- delivery tracking, and request correlation to identity.email_codes.
--
-- Before this migration, email codes were purpose-agnostic: a login code
-- could be used for password reset or registration, and a successfully
-- verified code could be replayed because there was no used_at.
--
-- Backward compatible: old rows get NULL defaults for new columns; the
-- application layer treats NULL-purpose codes as valid for any purpose
-- during the migration window.

ALTER TABLE identity.email_codes
  ADD COLUMN purpose           TEXT,          -- login, registration, password_reset, recent_auth
  ADD COLUMN max_attempts      INT NOT NULL DEFAULT 5,
  ADD COLUMN used_at           TIMESTAMPTZ,
  ADD COLUMN revoked_at        TIMESTAMPTZ,
  ADD COLUMN delivery_status   TEXT NOT NULL DEFAULT 'pending',  -- pending, accepted, failed
  ADD COLUMN request_id        UUID;
