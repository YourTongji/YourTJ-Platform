// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'selection_sync_job.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

SelectionSyncJob _$SelectionSyncJobFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('SelectionSyncJob', json, ($checkedConvert) {
  $checkKeys(
    json,
    requiredKeys: const [
      'id',
      'requestedBy',
      'status',
      'step',
      'attempts',
      'progressCurrent',
      'progressTotal',
      'nextAttemptAt',
      'lastErrorCode',
      'result',
      'createdAt',
      'updatedAt',
    ],
  );
  final val = SelectionSyncJob(
    id: $checkedConvert('id', (v) => v as String),
    requestedBy: $checkedConvert('requestedBy', (v) => v as String),
    status: $checkedConvert(
      'status',
      (v) => $enumDecode(
        _$SelectionSyncJobStatusEnumEnumMap,
        v,
        unknownValue: SelectionSyncJobStatusEnum.unknownDefaultOpenApi,
      ),
    ),
    step: $checkedConvert(
      'step',
      (v) => $enumDecode(
        _$SelectionSyncJobStepEnumEnumMap,
        v,
        unknownValue: SelectionSyncJobStepEnum.unknownDefaultOpenApi,
      ),
    ),
    attempts: $checkedConvert('attempts', (v) => (v as num).toInt()),
    progressCurrent: $checkedConvert(
      'progressCurrent',
      (v) => (v as num).toInt(),
    ),
    progressTotal: $checkedConvert(
      'progressTotal',
      (v) => $enumDecode(
        _$SelectionSyncJobProgressTotalEnumEnumMap,
        v,
        unknownValue: SelectionSyncJobProgressTotalEnum.unknownDefaultOpenApi,
      ),
    ),
    nextAttemptAt: $checkedConvert('nextAttemptAt', (v) => (v as num).toInt()),
    lastErrorCode: $checkedConvert('lastErrorCode', (v) => v as String?),
    result: $checkedConvert(
      'result',
      (v) =>
          (v as Map<String, dynamic>).map((k, e) => MapEntry(k, e as Object)),
    ),
    startedAt: $checkedConvert('startedAt', (v) => (v as num?)?.toInt()),
    completedAt: $checkedConvert('completedAt', (v) => (v as num?)?.toInt()),
    createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
    updatedAt: $checkedConvert('updatedAt', (v) => (v as num).toInt()),
  );
  return val;
});

Map<String, dynamic> _$SelectionSyncJobToJson(SelectionSyncJob instance) =>
    <String, dynamic>{
      'id': instance.id,
      'requestedBy': instance.requestedBy,
      'status': _$SelectionSyncJobStatusEnumEnumMap[instance.status]!,
      'step': _$SelectionSyncJobStepEnumEnumMap[instance.step]!,
      'attempts': instance.attempts,
      'progressCurrent': instance.progressCurrent,
      'progressTotal':
          _$SelectionSyncJobProgressTotalEnumEnumMap[instance.progressTotal]!,
      'nextAttemptAt': instance.nextAttemptAt,
      'lastErrorCode': instance.lastErrorCode,
      'result': instance.result,
      'startedAt': ?instance.startedAt,
      'completedAt': ?instance.completedAt,
      'createdAt': instance.createdAt,
      'updatedAt': instance.updatedAt,
    };

const _$SelectionSyncJobStatusEnumEnumMap = {
  SelectionSyncJobStatusEnum.queued: 'queued',
  SelectionSyncJobStatusEnum.running: 'running',
  SelectionSyncJobStatusEnum.succeeded: 'succeeded',
  SelectionSyncJobStatusEnum.dead: 'dead',
  SelectionSyncJobStatusEnum.cancelled: 'cancelled',
  SelectionSyncJobStatusEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$SelectionSyncJobStepEnumEnumMap = {
  SelectionSyncJobStepEnum.queued: 'queued',
  SelectionSyncJobStepEnum.materialize: 'materialize',
  SelectionSyncJobStepEnum.catalogue: 'catalogue',
  SelectionSyncJobStepEnum.search: 'search',
  SelectionSyncJobStepEnum.cache: 'cache',
  SelectionSyncJobStepEnum.complete: 'complete',
  SelectionSyncJobStepEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$SelectionSyncJobProgressTotalEnumEnumMap = {
  SelectionSyncJobProgressTotalEnum.number4: 4,
  SelectionSyncJobProgressTotalEnum.unknownDefaultOpenApi: 11184809,
};
