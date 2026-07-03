-- 0005_forum_parity.sql — Forum Discourse Parity (F0 + F1)
--
-- This migration aligns the forum domain with Discourse feature parity:
-- tags, full state machine, edit/soft-delete/revisions, read tracking,
-- subscriptions, flags, trust levels, sanctions, watched words, bookmarks,
-- mod actions, search, notifications, and rate limiting.
--
-- Append-only: never edit an applied migration. Add 0006 for F2/F3.

-- ============================================================================
-- 2.1 Boards: hierarchy + metadata
-- ============================================================================
ALTER TABLE forum.boards
  ADD COLUMN parent_id          BIGINT REFERENCES forum.boards(id),
  ADD COLUMN description        TEXT,
  ADD COLUMN position           INT NOT NULL DEFAULT 0,
  ADD COLUMN is_locked          BOOLEAN NOT NULL DEFAULT FALSE,
  ADD COLUMN min_trust_to_post  SMALLINT NOT NULL DEFAULT 0,
  ADD COLUMN thread_count       INT NOT NULL DEFAULT 0;

-- ============================================================================
-- 2.2 Threads: full state machine
-- ============================================================================
ALTER TABLE forum.threads
  ADD COLUMN pinned_at          TIMESTAMPTZ,
  ADD COLUMN pinned_globally    BOOLEAN NOT NULL DEFAULT FALSE,
  ADD COLUMN closed_at          TIMESTAMPTZ,
  ADD COLUMN archived_at        TIMESTAMPTZ,
  ADD COLUMN deleted_at         TIMESTAMPTZ,
  ADD COLUMN deleted_by         BIGINT REFERENCES identity.accounts(id),
  ADD COLUMN edited_at          TIMESTAMPTZ,
  ADD COLUMN hidden_at          TIMESTAMPTZ;

-- ============================================================================
-- 2.3 Comments: soft-delete + edit
-- ============================================================================
ALTER TABLE forum.comments
  ADD COLUMN deleted_at         TIMESTAMPTZ,
  ADD COLUMN deleted_by         BIGINT REFERENCES identity.accounts(id),
  ADD COLUMN edited_at          TIMESTAMPTZ,
  ADD COLUMN hidden_at          TIMESTAMPTZ;

-- ============================================================================
-- 2.4 Tags
-- ============================================================================
CREATE TABLE forum.tags (
  id            BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  slug          TEXT UNIQUE NOT NULL,
  name          TEXT NOT NULL,
  description   TEXT,
  thread_count  INT NOT NULL DEFAULT 0,
  created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE forum.thread_tags (
  thread_id     BIGINT NOT NULL REFERENCES forum.threads(id) ON DELETE CASCADE,
  tag_id        BIGINT NOT NULL REFERENCES forum.tags(id) ON DELETE CASCADE,
  PRIMARY KEY (thread_id, tag_id)
);

CREATE INDEX ON forum.thread_tags (tag_id);

-- ============================================================================
-- 2.5 Post revisions (edit history)
-- ============================================================================
CREATE TABLE forum.post_revisions (
  id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  post_type   TEXT NOT NULL CHECK (post_type IN ('thread', 'comment')),
  post_id     BIGINT NOT NULL,
  seq         INT NOT NULL,
  editor_id   BIGINT NOT NULL REFERENCES identity.accounts(id),
  old_title   TEXT,
  old_body    TEXT NOT NULL,
  created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (post_type, post_id, seq)
);

-- ============================================================================
-- 2.6 Read tracking
-- ============================================================================
CREATE TABLE forum.thread_reads (
  account_id            BIGINT NOT NULL REFERENCES identity.accounts(id),
  thread_id             BIGINT NOT NULL REFERENCES forum.threads(id) ON DELETE CASCADE,
  last_read_comment_id  BIGINT,
  updated_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (account_id, thread_id)
);

-- ============================================================================
-- 2.7 Subscriptions (watching / tracking / muted)
-- ============================================================================
CREATE TABLE forum.subscriptions (
  account_id    BIGINT NOT NULL REFERENCES identity.accounts(id),
  target_type   TEXT NOT NULL CHECK (target_type IN ('board', 'thread')),
  target_id     BIGINT NOT NULL,
  level         TEXT NOT NULL CHECK (level IN ('watching', 'tracking', 'muted')),
  created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (account_id, target_type, target_id)
);

CREATE INDEX ON forum.subscriptions (target_type, target_id, level);

-- ============================================================================
-- 2.8 Flags (reports)
-- ============================================================================
CREATE TABLE forum.flags (
  id            BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  target_type   TEXT NOT NULL CHECK (target_type IN ('thread', 'comment')),
  target_id     BIGINT NOT NULL,
  reporter_id   BIGINT NOT NULL REFERENCES identity.accounts(id),
  reason        TEXT NOT NULL CHECK (reason IN ('spam', 'abuse', 'off_topic', 'illegal', 'other')),
  note          TEXT,
  weight        REAL NOT NULL DEFAULT 1.0,
  status        TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'upheld', 'rejected', 'ignored')),
  handled_by    BIGINT REFERENCES identity.accounts(id),
  handled_at    TIMESTAMPTZ,
  created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (target_type, target_id, reporter_id)
);

