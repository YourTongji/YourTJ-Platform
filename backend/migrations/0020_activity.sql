-- 0020_activity.sql — daily user activity and versioned scoring policy.
-- Counts are projected from idempotent activation/reversal events. Scores are
-- computed with the current policy so changing weights reinterprets history
-- without rewriting the underlying counts.

CREATE SCHEMA IF NOT EXISTS activity;

CREATE TABLE activity.events (
  id                 BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  event_key          TEXT NOT NULL UNIQUE,
  source_key         TEXT NOT NULL,
  generation         INT NOT NULL CHECK (generation > 0),
  account_id         BIGINT NOT NULL REFERENCES identity.accounts(id) ON DELETE CASCADE,
  kind               TEXT NOT NULL CHECK (kind IN ('thread', 'comment', 'like')),
  delta              SMALLINT NOT NULL CHECK (delta IN (-1, 1)),
  activity_date      DATE NOT NULL,
  occurred_at        TIMESTAMPTZ NOT NULL,
  reverses_event_id  BIGINT REFERENCES activity.events(id),
  created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (source_key, generation, delta),
  UNIQUE (reverses_event_id),
  CHECK (
    (delta = 1 AND reverses_event_id IS NULL)
    OR (delta = -1 AND reverses_event_id IS NOT NULL)
  )
);

CREATE INDEX activity_events_source_idx
  ON activity.events (source_key, generation DESC);
CREATE INDEX activity_events_account_date_idx
  ON activity.events (account_id, activity_date);

CREATE TABLE activity.daily_counts (
  account_id       BIGINT NOT NULL REFERENCES identity.accounts(id) ON DELETE CASCADE,
  activity_date    DATE NOT NULL,
  threads_created  INT NOT NULL DEFAULT 0 CHECK (threads_created >= 0),
  comments_created INT NOT NULL DEFAULT 0 CHECK (comments_created >= 0),
  likes_given      INT NOT NULL DEFAULT 0 CHECK (likes_given >= 0),
  updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (account_id, activity_date)
);

CREATE INDEX activity_daily_counts_date_idx
  ON activity.daily_counts (activity_date, account_id);

CREATE TABLE activity.score_policies (
  version         BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  thread_weight   INT NOT NULL CHECK (thread_weight BETWEEN 0 AND 1000),
  comment_weight  INT NOT NULL CHECK (comment_weight BETWEEN 0 AND 1000),
  like_weight     INT NOT NULL CHECK (like_weight BETWEEN 0 AND 1000),
  reason          TEXT NOT NULL CHECK (length(reason) BETWEEN 1 AND 500),
  changed_by      BIGINT REFERENCES identity.accounts(id),
  created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO activity.score_policies
  (thread_weight, comment_weight, like_weight, reason)
VALUES (10, 3, 1, 'initial activity policy');
