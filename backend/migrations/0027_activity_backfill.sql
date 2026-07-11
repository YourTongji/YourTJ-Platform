-- 0027_activity_backfill.sql — project existing community contributions.
-- Append-only and idempotent: rerunning the statement inserts no duplicate
-- event keys and therefore cannot increment daily counts twice.

WITH existing_contributions AS (
  SELECT
    'forum_thread:' || thread.id::text AS source_key,
    thread.author_id AS account_id,
    'thread'::text AS kind,
    thread.created_at AS occurred_at
  FROM forum.threads thread
  WHERE thread.author_id IS NOT NULL
    AND thread.status = 'visible'
    AND thread.hidden_at IS NULL
    AND thread.deleted_at IS NULL
    AND thread.archived_at IS NULL

  UNION ALL

  SELECT
    'forum_comment:' || comment.id::text AS source_key,
    comment.author_id AS account_id,
    'comment'::text AS kind,
    comment.created_at AS occurred_at
  FROM forum.comments comment
  JOIN forum.threads thread ON thread.id = comment.thread_id
  WHERE comment.author_id IS NOT NULL
    AND comment.hidden_at IS NULL
    AND comment.deleted_at IS NULL
    AND thread.status = 'visible'
    AND thread.hidden_at IS NULL
    AND thread.deleted_at IS NULL
    AND thread.archived_at IS NULL

  UNION ALL

  SELECT
    'forum_vote:thread:' || vote.post_id::text || ':' || vote.account_id::text
      AS source_key,
    vote.account_id,
    'like'::text AS kind,
    vote.updated_at AS occurred_at
  FROM forum.votes vote
  JOIN forum.threads thread ON thread.id = vote.post_id
  WHERE vote.post_type = 'thread'
    AND vote.value = 1
    AND thread.status = 'visible'
    AND thread.hidden_at IS NULL
    AND thread.deleted_at IS NULL
    AND thread.archived_at IS NULL

  UNION ALL

  SELECT
    'forum_vote:comment:' || vote.post_id::text || ':' || vote.account_id::text AS source_key,
    vote.account_id,
    'like'::text AS kind,
    vote.updated_at AS occurred_at
  FROM forum.votes vote
  JOIN forum.comments comment ON comment.id = vote.post_id
  JOIN forum.threads thread ON thread.id = comment.thread_id
  WHERE vote.post_type = 'comment'
    AND vote.value = 1
    AND comment.hidden_at IS NULL
    AND comment.deleted_at IS NULL
    AND thread.status = 'visible'
    AND thread.hidden_at IS NULL
    AND thread.deleted_at IS NULL
    AND thread.archived_at IS NULL

  UNION ALL

  SELECT
    'review_like:' || review_like.review_id::text || ':' || review_like.account_id::text
      AS source_key,
    review_like.account_id,
    'like'::text AS kind,
    review_like.created_at AS occurred_at
  FROM reviews.review_likes review_like
  JOIN reviews.reviews review ON review.id = review_like.review_id
  WHERE review.status = 'visible'
), inserted_events AS (
  INSERT INTO activity.events (
    event_key,
    source_key,
    generation,
    account_id,
    kind,
    delta,
    activity_date,
    occurred_at
  )
  SELECT
    contribution.source_key || ':1:activate',
    contribution.source_key,
    1,
    contribution.account_id,
    contribution.kind,
    1,
    (contribution.occurred_at AT TIME ZONE 'Asia/Shanghai')::date,
    contribution.occurred_at
  FROM existing_contributions contribution
  ON CONFLICT DO NOTHING
  RETURNING account_id, kind, activity_date
)
INSERT INTO activity.daily_counts (
  account_id,
  activity_date,
  threads_created,
  comments_created,
  likes_given
)
SELECT
  event.account_id,
  event.activity_date,
  COUNT(*) FILTER (WHERE event.kind = 'thread')::int,
  COUNT(*) FILTER (WHERE event.kind = 'comment')::int,
  COUNT(*) FILTER (WHERE event.kind = 'like')::int
FROM inserted_events event
GROUP BY event.account_id, event.activity_date
ON CONFLICT (account_id, activity_date) DO UPDATE
SET threads_created = activity.daily_counts.threads_created + EXCLUDED.threads_created,
    comments_created = activity.daily_counts.comments_created + EXCLUDED.comments_created,
    likes_given = activity.daily_counts.likes_given + EXCLUDED.likes_given,
    updated_at = now();
