// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'media_reconciliation_finding.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

MediaReconciliationFinding _$MediaReconciliationFindingFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('MediaReconciliationFinding', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['assetId', 'issueCodes']);
  final val = MediaReconciliationFinding(
    assetId: $checkedConvert('assetId', (v) => v as String),
    issueCodes: $checkedConvert(
      'issueCodes',
      (v) => (v as List<dynamic>)
          .map((e) => $enumDecode(_$MediaReconciliationIssueCodeEnumMap, e))
          .toSet(),
    ),
  );
  return val;
});

Map<String, dynamic> _$MediaReconciliationFindingToJson(
  MediaReconciliationFinding instance,
) => <String, dynamic>{
  'assetId': instance.assetId,
  'issueCodes': instance.issueCodes
      .map((e) => _$MediaReconciliationIssueCodeEnumMap[e]!)
      .toList(),
};

const _$MediaReconciliationIssueCodeEnumMap = {
  MediaReconciliationIssueCode.publicationMissing: 'publication_missing',
  MediaReconciliationIssueCode.publishedVariantSetIncomplete:
      'published_variant_set_incomplete',
  MediaReconciliationIssueCode.publishedUploadNotClean:
      'published_upload_not_clean',
  MediaReconciliationIssueCode.hiddenAssetHasPublishedVariant:
      'hidden_asset_has_published_variant',
  MediaReconciliationIssueCode.processingLeaseStale: 'processing_lease_stale',
  MediaReconciliationIssueCode.processingWithoutActiveJob:
      'processing_without_active_job',
  MediaReconciliationIssueCode.failedPublicationJobMismatch:
      'failed_publication_job_mismatch',
  MediaReconciliationIssueCode.deletionLeaseStale: 'deletion_lease_stale',
  MediaReconciliationIssueCode.cleanupLeaseStale: 'cleanup_lease_stale',
  MediaReconciliationIssueCode.hiddenWithoutActiveDeletionJob:
      'hidden_without_active_deletion_job',
  MediaReconciliationIssueCode.cleanupPlanIncomplete: 'cleanup_plan_incomplete',
  MediaReconciliationIssueCode.deletionCompletionPending:
      'deletion_completion_pending',
  MediaReconciliationIssueCode.deletionDeadLetter: 'deletion_dead_letter',
  MediaReconciliationIssueCode.processingDeadLetter: 'processing_dead_letter',
  MediaReconciliationIssueCode.unknownDefaultOpenApi:
      'unknown_default_open_api',
};
