-- 0060_activity_check_ins.sql — idempotent daily check-ins on the canonical
-- Asia/Shanghai activity calendar. Check-ins are immutable source facts and
-- participate in the same versioned scoring policy as other contributions.

DO $$
BEGIN
  IF EXISTS (SELECT 1 FROM forum.boards WHERE min_trust_to_post NOT BETWEEN 1 AND 6) THEN
    RAISE EXCEPTION 'forum.boards contains min_trust_to_post outside the unified 1-6 range';
  END IF;
  IF EXISTS (SELECT 1 FROM identity.accounts WHERE trust_level NOT BETWEEN 0 AND 6) THEN
    RAISE EXCEPTION 'identity.accounts contains trust_level outside the unified 0-6 range';
  END IF;
END $$;

ALTER TABLE forum.boards
  ALTER COLUMN min_trust_to_post SET DEFAULT 1,
  ADD CONSTRAINT forum_boards_min_trust_to_post_check
    CHECK (min_trust_to_post BETWEEN 1 AND 6);

ALTER TABLE identity.accounts
  ADD CONSTRAINT identity_accounts_trust_level_check CHECK (trust_level BETWEEN 0 AND 6);

ALTER TABLE activity.events
  DROP CONSTRAINT events_kind_check;

ALTER TABLE activity.events
  ADD CONSTRAINT events_kind_check
  CHECK (kind IN ('thread', 'comment', 'like', 'check_in'));

ALTER TABLE activity.daily_counts
  ADD COLUMN check_ins INT NOT NULL DEFAULT 0 CHECK (check_ins BETWEEN 0 AND 1),
  ADD COLUMN score BIGINT NOT NULL DEFAULT 0 CHECK (score >= 0);

ALTER TABLE activity.score_policies
  ADD COLUMN check_in_weight INT NOT NULL DEFAULT 1
  CHECK (check_in_weight BETWEEN 0 AND 1000);

ALTER TABLE activity.account_trust_progress
  ADD COLUMN last_scheduled_evaluation_date DATE,
  ADD COLUMN promotion_blocked_until TIMESTAMPTZ,
  ADD COLUMN promotion_score_floor BIGINT CHECK (
    promotion_score_floor IS NULL OR promotion_score_floor >= 0
  );

ALTER TABLE activity.trust_level_policies
  ADD COLUMN demotion_cooldown_days INT NOT NULL DEFAULT 7
  CHECK (demotion_cooldown_days BETWEEN 0 AND 365);

CREATE TABLE activity.trust_evaluation_runs (
  activity_date   DATE PRIMARY KEY,
  status          TEXT NOT NULL CHECK (status IN ('queued', 'running', 'completed', 'failed')),
  lease_token     UUID,
  lease_expires_at TIMESTAMPTZ,
  attempts        INT NOT NULL DEFAULT 0 CHECK (attempts >= 0),
  upgraded_count  INT NOT NULL DEFAULT 0 CHECK (upgraded_count >= 0),
  error_code      TEXT,
  started_at      TIMESTAMPTZ,
  completed_at    TIMESTAMPTZ,
  updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK (
    (status = 'running' AND lease_token IS NOT NULL AND lease_expires_at IS NOT NULL)
    OR (status <> 'running' AND lease_token IS NULL AND lease_expires_at IS NULL)
  )
);

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
  'repaired missing trust initialization history',
  'trust:init-repair:' || progress.account_id::text
FROM activity.account_trust_progress progress
WHERE NOT EXISTS (
  SELECT 1
  FROM activity.trust_level_events event
  WHERE event.account_id = progress.account_id
    AND event.event_kind IN ('registration', 'backfill_initialized')
)
ON CONFLICT (event_key) DO NOTHING;

CREATE TABLE activity.check_ins (
  account_id      BIGINT NOT NULL REFERENCES identity.accounts(id) ON DELETE CASCADE,
  activity_date   DATE NOT NULL,
  checked_in_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (account_id, activity_date),
  CHECK (activity_date = (checked_in_at AT TIME ZONE 'Asia/Shanghai')::date)
);

CREATE TABLE activity.account_scores (
  account_id           BIGINT PRIMARY KEY REFERENCES identity.accounts(id) ON DELETE CASCADE,
  qualifying_score     BIGINT NOT NULL DEFAULT 0 CHECK (qualifying_score >= 0),
  score_policy_version BIGINT NOT NULL REFERENCES activity.score_policies(version),
  trust_policy_version BIGINT NOT NULL REFERENCES activity.trust_level_policies(version),
  updated_at           TIMESTAMPTZ NOT NULL DEFAULT now()
);

WITH policy AS (
  SELECT
    score.version AS score_policy_version,
    trust.version AS trust_policy_version,
    score.thread_weight,
    score.comment_weight,
    score.like_weight,
    score.check_in_weight,
    trust.like_daily_cap
  FROM activity.trust_level_policies trust
  INNER JOIN activity.score_policies score ON score.version = trust.score_policy_version
  ORDER BY trust.version DESC
  LIMIT 1
)
UPDATE activity.daily_counts counts
SET score = (
  counts.threads_created::bigint * policy.thread_weight
  + counts.comments_created::bigint * policy.comment_weight
  + LEAST(counts.likes_given::bigint * policy.like_weight, policy.like_daily_cap::bigint)
  + counts.check_ins::bigint * policy.check_in_weight
)::bigint
FROM policy;

WITH policy AS (
  SELECT
    score.version AS score_policy_version,
    trust.version AS trust_policy_version
  FROM activity.trust_level_policies trust
  INNER JOIN activity.score_policies score ON score.version = trust.score_policy_version
  ORDER BY trust.version DESC
  LIMIT 1
)
INSERT INTO activity.account_scores (
  account_id,
  qualifying_score,
  score_policy_version,
  trust_policy_version
)
SELECT
  counts.account_id,
  COALESCE(SUM(counts.score), 0)::bigint,
  policy.score_policy_version,
  policy.trust_policy_version
FROM activity.daily_counts counts
CROSS JOIN policy
GROUP BY counts.account_id, policy.score_policy_version, policy.trust_policy_version;

-- The lifetime totals introduced by 0059 duplicated daily_counts without
-- supporting the per-day like cap. The versioned score must remain derived
-- from the one authoritative daily projection until a durable reprojected
-- score cache exists.
DROP TABLE activity.account_totals;
