// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'trust_level_event.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

TrustLevelEvent _$TrustLevelEventFromJson(Map<String, dynamic> json) =>
    $checkedCreate('TrustLevelEvent', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'id',
          'accountId',
          'eventKind',
          'fromLevel',
          'toLevel',
          'qualifyingScore',
          'policyVersion',
          'actorKind',
          'actorAccountId',
          'reason',
          'governanceEventId',
          'createdAt',
        ],
      );
      final val = TrustLevelEvent(
        id: $checkedConvert('id', (v) => v as String),
        accountId: $checkedConvert('accountId', (v) => v as String),
        eventKind: $checkedConvert(
          'eventKind',
          (v) => $enumDecode(
            _$TrustLevelEventEventKindEnumEnumMap,
            v,
            unknownValue: TrustLevelEventEventKindEnum.unknownDefaultOpenApi,
          ),
        ),
        fromLevel: $checkedConvert('fromLevel', (v) => (v as num).toInt()),
        toLevel: $checkedConvert('toLevel', (v) => (v as num).toInt()),
        qualifyingScore: $checkedConvert(
          'qualifyingScore',
          (v) => (v as num).toInt(),
        ),
        policyVersion: $checkedConvert(
          'policyVersion',
          (v) => (v as num).toInt(),
        ),
        actorKind: $checkedConvert(
          'actorKind',
          (v) => $enumDecode(
            _$TrustLevelEventActorKindEnumEnumMap,
            v,
            unknownValue: TrustLevelEventActorKindEnum.unknownDefaultOpenApi,
          ),
        ),
        actorAccountId: $checkedConvert('actorAccountId', (v) => v as String?),
        reason: $checkedConvert('reason', (v) => v as String?),
        governanceEventId: $checkedConvert(
          'governanceEventId',
          (v) => v as String?,
        ),
        createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
      );
      return val;
    });

Map<String, dynamic> _$TrustLevelEventToJson(TrustLevelEvent instance) =>
    <String, dynamic>{
      'id': instance.id,
      'accountId': instance.accountId,
      'eventKind': _$TrustLevelEventEventKindEnumEnumMap[instance.eventKind]!,
      'fromLevel': instance.fromLevel,
      'toLevel': instance.toLevel,
      'qualifyingScore': instance.qualifyingScore,
      'policyVersion': instance.policyVersion,
      'actorKind': _$TrustLevelEventActorKindEnumEnumMap[instance.actorKind]!,
      'actorAccountId': instance.actorAccountId,
      'reason': instance.reason,
      'governanceEventId': instance.governanceEventId,
      'createdAt': instance.createdAt,
    };

const _$TrustLevelEventEventKindEnumEnumMap = {
  TrustLevelEventEventKindEnum.upgrade: 'upgrade',
  TrustLevelEventEventKindEnum.demotion: 'demotion',
  TrustLevelEventEventKindEnum.manualSet: 'manual_set',
  TrustLevelEventEventKindEnum.overrideClear: 'override_clear',
  TrustLevelEventEventKindEnum.backfillInitialized: 'backfill_initialized',
  TrustLevelEventEventKindEnum.registration: 'registration',
  TrustLevelEventEventKindEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};

const _$TrustLevelEventActorKindEnumEnumMap = {
  TrustLevelEventActorKindEnum.system: 'system',
  TrustLevelEventActorKindEnum.account: 'account',
  TrustLevelEventActorKindEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};
