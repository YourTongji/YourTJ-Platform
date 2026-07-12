-- Database-only fail-closed preflight for materialized media facts.
-- This does not parse retained Forum Markdown and cannot inspect OSS. Run it after migration 0057,
-- then complete the separate application-level Markdown and OSS inventory reconciliations before
-- setting MEDIA_RETENTION_GC_ENABLED=true.

DO $$
DECLARE
  profile_drift BIGINT;
  promotion_drift BIGINT;
  invalid_draft_references BIGINT;
  draft_fact_drift BIGINT;
  invalid_active_usages BIGINT;
  invalid_active_bindings BIGINT;
  deletion_anomalies BIGINT;
  unredacted_blocked BIGINT;
BEGIN
  WITH expected AS (
    SELECT profile.account_id AS target_id, 'profile_avatar'::text AS target_type,
           profile.avatar_asset_id AS asset_id
    FROM identity.profiles profile WHERE profile.avatar_asset_id IS NOT NULL
    UNION ALL
    SELECT profile.account_id, 'profile_banner', profile.banner_asset_id
    FROM identity.profiles profile WHERE profile.banner_asset_id IS NOT NULL
  ), actual AS (
    SELECT binding.target_id, binding.target_type, binding.asset_id
    FROM media.asset_bindings binding
    WHERE binding.target_type IN ('profile_avatar', 'profile_banner')
      AND binding.detached_at IS NULL
  )
  SELECT count(*)::bigint INTO profile_drift
  FROM expected FULL JOIN actual USING (target_id, target_type, asset_id)
  WHERE expected.asset_id IS NULL OR actual.asset_id IS NULL;

  WITH expected AS (
    SELECT promotion.id AS target_id, promotion.asset_id
    FROM platform.promotions promotion
    WHERE promotion.status <> 'archived' AND promotion.asset_id IS NOT NULL
  ), actual AS (
    SELECT binding.target_id, binding.asset_id
    FROM media.asset_bindings binding
    WHERE binding.target_type = 'platform_promotion' AND binding.detached_at IS NULL
  )
  SELECT count(*)::bigint INTO promotion_drift
  FROM expected FULL JOIN actual USING (target_id, asset_id)
  WHERE expected.asset_id IS NULL OR actual.asset_id IS NULL;

  WITH raw AS (
    SELECT draft.account_id, draft.draft_key, draft.payload->>'kind' AS kind,
           asset.value, asset.ordinality
    FROM forum.drafts draft
    CROSS JOIN LATERAL jsonb_array_elements_text(
      CASE WHEN jsonb_typeof(draft.payload->'attachmentAssetIds') = 'array'
           THEN draft.payload->'attachmentAssetIds' ELSE '[]'::jsonb END
    ) WITH ORDINALITY AS asset(value, ordinality)
  ), valid AS (
    SELECT raw.account_id, raw.draft_key, upload.id AS asset_id, raw.ordinality
    FROM raw
    JOIN media.uploads upload
      ON raw.value = upload.id::text
     AND upload.account_id = raw.account_id
     AND upload.kind = 'image'
     AND upload.status IN ('pending', 'clean', 'blocked')
     AND upload.usage = CASE raw.kind
           WHEN 'thread' THEN 'forum_thread'
           WHEN 'comment' THEN 'forum_comment'
         END
    WHERE raw.kind IN ('thread', 'comment')
      AND raw.ordinality <= CASE raw.kind WHEN 'thread' THEN 8 ELSE 4 END
  )
  SELECT (SELECT count(*) FROM raw) - (SELECT count(*) FROM valid)
    INTO invalid_draft_references;

  WITH expected AS (
    SELECT draft.account_id, draft.draft_key, upload.id AS asset_id,
           CASE draft.payload->>'kind'
             WHEN 'thread' THEN 'forum_thread'
             WHEN 'comment' THEN 'forum_comment'
           END AS target_type,
           (asset.ordinality - 1)::smallint AS position
    FROM forum.drafts draft
    CROSS JOIN LATERAL jsonb_array_elements_text(
      CASE WHEN jsonb_typeof(draft.payload->'attachmentAssetIds') = 'array'
           THEN draft.payload->'attachmentAssetIds' ELSE '[]'::jsonb END
    ) WITH ORDINALITY AS asset(value, ordinality)
    JOIN media.uploads upload
      ON asset.value = upload.id::text
     AND upload.account_id = draft.account_id
     AND upload.kind = 'image'
     AND upload.status IN ('pending', 'clean', 'blocked')
     AND upload.usage = CASE draft.payload->>'kind'
           WHEN 'thread' THEN 'forum_thread'
           WHEN 'comment' THEN 'forum_comment'
         END
    WHERE draft.payload->>'kind' IN ('thread', 'comment')
      AND asset.ordinality <= CASE draft.payload->>'kind' WHEN 'thread' THEN 8 ELSE 4 END
  ), actual AS (
    SELECT account_id, draft_key, asset_id, target_type, position
    FROM media.draft_asset_references
  )
  SELECT count(*)::bigint INTO draft_fact_drift
  FROM expected FULL JOIN actual
    USING (account_id, draft_key, asset_id, target_type, position)
  WHERE expected.asset_id IS NULL OR actual.asset_id IS NULL;

  SELECT count(*)::bigint INTO invalid_active_usages
  FROM media.asset_usages usage
  JOIN media.uploads upload ON upload.id = usage.asset_id
  LEFT JOIN forum.threads thread
    ON usage.target_type = 'forum_thread' AND thread.id = usage.target_id
  LEFT JOIN forum.comments comment
    ON usage.target_type = 'forum_comment' AND comment.id = usage.target_id
  WHERE usage.detached_at IS NULL
    AND (
      upload.account_id <> usage.owner_account_id
      OR upload.kind <> 'image'
      OR upload.status <> 'clean'
      OR upload.usage IS DISTINCT FROM usage.target_type
      OR (usage.target_type = 'forum_thread'
          AND (thread.id IS NULL OR thread.author_id <> usage.owner_account_id
               OR thread.deleted_at IS NOT NULL))
      OR (usage.target_type = 'forum_comment'
          AND (comment.id IS NULL OR comment.author_id <> usage.owner_account_id
               OR comment.deleted_at IS NOT NULL))
    );

  SELECT count(*)::bigint INTO invalid_active_bindings
  FROM media.asset_bindings binding
  JOIN media.uploads upload ON upload.id = binding.asset_id
  LEFT JOIN identity.profiles profile ON profile.account_id = binding.target_id
  LEFT JOIN platform.promotions promotion ON promotion.id = binding.target_id
  WHERE binding.detached_at IS NULL
    AND (
      upload.account_id <> binding.owner_account_id
      OR upload.kind <> 'image'
      OR upload.status <> 'clean'
      OR (binding.target_type = 'profile_avatar'
          AND (profile.account_id IS NULL OR profile.avatar_asset_id <> binding.asset_id
               OR profile.account_id <> binding.owner_account_id
               OR upload.usage IS DISTINCT FROM 'profile_avatar'))
      OR (binding.target_type = 'profile_banner'
          AND (profile.account_id IS NULL OR profile.banner_asset_id <> binding.asset_id
               OR profile.account_id <> binding.owner_account_id
               OR upload.usage IS DISTINCT FROM 'profile_banner'))
      OR (binding.target_type = 'platform_promotion'
          AND (promotion.id IS NULL OR promotion.asset_id <> binding.asset_id
               OR promotion.status = 'archived'))
    );

  SELECT count(*)::bigint INTO deletion_anomalies
  FROM media.uploads upload
  WHERE upload.status = 'quarantined'
    AND NOT EXISTS (
      SELECT 1 FROM media.object_deletion_jobs job
      WHERE job.upload_id = upload.id AND job.status <> 'succeeded'
    );

  SELECT count(*)::bigint INTO unredacted_blocked
  FROM media.uploads upload
  WHERE upload.status = 'blocked'
    AND (upload.redacted_at IS NULL OR upload.oss_key <> 'redacted/' || upload.id
         OR upload.url <> '' OR upload.sha256 <> '');

  RAISE NOTICE
    'media DB preflight: profile=%, promotion=%, invalid_draft=%, draft_fact=%, invalid_usage=%, invalid_binding=%, deletion_anomaly=%, unredacted_blocked=%',
    profile_drift, promotion_drift, invalid_draft_references, draft_fact_drift,
    invalid_active_usages, invalid_active_bindings, deletion_anomalies, unredacted_blocked;
  IF profile_drift + promotion_drift + invalid_draft_references + draft_fact_drift
       + invalid_active_usages + invalid_active_bindings + deletion_anomalies
       + unredacted_blocked <> 0 THEN
    RAISE EXCEPTION 'media retention reconciliation failed; keep GC disabled';
  END IF;
END;
$$;
