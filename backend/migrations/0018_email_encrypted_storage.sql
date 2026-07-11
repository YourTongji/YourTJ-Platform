-- 0018_email_encrypted_storage.sql — allow encrypted-only identity rows.

ALTER TABLE identity.accounts
  ALTER COLUMN email DROP NOT NULL;

ALTER TABLE identity.email_codes
  ALTER COLUMN email DROP NOT NULL;

ALTER TABLE identity.accounts
  ADD CONSTRAINT accounts_email_storage_check CHECK (
    email IS NOT NULL
    OR (
      email_ciphertext IS NOT NULL
      AND email_key_version IS NOT NULL
      AND email_blind_index IS NOT NULL
    )
  );

ALTER TABLE identity.email_codes
  ADD CONSTRAINT email_codes_storage_check CHECK (
    email IS NOT NULL
    OR (email_key_version IS NOT NULL AND email_blind_index IS NOT NULL)
  );
