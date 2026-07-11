-- 0034_session_family_and_audit.sql — Add session family tracking,
-- refresh token reuse detection, and device metadata.
--
-- Before this migration, refresh rotation existed but could not detect
-- reuse of a compromised old refresh token.

ALTER TABLE identity.sessions
  ADD COLUMN family_id            UUID,
  ADD COLUMN parent_session_id    BIGINT REFERENCES identity.sessions(id),
  ADD COLUMN replaced_by_session_id BIGINT REFERENCES identity.sessions(id),
  ADD COLUMN last_used_at         TIMESTAMPTZ,
  ADD COLUMN device_name          TEXT,
  ADD COLUMN ip_prefix            TEXT,
  ADD COLUMN recent_auth_at       TIMESTAMPTZ,
  ADD COLUMN recent_auth_method   TEXT;

COMMENT ON COLUMN identity.sessions.family_id IS 'Groups sessions created by the same refresh chain';
COMMENT ON COLUMN identity.sessions.parent_session_id IS 'The session that this one replaced';
COMMENT ON COLUMN identity.sessions.replaced_by_session_id IS 'The session that replaced this one';
