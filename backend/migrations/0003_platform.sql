-- 0003_platform.sql — platform-level settings and announcements.
-- Append-only. Settings are simple key-value; announcements are public.
CREATE SCHEMA IF NOT EXISTS platform;

CREATE TABLE platform.announcements (
  id         BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  title      TEXT NOT NULL,
  body       TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE platform.settings (
  key        TEXT PRIMARY KEY,
  value      TEXT NOT NULL,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO platform.settings (key, value) VALUES
  ('app_name', 'YourTJ'),
  ('version', '2.0.0'),
  ('contact_email', 'admin@yourtj.de');
