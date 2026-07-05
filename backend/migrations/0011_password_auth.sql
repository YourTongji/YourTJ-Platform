-- Add password_hash column for password-based authentication.
-- NULL means the account only uses email-code login (legacy / newly created without password).
ALTER TABLE identity.accounts ADD COLUMN IF NOT EXISTS password_hash TEXT;
