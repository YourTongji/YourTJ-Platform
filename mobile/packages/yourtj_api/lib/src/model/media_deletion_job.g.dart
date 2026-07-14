// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'media_deletion_job.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

MediaDeletionJob _$MediaDeletionJobFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('MediaDeletionJob', json, ($checkedConvert) {
  $checkKeys(
    json,
    requiredKeys: const [
      'id',
      'uploadId',
      'accountId',
      'uploadStatus',
      'requestSource',
      'reason',
      'status',
      'attemptCount',
      'lastErrorCode',
      'availableAt',
      'createdAt',
      'updatedAt',
    ],
  );
  final val = MediaDeletionJob(
    id: $checkedConvert('id', (v) => v as String),
    uploadId: $checkedConvert('uploadId', (v) => v as String),
    accountId: $checkedConvert('accountId', (v) => v as String),
    uploadStatus: $checkedConvert(
      'uploadStatus',
      (v) => $enumDecode(
        _$MediaDeletionJobUploadStatusEnumEnumMap,
        v,
        unknownValue: MediaDeletionJobUploadStatusEnum.unknownDefaultOpenApi,
      ),
    ),
    requestSource: $checkedConvert(
      'requestSource',
      (v) => $enumDecode(
        _$MediaDeletionJobRequestSourceEnumEnumMap,
        v,
        unknownValue: MediaDeletionJobRequestSourceEnum.unknownDefaultOpenApi,
      ),
    ),
    reason: $checkedConvert('reason', (v) => v as String),
    status: $checkedConvert(
      'status',
      (v) => $enumDecode(
        _$MediaDeletionJobStatusEnumEnumMap,
        v,
        unknownValue: MediaDeletionJobStatusEnum.unknownDefaultOpenApi,
      ),
    ),
    attemptCount: $checkedConvert('attemptCount', (v) => (v as num).toInt()),
    lastErrorCode: $checkedConvert('lastErrorCode', (v) => v as String?),
    availableAt: $checkedConvert('availableAt', (v) => (v as num).toInt()),
    createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
    updatedAt: $checkedConvert('updatedAt', (v) => (v as num).toInt()),
  );
  return val;
});

Map<String, dynamic> _$MediaDeletionJobToJson(MediaDeletionJob instance) =>
    <String, dynamic>{
      'id': instance.id,
      'uploadId': instance.uploadId,
      'accountId': instance.accountId,
      'uploadStatus':
          _$MediaDeletionJobUploadStatusEnumEnumMap[instance.uploadStatus]!,
      'requestSource':
          _$MediaDeletionJobRequestSourceEnumEnumMap[instance.requestSource]!,
      'reason': instance.reason,
      'status': _$MediaDeletionJobStatusEnumEnumMap[instance.status]!,
      'attemptCount': instance.attemptCount,
      'lastErrorCode': instance.lastErrorCode,
      'availableAt': instance.availableAt,
      'createdAt': instance.createdAt,
      'updatedAt': instance.updatedAt,
    };

const _$MediaDeletionJobUploadStatusEnumEnumMap = {
  MediaDeletionJobUploadStatusEnum.pending: 'pending',
  MediaDeletionJobUploadStatusEnum.clean: 'clean',
  MediaDeletionJobUploadStatusEnum.quarantined: 'quarantined',
  MediaDeletionJobUploadStatusEnum.blocked: 'blocked',
  MediaDeletionJobUploadStatusEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};

const _$MediaDeletionJobRequestSourceEnumEnumMap = {
  MediaDeletionJobRequestSourceEnum.retentionGc: 'retention_gc',
  MediaDeletionJobRequestSourceEnum.accountPurge: 'account_purge',
  MediaDeletionJobRequestSourceEnum.intentCleanup: 'intent_cleanup',
  MediaDeletionJobRequestSourceEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};

const _$MediaDeletionJobStatusEnumEnumMap = {
  MediaDeletionJobStatusEnum.queued: 'queued',
  MediaDeletionJobStatusEnum.leased: 'leased',
  MediaDeletionJobStatusEnum.succeeded: 'succeeded',
  MediaDeletionJobStatusEnum.deadLetter: 'dead_letter',
  MediaDeletionJobStatusEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