CREATE INDEX ON forum.flags (status, created_at DESC);

-- ============================================================================
-- 2.9 Watched words (sensitive word filter)
-- ============================================================================
CREATE TABLE forum.watched_words (
  id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  word        TEXT UNIQUE NOT NULL,
  action      TEXT NOT NULL CHECK (action IN ('block', 'censor', 'queue')),
  created_by  BIGINT REFERENCES identity.accounts(id),
  created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ============================================================================
-- 2.10 Mod actions (staff action log)
-- ============================================================================
CREATE TABLE forum.mod_actions (
  id            BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  actor_id      BIGINT NOT NULL REFERENCES identity.accounts(id),
  action        TEXT NOT NULL,
  target_type   TEXT NOT NULL,
  target_id     BIGINT NOT NULL,
  reason        TEXT,
  metadata      JSONB,
  created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX ON forum.mod_actions (created_at DESC);

-- ============================================================================
-- 2.11 Bookmarks
-- ============================================================================
CREATE TABLE forum.bookmarks (
  account_id    BIGINT NOT NULL REFERENCES identity.accounts(id),
  target_type   TEXT NOT NULL CHECK (target_type IN ('thread', 'comment')),
  target_id     BIGINT NOT NULL,
  note          TEXT,
  created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (account_id, target_type, target_id)
);

CREATE INDEX ON forum.bookmarks (account_id, created_at DESC);

-- ============================================================================
-- 2.12 User stats (trust level data source)
-- ============================================================================
CREATE TABLE forum.user_stats (
  account_id        BIGINT PRIMARY KEY REFERENCES identity.accounts(id),
  threads_created   INT NOT NULL DEFAULT 0,
  comments_created  INT NOT NULL DEFAULT 0,
  votes_cast        INT NOT NULL DEFAULT 0,
  votes_received    INT NOT NULL DEFAULT 0,
  flags_upheld      INT NOT NULL DEFAULT 0,
  flagged_upheld    INT NOT NULL DEFAULT 0,
  last_posted_at    TIMESTAMPTZ,
  updated_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ============================================================================
-- 2.13 Identity changes: trust_level + sanctions
-- ============================================================================
ALTER TABLE identity.accounts
  ADD COLUMN trust_level SMALLINT NOT NULL DEFAULT 0;

CREATE TABLE identity.sanctions (
  id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  account_id  BIGINT NOT NULL REFERENCES identity.accounts(id),
  kind        TEXT NOT NULL CHECK (kind IN ('silence', 'suspend')),
  reason      TEXT NOT NULL,
  issued_by   BIGINT NOT NULL REFERENCES identity.accounts(id),
  starts_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
  ends_at     TIMESTAMPTZ,
  revoked_at  TIMESTAMPTZ,
  revoked_by  BIGINT REFERENCES identity.accounts(id),
  created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX ON identity.sanctions (account_id, ends_at);

-- ============================================================================
-- 2.14 Notification prefs
-- ============================================================================
CREATE TABLE forum.notification_prefs (
  account_id  BIGINT PRIMARY KEY REFERENCES identity.accounts(id),
  prefs       JSONB NOT NULL DEFAULT '{}'::jsonb,
  updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ============================================================================
-- 2.15 Notifications: aggregation key
-- ============================================================================
ALTER TABLE forum.notifications
  ADD COLUMN aggregation_key TEXT;

CREATE INDEX ON forum.notifications (account_id, aggregation_key, created_at DESC);

-- ============================================================================
-- 2.16 New indexes for read paths
-- ============================================================================
CREATE INDEX ON forum.threads (board_id, pinned_at DESC NULLS LAST, last_activity_at DESC)
  WHERE deleted_at IS NULL;

CREATE INDEX ON forum.threads (pinned_globally DESC, last_activity_at DESC)
  WHERE pinned_globally = TRUE AND deleted_at IS NULL;

CREATE INDEX ON forum.threads (author_id, created_at DESC);
