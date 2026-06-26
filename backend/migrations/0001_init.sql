-- 0001_init.sql — initial schema for YourTJ Platform (PolarDB / PostgreSQL).
-- Single database, one schema per domain. Migrations are append-only: never edit
-- an applied migration; add a new numbered file instead.

CREATE EXTENSION IF NOT EXISTS citext;

CREATE SCHEMA IF NOT EXISTS identity;
CREATE SCHEMA IF NOT EXISTS courses;
CREATE SCHEMA IF NOT EXISTS reviews;
CREATE SCHEMA IF NOT EXISTS credit;
CREATE SCHEMA IF NOT EXISTS forum;

-- ============================ identity ============================
CREATE TYPE identity.account_role   AS ENUM ('user', 'mod', 'admin');
CREATE TYPE identity.account_status AS ENUM ('active', 'suspended', 'deleted');

CREATE TABLE identity.accounts (
  id                BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  email             CITEXT UNIQUE NOT NULL,            -- @tongji.edu.cn; encrypt at rest
  email_verified_at TIMESTAMPTZ,
  handle            CITEXT UNIQUE NOT NULL,            -- public pseudonym
  avatar_url        TEXT,
  role              identity.account_role   NOT NULL DEFAULT 'user',
  status            identity.account_status NOT NULL DEFAULT 'active',
  created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
  last_active_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE identity.account_keys (
  account_id  BIGINT NOT NULL REFERENCES identity.accounts(id),
  public_key  TEXT   NOT NULL UNIQUE,                  -- base64 Ed25519 public key
  algo        TEXT   NOT NULL DEFAULT 'ed25519',
  created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  revoked_at  TIMESTAMPTZ,
  PRIMARY KEY (account_id, public_key)
);

CREATE TABLE identity.email_codes (
  email      CITEXT NOT NULL,
  code_hash  TEXT   NOT NULL,
  expires_at TIMESTAMPTZ NOT NULL,
  attempts   INT    NOT NULL DEFAULT 0,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX ON identity.email_codes (email, expires_at);

CREATE TABLE identity.sessions (
  id           BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  account_id   BIGINT NOT NULL REFERENCES identity.accounts(id),
  refresh_hash TEXT   NOT NULL,
  user_agent   TEXT,
  ip           INET,
  expires_at   TIMESTAMPTZ NOT NULL,
  revoked_at   TIMESTAMPTZ,
  created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX ON identity.sessions (account_id);

CREATE TABLE identity.legacy_wallet_links (
  legacy_user_hash TEXT PRIMARY KEY,
  account_id       BIGINT REFERENCES identity.accounts(id),
  claimed_at       TIMESTAMPTZ
);

-- ============================ courses ============================
CREATE TABLE courses.teachers (
  id            BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  tid           TEXT,
  name          TEXT NOT NULL,
  title         TEXT,
  department    TEXT,
  name_pinyin   TEXT,                                  -- precomputed by sync job
  name_initials TEXT
);

CREATE TABLE courses.courses (
  id              BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  code            TEXT NOT NULL,
  name            TEXT NOT NULL,
  credit          REAL DEFAULT 0,
  department      TEXT,
  teacher_id      BIGINT REFERENCES courses.teachers(id),
  review_count    INT  NOT NULL DEFAULT 0,             -- maintained incrementally
  review_avg      REAL NOT NULL DEFAULT 0,
  name_pinyin     TEXT,
  name_initials   TEXT,
  search_keywords TEXT,
  is_legacy       INT DEFAULT 0,
  is_icu          INT DEFAULT 0
);
CREATE INDEX ON courses.courses (code);
CREATE INDEX ON courses.courses (department);

CREATE TABLE courses.course_aliases (
  course_id BIGINT NOT NULL REFERENCES courses.courses(id),
  alias     TEXT NOT NULL,
  PRIMARY KEY (course_id, alias)
);
-- 选课(PK) 一系统 mirror tables (calendar/campus/faculty/major/coursedetail/
-- teacher_timeslots …) are migrated as-is in a later migration.

-- ============================ reviews ============================
CREATE TYPE reviews.review_status AS ENUM ('visible', 'hidden', 'pending');

CREATE TABLE reviews.reviews (
  id               BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  course_id        BIGINT NOT NULL REFERENCES courses.courses(id) ON DELETE CASCADE,
  account_id       BIGINT REFERENCES identity.accounts(id),
  rating           INT NOT NULL CHECK (rating BETWEEN 0 AND 5),
  comment          TEXT,
  score            TEXT,
  semester         TEXT,
  approve_count    INT NOT NULL DEFAULT 0,
  disapprove_count INT NOT NULL DEFAULT 0,
  status           reviews.review_status NOT NULL DEFAULT 'visible',
  is_legacy        INT DEFAULT 0,
  is_icu           INT DEFAULT 0,
  created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX ON reviews.reviews (course_id, status, created_at DESC);
CREATE INDEX ON reviews.reviews (account_id);

CREATE TABLE reviews.review_likes (
  review_id  BIGINT NOT NULL REFERENCES reviews.reviews(id) ON DELETE CASCADE,
  account_id BIGINT NOT NULL REFERENCES identity.accounts(id),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (review_id, account_id)
);

CREATE TABLE reviews.review_reports (
  id                  BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  review_id           BIGINT NOT NULL REFERENCES reviews.reviews(id) ON DELETE CASCADE,
  reporter_account_id BIGINT NOT NULL REFERENCES identity.accounts(id),
  reason              TEXT NOT NULL,
  status              TEXT NOT NULL DEFAULT 'open',
  admin_note          TEXT,
  created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
  resolved_at         TIMESTAMPTZ,
  UNIQUE (review_id, reporter_account_id)
);

-- ============================ credit (Web2.5 ledger) ============================
-- credit.ledger is the single source of truth: append-only, monotonic seq,
-- prev_hash chained, each entry Ed25519-signed. Balances are a derived cache.
-- COMPLIANCE: closed-loop only — no recharge / withdraw / fiat conversion /
-- unrestricted transfer. Value moves only via mint / escrow / tip / bounty.
CREATE TABLE credit.ledger (
  seq          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  tx_id        TEXT UNIQUE NOT NULL,
  type         TEXT NOT NULL,            -- mint/transfer/escrow_hold/escrow_release/admin_adjust
  from_account BIGINT REFERENCES identity.accounts(id),   -- NULL for mint
  to_account   BIGINT REFERENCES identity.accounts(id),
  amount       BIGINT NOT NULL CHECK (amount > 0),
  nonce        TEXT NOT NULL,
  metadata     JSONB,
  signer       TEXT NOT NULL,            -- 'system' or the originator's Ed25519 public key
  signature    TEXT NOT NULL,            -- Ed25519(canonical(payload))
  prev_hash    TEXT NOT NULL,            -- previous entry hash (genesis = 64 zeros)
  hash         TEXT NOT NULL,            -- SHA256(canonical(payload) || prev_hash)
  created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX ON credit.ledger (from_account);
CREATE INDEX ON credit.ledger (to_account);

CREATE TABLE credit.wallets (
  account_id     BIGINT PRIMARY KEY REFERENCES identity.accounts(id),
  balance        BIGINT NOT NULL DEFAULT 0,     -- derived cache; authoritative is ledger
  last_seq       BIGINT NOT NULL DEFAULT 0,     -- ledger seq settled into this cache
  last_active_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE credit.checkpoints (
  id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  up_to_seq   BIGINT NOT NULL,
  merkle_root TEXT NOT NULL,
  system_sig  TEXT NOT NULL,                    -- system key signature; not anchored to a real chain
  created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ============================ forum (Phase B) ============================
CREATE TABLE forum.boards (
  id   BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  slug TEXT UNIQUE,
  name TEXT
);

CREATE TABLE forum.threads (
  id               BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  board_id         BIGINT REFERENCES forum.boards(id),
  author_id        BIGINT REFERENCES identity.accounts(id),
  title            TEXT NOT NULL,
  body             TEXT,
  reply_count      INT DEFAULT 0,
  vote_count       INT DEFAULT 0,
  hot_score        DOUBLE PRECISION DEFAULT 0,         -- refreshed by job → Redis ZSET
  status           TEXT DEFAULT 'visible',
  created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
  last_activity_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX ON forum.threads (board_id, last_activity_at DESC);

CREATE TABLE forum.comments (
  id         BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  thread_id  BIGINT REFERENCES forum.threads(id),
  parent_id  BIGINT REFERENCES forum.comments(id),
  path       TEXT,                                     -- materialized path e.g. '0003.0007'
  author_id  BIGINT REFERENCES identity.accounts(id),
  body       TEXT,
  vote_count INT DEFAULT 0,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX ON forum.comments (thread_id, path);

CREATE TABLE forum.notifications (
  id         BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  account_id BIGINT REFERENCES identity.accounts(id),
  type       TEXT,
  payload    JSONB,
  read_at    TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX ON forum.notifications (account_id, created_at DESC);
