//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'media_deletion_job.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class MediaDeletionJob {
  /// Returns a new [MediaDeletionJob] instance.
  MediaDeletionJob({
    required this.id,

    required this.uploadId,

    required this.accountId,

    required this.uploadStatus,

    required this.requestSource,

    required this.reason,

    required this.status,

    required this.attemptCount,

    required this.lastErrorCode,

    required this.availableAt,

    required this.createdAt,

    required this.updatedAt,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'uploadId', required: true, includeIfNull: false)
  final String uploadId;

  @JsonKey(name: r'accountId', required: true, includeIfNull: false)
  final String accountId;

  @JsonKey(
    name: r'uploadStatus',
    required: true,
    includeIfNull: false,
    unknownEnumValue: MediaDeletionJobUploadStatusEnum.unknownDefaultOpenApi,
  )
  final MediaDeletionJobUploadStatusEnum uploadStatus;

  @JsonKey(
    name: r'requestSource',
    required: true,
    includeIfNull: false,
    unknownEnumValue: MediaDeletionJobRequestSourceEnum.unknownDefaultOpenApi,
  )
  final MediaDeletionJobRequestSourceEnum requestSource;

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  @JsonKey(
    name: r'status',
    required: true,
    includeIfNull: false,
    unknownEnumValue: MediaDeletionJobStatusEnum.unknownDefaultOpenApi,
  )
  final MediaDeletionJobStatusEnum status;

  // minimum: 0
  @JsonKey(name: r'attemptCount', required: true, includeIfNull: false)
  final int attemptCount;

  @JsonKey(name: r'lastErrorCode', required: true, includeIfNull: true)
  final String? lastErrorCode;

  /// Unix seconds
  @JsonKey(name: r'availableAt', required: true, includeIfNull: false)
  final int availableAt;

  /// Unix seconds
  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  /// Unix seconds
  @JsonKey(name: r'updatedAt', required: true, includeIfNull: false)
  final int updatedAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is MediaDeletionJob &&
          other.id == id &&
          other.uploadId == uploadId &&
          other.accountId == accountId &&
          other.uploadStatus == uploadStatus &&
          other.requestSource == requestSource &&
          other.reason == reason &&
          other.status == status &&
          other.attemptCount == attemptCount &&
          other.lastErrorCode == lastErrorCode &&
          other.availableAt == availableAt &&
          other.createdAt == createdAt &&
          other.updatedAt == updatedAt;

  @override
  int get hashCode =>
      id.hashCode +
      uploadId.hashCode +
      accountId.hashCode +
      uploadStatus.hashCode +
      requestSource.hashCode +
      reason.hashCode +
      status.hashCode +
      attemptCount.hashCode +
      (lastErrorCode == null ? 0 : lastErrorCode.hashCode) +
      availableAt.hashCode +
      createdAt.hashCode +
      updatedAt.hashCode;

  factory MediaDeletionJob.fromJson(Map<String, dynamic> json) =>
      _$MediaDeletionJobFromJson(json);

  Map<String, dynamic> toJson() => _$MediaDeletionJobToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum MediaDeletionJobUploadStatusEnum {
  @JsonValue(r'pending')
  pending(r'pending'),
  @JsonValue(r'clean')
  clean(r'clean'),
  @JsonValue(r'quarantined')
  quarantined(r'quarantined'),
  @JsonValue(r'blocked')
  blocked(r'blocked'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const MediaDeletionJobUploadStatusEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum MediaDeletionJobRequestSourceEnum {
  @JsonValue(r'retention_gc')
  retentionGc(r'retention_gc'),
  @JsonValue(r'account_purge')
  accountPurge(r'account_purge'),
  @JsonValue(r'intent_cleanup')
  intentCleanup(r'intent_cleanup'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const MediaDeletionJobRequestSourceEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum MediaDeletionJobStatusEnum {
  @JsonValue(r'queued')
  queued(r'queued'),
  @JsonValue(r'leased')
  leased(r'leased'),
  @JsonValue(r'succeeded')
  succeeded(r'succeeded'),
  @JsonValue(r'dead_letter')
  deadLetter(r'dead_letter'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const MediaDeletionJobStatusEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
