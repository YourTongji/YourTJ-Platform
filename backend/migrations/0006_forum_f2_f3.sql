-- 0006_forum_f2_f3.sql — Forum Discourse Parity F2 + F3
--
-- F2: Media, drafts, quotes, onebox, profiles, blocking, SSE, badges
-- F3: DMs, polls, email digest, auto-archive, solved answers
--
-- Append-only: never edit an applied migration.

-- ============================================================================
-- 2.1 New schema: media (owned by new media crate)
-- ============================================================================
CREATE SCHEMA IF NOT EXISTS media;

CREATE TABLE media.uploads (
  id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  account_id  BIGINT NOT NULL REFERENCES identity.accounts(id),
  kind        TEXT NOT NULL CHECK (kind IN ('image', 'file')),
  oss_key     TEXT NOT NULL UNIQUE,
  url         TEXT NOT NULL,
  bytes       BIGINT NOT NULL,
  mime        TEXT NOT NULL,
  sha256      TEXT NOT NULL,
  status      TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'clean', 'blocked')),
  created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX ON media.uploads (account_id, created_at DESC);
CREATE INDEX ON media.uploads (status, created_at);

-- ============================================================================
-- 2.2 Drafts
-- ============================================================================
CREATE TABLE forum.drafts (
  account_id  BIGINT NOT NULL REFERENCES identity.accounts(id),
  draft_key   TEXT NOT NULL,                              -- e.g. "thread:{board_id}" or "comment:{thread_id}"
  payload     JSONB NOT NULL,                             -- { title, body, tags?, parentId? }
  updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (account_id, draft_key)
);

-- ============================================================================
-- 2.3 Badges (in platform schema)
-- ============================================================================
CREATE TABLE platform.badges (
  id           BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  slug         TEXT UNIQUE NOT NULL,                       -- e.g. "first-thread", "quality-author"
  name         TEXT NOT NULL,                              -- 首次发帖, 优质作者
  description  TEXT,
  icon_url     TEXT,
  mint_amount  BIGINT NOT NULL DEFAULT 0,                  -- 0 = honorary only (no credit)
  created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE platform.account_badges (
  account_id  BIGINT NOT NULL REFERENCES identity.accounts(id),
  badge_id    BIGINT NOT NULL REFERENCES platform.badges(id),
  awarded_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  awarded_by  BIGINT NOT NULL REFERENCES identity.accounts(id),  -- system or mod
  PRIMARY KEY (account_id, badge_id)
);

-- ============================================================================
-- 2.4 User ignores (blocking)
-- ============================================================================
CREATE TABLE forum.user_ignores (
  account_id          BIGINT NOT NULL REFERENCES identity.accounts(id),
  ignored_account_id  BIGINT NOT NULL REFERENCES identity.accounts(id),
  created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (account_id, ignored_account_id),
  CHECK (account_id != ignored_account_id)
);

-- ============================================================================
-- 2.5 DMs (1:1 private messages)
-- ============================================================================
CREATE TABLE forum.dm_conversations (
  id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE forum.dm_participants (
  conversation_id  BIGINT NOT NULL REFERENCES forum.dm_conversations(id),
  account_id       BIGINT NOT NULL REFERENCES identity.accounts(id),
  joined_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (conversation_id, account_id)
);

CREATE TABLE forum.dm_messages (
  id               BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  conversation_id  BIGINT NOT NULL REFERENCES forum.dm_conversations(id),
  sender_id        BIGINT NOT NULL REFERENCES identity.accounts(id),
  body             TEXT NOT NULL,
  created_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX ON forum.dm_messages (conversation_id, created_at ASC);

-- ============================================================================
-- 2.6 Polls
-- ============================================================================
CREATE TABLE forum.polls (
  id            BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  thread_id     BIGINT NOT NULL REFERENCES forum.threads(id) ON DELETE CASCADE UNIQUE,
  question      TEXT NOT NULL,
  multi_select  BOOLEAN NOT NULL DEFAULT FALSE,   -- false = single, true = multi
  closes_at     TIMESTAMPTZ,                      -- NULL = no close time
  created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE forum.poll_options (
  id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  poll_id     BIGINT NOT NULL REFERENCES forum.polls(id) ON DELETE CASCADE,
  position    INT NOT NULL DEFAULT 0,
  label       TEXT NOT NULL,
  vote_count  INT NOT NULL DEFAULT 0
);

CREATE INDEX ON forum.poll_options (poll_id, position);

CREATE TABLE forum.poll_votes (
  poll_option_id  BIGINT NOT NULL REFERENCES forum.poll_options(id) ON DELETE CASCADE,
  account_id      BIGINT NOT NULL REFERENCES identity.accounts(id),
  created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (poll_option_id, account_id)
);

-- ============================================================================
-- 2.7 Solved (Q&A boards)
-- ============================================================================
ALTER TABLE forum.boards
  ADD COLUMN is_qa BOOLEAN NOT NULL DEFAULT FALSE;  -- 问答板块

ALTER TABLE forum.threads
  ADD COLUMN solved_answer_id BIGINT REFERENCES forum.comments(id);  -- 采纳的答案

-- ============================================================================
-- 2.8 Link previews for Onebox
-- ============================================================================
CREATE TABLE platform.link_previews (
  url_hash    TEXT PRIMARY KEY,    -- SHA-256 hex of the URL
  url         TEXT NOT NULL,
  title       TEXT,
  description TEXT,
  image_url   TEXT,
  site_name   TEXT,
  fetched_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ============================================================================
-- New indexes
-- ============================================================================
CREATE INDEX ON forum.drafts (account_id, updated_at DESC);
CREATE INDEX ON platform.account_badges (account_id);
CREATE INDEX ON platform.badges (slug);

-- ============================================================================
-- 2.9 Thread featuring (badge-related)
-- ============================================================================
ALTER TABLE forum.threads
  ADD COLUMN featured_at TIMESTAMPTZ;
