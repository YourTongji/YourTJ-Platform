-- 0016_email_encryption.sql — versioned AEAD email encryption with HMAC blind index.
-- Plaintext email column is retained as a compatibility path; it MUST be backfilled
-- with NULL before enabling strict mode. The server validates this at startup.

-- Add columns for encrypted email + versioned blind index.
ALTER TABLE identity.accounts
  ADD COLUMN email_ciphertext TEXT,
  ADD COLUMN email_key_version SMALLINT,
  ADD COLUMN email_blind_index TEXT;   -- hex(SHA256-HMAC) deterministic lookup

-- Unique index on the blind index (when populated) for exact-match lookups.
CREATE UNIQUE INDEX accounts_email_blind_idx
  ON identity.accounts (email_blind_index) WHERE email_blind_index IS NOT NULL;

-- email_codes also needs a blind-index path for verification-code lookups.
ALTER TABLE identity.email_codes
  ADD COLUMN email_blind_index TEXT,
  ADD COLUMN email_key_version SMALLINT;

CREATE INDEX email_codes_blind_idx
  ON identity.email_codes (email_blind_index, expires_at)
  WHERE email_blind_index IS NOT NULL;

-- password_hash lookup table needs blind-index migration.
ALTER TABLE identity.accounts
  ADD COLUMN password_email_blind TEXT;   -- blind index used for find_password_hash lookups

CREATE INDEX accounts_password_email_blind_idx
  ON identity.accounts (password_email_blind) WHERE password_email_blind IS NOT NULL;
