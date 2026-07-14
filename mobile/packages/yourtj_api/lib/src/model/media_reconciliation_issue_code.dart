//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

/// Stable database-state anomaly category. Provider object inventory is reconciled separately.
enum MediaReconciliationIssueCode {
  /// Stable database-state anomaly category. Provider object inventory is reconciled separately.
  @JsonValue(r'publication_missing')
  publicationMissing(r'publication_missing'),

  /// Stable database-state anomaly category. Provider object inventory is reconciled separately.
  @JsonValue(r'published_variant_set_incomplete')
  publishedVariantSetIncomplete(r'published_variant_set_incomplete'),

  /// Stable database-state anomaly category. Provider object inventory is reconciled separately.
  @JsonValue(r'published_upload_not_clean')
  publishedUploadNotClean(r'published_upload_not_clean'),

  /// Stable database-state anomaly category. Provider object inventory is reconciled separately.
  @JsonValue(r'hidden_asset_has_published_variant')
  hiddenAssetHasPublishedVariant(r'hidden_asset_has_published_variant'),

  /// Stable database-state anomaly category. Provider object inventory is reconciled separately.
  @JsonValue(r'processing_lease_stale')
  processingLeaseStale(r'processing_lease_stale'),

  /// Stable database-state anomaly category. Provider object inventory is reconciled separately.
  @JsonValue(r'processing_without_active_job')
  processingWithoutActiveJob(r'processing_without_active_job'),

  /// Stable database-state anomaly category. Provider object inventory is reconciled separately.
  @JsonValue(r'failed_publication_job_mismatch')
  failedPublicationJobMismatch(r'failed_publication_job_mismatch'),

  /// Stable database-state anomaly category. Provider object inventory is reconciled separately.
  @JsonValue(r'deletion_lease_stale')
  deletionLeaseStale(r'deletion_lease_stale'),

  /// Stable database-state anomaly category. Provider object inventory is reconciled separately.
  @JsonValue(r'cleanup_lease_stale')
  cleanupLeaseStale(r'cleanup_lease_stale'),

  /// Stable database-state anomaly category. Provider object inventory is reconciled separately.
  @JsonValue(r'hidden_without_active_deletion_job')
  hiddenWithoutActiveDeletionJob(r'hidden_without_active_deletion_job'),

  /// Stable database-state anomaly category. Provider object inventory is reconciled separately.
  @JsonValue(r'cleanup_plan_incomplete')
  cleanupPlanIncomplete(r'cleanup_plan_incomplete'),

  /// Stable database-state anomaly category. Provider object inventory is reconciled separately.
  @JsonValue(r'deletion_completion_pending')
  deletionCompletionPending(r'deletion_completion_pending'),

  /// Stable database-state anomaly category. Provider object inventory is reconciled separately.
  @JsonValue(r'deletion_dead_letter')
  deletionDeadLetter(r'deletion_dead_letter'),

  /// Stable database-state anomaly category. Provider object inventory is reconciled separately.
  @JsonValue(r'processing_dead_letter')
  processingDeadLetter(r'processing_dead_letter'),

  /// Stable database-state anomaly category. Provider object inventory is reconciled separately.
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const MediaReconciliationIssueCode(this.value);

  final String value;

  @override
  String toString() => value;
}
