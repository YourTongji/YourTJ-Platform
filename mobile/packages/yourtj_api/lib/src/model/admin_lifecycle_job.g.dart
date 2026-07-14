// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'admin_lifecycle_job.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AdminLifecycleJob _$AdminLifecycleJobFromJson(Map<String, dynamic> json) =>
    $checkedCreate('AdminLifecycleJob', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'id',
          'accountId',
          'accountHandle',
          'accountState',
          'jobType',
          'status',
          'attempts',
          'nextAttemptAt',
          'createdAt',
          'updatedAt',
        ],
      );
      final val = AdminLifecycleJob(
        id: $checkedConvert('id', (v) => v as String),
        accountId: $checkedConvert('accountId', (v) => v as String),
        accountHandle: $checkedConvert('accountHandle', (v) => v as String),
        accountState: $checkedConvert(
          'accountState',
          (v) => $enumDecode(
            _$AccountLifecycleStateEnumMap,
            v,
            unknownValue: AccountLifecycleState.unknownDefaultOpenApi,
          ),
        ),
        jobType: $checkedConvert(
          'jobType',
          (v) => $enumDecode(
            _$AdminLifecycleJobJobTypeEnumEnumMap,
            v,
            unknownValue: AdminLifecycleJobJobTypeEnum.unknownDefaultOpenApi,
          ),
        ),
        status: $checkedConvert(
          'status',
          (v) => $enumDecode(
            _$AdminLifecycleJobStatusEnumEnumMap,
            v,
            unknownValue: AdminLifecycleJobStatusEnum.unknownDefaultOpenApi,
          ),
        ),
        attempts: $checkedConvert('attempts', (v) => (v as num).toInt()),
        nextAttemptAt: $checkedConvert(
          'nextAttemptAt',
          (v) => (v as num).toInt(),
        ),
        lockedAt: $checkedConvert('lockedAt', (v) => (v as num?)?.toInt()),
        lastErrorCode: $checkedConvert('lastErrorCode', (v) => v as String?),
        purgeStartedAt: $checkedConvert(
          'purgeStartedAt',
          (v) => (v as num?)?.toInt(),
        ),
        createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
        updatedAt: $checkedConvert('updatedAt', (v) => (v as num).toInt()),
      );
      return val;
    });

Map<String, dynamic> _$AdminLifecycleJobToJson(AdminLifecycleJob instance) =>
    <String, dynamic>{
      'id': instance.id,
      'accountId': instance.accountId,
      'accountHandle': instance.accountHandle,
      'accountState': _$AccountLifecycleStateEnumMap[instance.accountState]!,
      'jobType': _$AdminLifecycleJobJobTypeEnumEnumMap[instance.jobType]!,
      'status': _$AdminLifecycleJobStatusEnumEnumMap[instance.status]!,
      'attempts': instance.attempts,
      'nextAttemptAt': instance.nextAttemptAt,
      'lockedAt': ?instance.lockedAt,
      'lastErrorCode': ?instance.lastErrorCode,
      'purgeStartedAt': ?instance.purgeStartedAt,
      'createdAt': instance.createdAt,
      'updatedAt': instance.updatedAt,
    };

const _$AccountLifecycleStateEnumMap = {
  AccountLifecycleState.active: 'active',
  AccountLifecycleState.deactivated: 'deactivated',
  AccountLifecycleState.deletionRequested: 'deletion_requested',
  AccountLifecycleState.deleted: 'deleted',
  AccountLifecycleState.purged: 'purged',
  AccountLifecycleState.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$AdminLifecycleJobJobTypeEnumEnumMap = {
  AdminLifecycleJobJobTypeEnum.markDeleted: 'mark_deleted',
  AdminLifecycleJobJobTypeEnum.purge: 'purge',
  AdminLifecycleJobJobTypeEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};

const _$AdminLifecycleJobStatusEnumEnumMap = {
  AdminLifecycleJobStatusEnum.queued: 'queued',
  AdminLifecycleJobStatusEnum.running: 'running',
  AdminLifecycleJobStatusEnum.succeeded: 'succeeded',
  AdminLifecycleJobStatusEnum.failed: 'failed',
  AdminLifecycleJobStatusEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
