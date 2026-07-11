-- Compose every purpose introduced by the independent appeal and recent-auth migrations.
-- Both preceding application versions remain valid during a rolling deployment.

ALTER TABLE identity.email_codes
  DROP CONSTRAINT email_codes_purpose_check,
  ADD CONSTRAINT email_codes_purpose_check
    CHECK (purpose IN ('login', 'registration', 'password_reset', 'recent_auth', 'appeal'));
