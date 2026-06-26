-- 0002_escrow_selection.sql — credit escrow market + 选课 (PK) mirror tables.
-- Append-only. Money still flows through credit.ledger (escrow_hold / escrow_release);
-- these tables only track escrow state and listing metadata, never balances.

-- ============================ credit: escrow market ============================
CREATE TYPE credit.task_status     AS ENUM ('open', 'in_progress', 'submitted', 'completed', 'cancelled');
CREATE TYPE credit.product_status  AS ENUM ('on_sale', 'off_sale', 'sold_out');
CREATE TYPE credit.purchase_status AS ENUM ('pending', 'accepted', 'delivered', 'completed', 'cancelled');

CREATE TABLE credit.tasks (
  id            BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  creator_id    BIGINT NOT NULL REFERENCES identity.accounts(id),
  acceptor_id   BIGINT REFERENCES identity.accounts(id),
  title         TEXT NOT NULL,
  description   TEXT,
  reward_amount BIGINT NOT NULL CHECK (reward_amount > 0),
  contact_info  TEXT,                       -- only exposed to controlled parties
  status        credit.task_status NOT NULL DEFAULT 'open',
  hold_tx_id    TEXT,                        -- ledger escrow_hold reference
  created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX ON credit.tasks (status, created_at DESC);
CREATE INDEX ON credit.tasks (creator_id);
CREATE INDEX ON credit.tasks (acceptor_id);

CREATE TABLE credit.products (
  id            BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  seller_id     BIGINT NOT NULL REFERENCES identity.accounts(id),
  title         TEXT NOT NULL,
  description   TEXT,
  price         BIGINT NOT NULL CHECK (price > 0),
  stock         INT NOT NULL DEFAULT 0,
  delivery_info TEXT,                        -- only exposed to order parties
  status        credit.product_status NOT NULL DEFAULT 'on_sale',
  created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX ON credit.products (status, created_at DESC);
CREATE INDEX ON credit.products (seller_id);

CREATE TABLE credit.purchases (
  id           BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  product_id   BIGINT NOT NULL REFERENCES credit.products(id),
  buyer_id     BIGINT NOT NULL REFERENCES identity.accounts(id),
  seller_id    BIGINT NOT NULL REFERENCES identity.accounts(id),
  amount       BIGINT NOT NULL CHECK (amount > 0),
  status       credit.purchase_status NOT NULL DEFAULT 'pending',
  hold_tx_id   TEXT,                         -- ledger escrow_hold reference
  created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
  completed_at TIMESTAMPTZ
);
CREATE INDEX ON credit.purchases (buyer_id);
CREATE INDEX ON credit.purchases (seller_id);
CREATE INDEX ON credit.purchases (product_id);

-- ============================ selection (选课/PK mirror) ============================
-- Read-only mirror of 一系统, refreshed by the sync worker. Catalogue data only.
CREATE SCHEMA IF NOT EXISTS selection;

CREATE TABLE selection.calendars (
  id         BIGINT PRIMARY KEY,
  name       TEXT NOT NULL,
  is_current BOOLEAN NOT NULL DEFAULT false
);

CREATE TABLE selection.campuses (
  id   BIGINT PRIMARY KEY,
  name TEXT NOT NULL
);

CREATE TABLE selection.faculties (
  id        BIGINT PRIMARY KEY,
  name      TEXT NOT NULL,
  campus_id BIGINT REFERENCES selection.campuses(id)
);

CREATE TABLE selection.majors (
  id         BIGINT PRIMARY KEY,
  name       TEXT NOT NULL,
  faculty_id BIGINT REFERENCES selection.faculties(id),
  grade      TEXT
);

CREATE TABLE selection.course_natures (
  id   BIGINT PRIMARY KEY,
  name TEXT NOT NULL
);

CREATE TABLE selection.courses (
  id           BIGINT PRIMARY KEY,
  code         TEXT NOT NULL,
  name         TEXT NOT NULL,
  credit       REAL DEFAULT 0,
  nature_id    BIGINT REFERENCES selection.course_natures(id),
  calendar_id  BIGINT REFERENCES selection.calendars(id),
  campus_id    BIGINT REFERENCES selection.campuses(id),
  teacher_name TEXT
);
CREATE INDEX ON selection.courses (code);
CREATE INDEX ON selection.courses (calendar_id);

CREATE TABLE selection.major_courses (
  major_id  BIGINT NOT NULL REFERENCES selection.majors(id),
  course_id BIGINT NOT NULL REFERENCES selection.courses(id),
  grade     TEXT,
  PRIMARY KEY (major_id, course_id)
);

CREATE TABLE selection.timeslots (
  id           BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  course_id    BIGINT NOT NULL REFERENCES selection.courses(id),
  teacher_name TEXT,
  weekday      INT,
  start_slot   INT,
  end_slot     INT,
  weeks        TEXT,
  location     TEXT
);
CREATE INDEX ON selection.timeslots (course_id);
CREATE INDEX ON selection.timeslots (weekday, start_slot, end_slot);

CREATE TABLE selection.fetchlog (
  id         BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  source     TEXT NOT NULL,
  fetched_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
