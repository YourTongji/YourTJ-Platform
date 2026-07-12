//! Bounded, read-only media state reconciliation for operators.

use shared::AppResult;
use sqlx::PgConnection;

use crate::dto::{ProviderInventoryStatusDto, ReconciliationFindingDto, ReconciliationReportDto};

pub async fn inspect(
    connection: &mut PgConnection,
    cursor: Option<i64>,
    limit: i64,
) -> AppResult<ReconciliationReportDto> {
    let rows: Vec<(i64, Vec<String>)> = sqlx::query_as(
        "SELECT upload.id, issues.issue_codes FROM media.uploads upload \
         LEFT JOIN media.asset_publications publication ON publication.asset_id = upload.id \
         CROSS JOIN LATERAL (SELECT ARRAY_REMOVE(ARRAY[ \
           CASE WHEN publication.asset_id IS NULL THEN 'publication_missing' END, \
           CASE WHEN publication.status = 'published' AND EXISTS( \
             SELECT required.variant_kind FROM (VALUES ('thumb_256'), ('display_1280'), \
               ('full_2048')) required(variant_kind) EXCEPT SELECT variant.variant_kind \
             FROM media.asset_variants variant WHERE variant.asset_id = upload.id \
               AND variant.policy_version = publication.policy_version \
               AND variant.status = 'published') THEN 'published_variant_set_incomplete' END, \
           CASE WHEN publication.status = 'published' AND upload.status <> 'clean' \
             THEN 'published_upload_not_clean' END, \
           CASE WHEN upload.status IN ('blocked', 'quarantined') AND EXISTS( \
             SELECT 1 FROM media.asset_variants variant WHERE variant.asset_id = upload.id \
               AND variant.status = 'published') THEN 'hidden_asset_has_published_variant' END, \
           CASE WHEN EXISTS(SELECT 1 FROM media.variant_processing_jobs processing \
             WHERE processing.asset_id = upload.id AND processing.status = 'leased' \
               AND processing.lease_expires_at <= now()) THEN 'processing_lease_stale' END, \
           CASE WHEN publication.status = 'processing' AND NOT EXISTS(SELECT 1 \
             FROM media.variant_processing_jobs processing WHERE processing.asset_id = upload.id \
               AND processing.policy_version = publication.policy_version \
               AND processing.status IN ('queued', 'leased')) \
             THEN 'processing_without_active_job' END, \
           CASE WHEN publication.status = 'failed' AND NOT EXISTS(SELECT 1 \
             FROM media.variant_processing_jobs processing WHERE processing.asset_id = upload.id \
               AND processing.policy_version = publication.policy_version \
               AND processing.status = 'dead_letter') THEN 'failed_publication_job_mismatch' END, \
           CASE WHEN EXISTS(SELECT 1 FROM media.object_deletion_jobs job \
             WHERE job.upload_id = upload.id AND job.status = 'leased' \
               AND job.lease_expires_at <= now()) THEN 'deletion_lease_stale' END, \
           CASE WHEN EXISTS(SELECT 1 FROM media.object_deletion_jobs job \
             JOIN media.object_cleanup_steps step ON step.deletion_job_id = job.id \
             WHERE job.upload_id = upload.id AND step.status = 'leased' \
               AND step.lease_expires_at <= now()) THEN 'cleanup_lease_stale' END, \
           CASE WHEN upload.status = 'quarantined' AND NOT EXISTS( \
             SELECT 1 FROM media.object_deletion_jobs job WHERE job.upload_id = upload.id \
               AND job.status <> 'succeeded') THEN 'hidden_without_active_deletion_job' END, \
           CASE WHEN EXISTS(SELECT 1 FROM media.object_deletion_jobs job \
             WHERE job.upload_id = upload.id AND job.status <> 'succeeded' AND ( \
               NOT EXISTS(SELECT 1 FROM media.object_cleanup_steps step \
                 WHERE step.deletion_job_id = job.id AND step.step_kind = 'ingest_delete') \
               OR EXISTS(SELECT 1 FROM media.asset_variants variant WHERE variant.asset_id = upload.id \
                 AND variant.status <> 'deleted' AND ( \
                   NOT EXISTS(SELECT 1 FROM media.object_cleanup_steps step WHERE step.deletion_job_id = job.id \
                     AND step.object_key = variant.object_key AND step.step_kind = 'cdn_purge') \
                   OR NOT EXISTS(SELECT 1 FROM media.object_cleanup_steps step WHERE step.deletion_job_id = job.id \
                     AND step.object_key = variant.object_key AND step.step_kind = 'delivery_delete'))))) \
             THEN 'cleanup_plan_incomplete' END, \
           CASE WHEN EXISTS(SELECT 1 FROM media.object_deletion_jobs job WHERE job.upload_id = upload.id \
             AND job.status <> 'succeeded' AND NOT EXISTS(SELECT 1 FROM media.object_cleanup_steps step \
               WHERE step.deletion_job_id = job.id AND step.status <> 'succeeded')) \
             THEN 'deletion_completion_pending' END, \
           CASE WHEN EXISTS(SELECT 1 FROM media.object_deletion_jobs job \
             WHERE job.upload_id = upload.id AND job.status = 'dead_letter') \
             THEN 'deletion_dead_letter' END, \
           CASE WHEN EXISTS(SELECT 1 FROM media.variant_processing_jobs processing \
             WHERE processing.asset_id = upload.id AND processing.status = 'dead_letter') \
             THEN 'processing_dead_letter' END \
         ], NULL)::text[] AS issue_codes) issues \
         WHERE upload.id > COALESCE($1, 0) AND cardinality(issues.issue_codes) > 0 \
         ORDER BY upload.id LIMIT $2",
    )
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&mut *connection)
    .await?;
    let has_more = rows.len() as i64 > limit;
    let visible = rows.into_iter().take(limit as usize).collect::<Vec<_>>();
    let next_cursor = has_more.then(|| visible.last().map(|row| row.0.to_string())).flatten();
    let items = visible
        .into_iter()
        .map(|(asset_id, issue_codes)| ReconciliationFindingDto {
            asset_id: asset_id.to_string(),
            issue_codes,
        })
        .collect();
    let (ingest_candidate_count, delivery_candidate_count): (i64, i64) = sqlx::query_as(
        "SELECT count(*) FILTER (WHERE redacted_at IS NULL), \
                (SELECT count(*) FROM media.asset_variants WHERE status <> 'deleted') \
         FROM media.uploads",
    )
    .fetch_one(&mut *connection)
    .await?;
    Ok(ReconciliationReportDto {
        dry_run: true,
        items,
        next_cursor,
        provider_inventory: ProviderInventoryStatusDto {
            state: "manual_inventory_required".into(),
            ingest_candidate_count,
            delivery_candidate_count,
        },
    })
}
