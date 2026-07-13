-- 0059_activity_trust_levels.sql — unified 0–6 trust levels owned by Activity.
-- Activity holds totals, versioned thresholds, current progress, and append-only
-- events. identity.accounts.trust_level remains a compatibility projection that
-- only Activity may write. Registered accounts start at Lv.1; Lv.0 is visitor UI.

CREATE TABLE activity.account_totals (
  account_id       BIGINT PRIMARY KEY REFERENCES identity.accounts(id) ON DELETE CASCADE,
  threads_created  INT NOT NULL DEFAULT 0 CHECK (threads_created >= 0),
  comments_created INT NOT NULL DEFAULT 0 CHECK (comments_created >= 0),
  likes_given      INT NOT NULL DEFAULT 0 CHECK (likes_given >= 0),
  updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE activity.trust_level_policies (
  version              BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  score_policy_version BIGINT NOT NULL REFERENCES activity.score_policies(version),
  threshold_level_2    INT NOT NULL CHECK (threshold_level_2 > 0),
  threshold_level_3    INT NOT NULL,
  threshold_level_4    INT NOT NULL,
  threshold_level_5    INT NOT NULL,
  threshold_level_6    INT NOT NULL,
  like_daily_cap       INT NOT NULL DEFAULT 20 CHECK (like_daily_cap BETWEEN 0 AND 100000),
  reason               TEXT NOT NULL CHECK (length(reason) BETWEEN 1 AND 500),
  changed_by           BIGINT REFERENCES identity.accounts(id),
  created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK (
    threshold_level_3 > threshold_level_2
    AND threshold_level_4 > threshold_level_3
    AND threshold_level_5 > threshold_level_4
    AND threshold_level_6 > threshold_level_5
  )
);

CREATE TABLE activity.account_trust_progress (
  account_id         BIGINT PRIMARY KEY REFERENCES identity.accounts(id) ON DELETE CASCADE,
  trust_level        SMALLINT NOT NULL CHECK (trust_level BETWEEN 1 AND 6),
  qualifying_score   BIGINT NOT NULL DEFAULT 0 CHECK (qualifying_score >= 0),
  policy_version     BIGINT NOT NULL REFERENCES activity.trust_level_policies(version),
  override_level     SMALLINT CHECK (override_level IS NULL OR override_level BETWEEN 1 AND 6),
  override_reason    TEXT,
  override_by        BIGINT REFERENCES identity.accounts(id),
  override_at        TIMESTAMPTZ,
  last_evaluated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK (
    (
      override_level IS NULL
      AND override_reason IS NULL
      AND override_by IS NULL
      AND override_at IS NULL
    )
    OR (
      override_level IS NOT NULL
      AND override_reason IS NOT NULL
      AND length(override_reason) BETWEEN 3 AND 500
      AND override_by IS NOT NULL
      AND override_at IS NOT NULL
    )
  )
);

CREATE TABLE activity.trust_level_events (
  id                   BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  account_id           BIGINT NOT NULL REFERENCES identity.accounts(id) ON DELETE CASCADE,
  event_kind           TEXT NOT NULL CHECK (
    event_kind IN (
      'upgrade',
      'demotion',
      'manual_set',
      'override_clear',
      'backfill_initialized',
      'registration'
    )
  ),
  from_level           SMALLINT NOT NULL CHECK (from_level BETWEEN 0 AND 6),
  to_level             SMALLINT NOT NULL CHECK (to_level BETWEEN 1 AND 6),
  qualifying_score     BIGINT NOT NULL DEFAULT 0 CHECK (qualifying_score >= 0),
  policy_version       BIGINT NOT NULL REFERENCES activity.trust_level_policies(version),
  actor_kind           TEXT NOT NULL CHECK (actor_kind IN ('system', 'account')),
  actor_account_id     BIGINT REFERENCES identity.accounts(id),
  reason               TEXT,
  governance_event_id  BIGINT,
  event_key            TEXT NOT NULL UNIQUE,
  created_at           TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX activity_trust_level_events_account_idx
  ON activity.trust_level_events (account_id, id DESC);

CREATE UNIQUE INDEX activity_trust_level_events_governance_demotion_idx
  ON activity.trust_level_events (governance_event_id)
  WHERE event_kind = 'demotion' AND governance_event_id IS NOT NULL;

INSERT INTO activity.trust_level_policies (
  score_policy_version,
  threshold_level_2,
  threshold_level_3,
  threshold_level_4,
  threshold_level_5,
  threshold_level_6,
  like_daily_cap,
  reason
)
SELECT
  score.version,
  30,
  120,
  400,
  1200,
  3000,
  20,
  'initial unified trust policy'
FROM activity.score_policies score
ORDER BY score.version DESC
LIMIT 1;

-- Board thresholds shift +1 so old TL1 boards still require the second tier.
UPDATE forum.boards
SET min_trust_to_post = LEAST(min_trust_to_post + 1, 6);

ALTER TABLE identity.accounts
  ALTER COLUMN trust_level SET DEFAULT 1;

UPDATE identity.accounts
SET trust_level = 1
WHERE trust_level <= 0
  AND status <> 'purged';

INSERT INTO activity.account_totals (
  account_id,
  threads_created,
  comments_created,
  likes_given
)
SELECT
  counts.account_id,
  COALESCE(SUM(counts.threads_created), 0)::int,
  COALESCE(SUM(counts.comments_created), 0)::int,
  COALESCE(SUM(counts.likes_given), 0)::int
FROM activity.daily_counts counts
GROUP BY counts.account_id
ON CONFLICT (account_id) DO UPDATE
SET threads_created = EXCLUDED.threads_created,
    comments_created = EXCLUDED.comments_created,
    likes_given = EXCLUDED.likes_given,
    updated_at = now();

WITH policy AS (
  SELECT
    trust.version AS trust_policy_version,
    score.thread_weight,
    score.comment_weight,
    score.like_weight,
    trust.like_daily_cap,
    trust.threshold_level_2,
    trust.threshold_level_3,
    trust.threshold_level_4,
    trust.threshold_level_5,
    trust.threshold_level_6
  FROM activity.trust_level_policies trust
  INNER JOIN activity.score_policies score
    ON score.version = trust.score_policy_version
  ORDER BY trust.version DESC
  LIMIT 1
),
scores AS (
  SELECT
    account.id AS account_id,
    COALESCE((
      SELECT SUM(
        counts.threads_created * policy.thread_weight
        + counts.comments_created * policy.comment_weight
        + LEAST(
            counts.likes_given * policy.like_weight,
            policy.like_daily_cap
          )
      )
      FROM activity.daily_counts counts
      CROSS JOIN policy
      WHERE counts.account_id = account.id
    ), 0)::bigint AS qualifying_score
  FROM identity.accounts account
  WHERE account.status <> 'purged'
),
levels AS (
  SELECT
    scores.account_id,
    scores.qualifying_score,
    CASE
      WHEN scores.qualifying_score >= policy.threshold_level_6 THEN 6
      WHEN scores.qualifying_score >= policy.threshold_level_5 THEN 5
      WHEN scores.qualifying_score >= policy.threshold_level_4 THEN 4
      WHEN scores.qualifying_score >= policy.threshold_level_3 THEN 3
      WHEN scores.qualifying_score >= policy.threshold_level_2 THEN 2
      ELSE 1
    END::smallint AS trust_level,
    policy.trust_policy_version
  FROM scores
  CROSS JOIN policy
)
INSERT INTO activity.account_trust_progress (
  account_id,
  trust_level,
  qualifying_score,
  policy_version
)
SELECT
  levels.account_id,
  levels.trust_level,
  levels.qualifying_score,
  levels.trust_policy_version
FROM levels
ON CONFLICT (account_id) DO NOTHING;

INSERT INTO activity.trust_level_events (
  account_id,
  event_kind,
  from_level,
  to_level,
  qualifying_score,
  policy_version,
  actor_kind,
  reason,
  event_key
)
SELECT
  progress.account_id,
  'backfill_initialized',
  0,
  progress.trust_level,
  progress.qualifying_score,
  progress.policy_version,
  'system',
  'unified trust level backfill',
  'trust:backfill:' || progress.account_id::text
FROM activity.account_trust_progress progress
ON CONFLICT (event_key) DO NOTHING;

UPDATE identity.accounts account
SET trust_level = progress.trust_level
FROM activity.account_trust_progress progress
WHERE account.id = progress.account_id
  AND account.status <> 'purged'
  AND account.trust_level IS DISTINCT FROM progress.trust_level;
