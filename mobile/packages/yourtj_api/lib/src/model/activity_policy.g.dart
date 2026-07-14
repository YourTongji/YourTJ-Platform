// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'activity_policy.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ActivityPolicy _$ActivityPolicyFromJson(Map<String, dynamic> json) =>
    $checkedCreate('ActivityPolicy', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'version',
          'timezone',
          'weights',
          'reason',
          'changedBy',
          'createdAt',
        ],
      );
      final val = ActivityPolicy(
        version: $checkedConvert('version', (v) => (v as num).toInt()),
        timezone: $checkedConvert(
          'timezone',
          (v) => $enumDecode(
            _$ActivityPolicyTimezoneEnumEnumMap,
            v,
            unknownValue: ActivityPolicyTimezoneEnum.unknownDefaultOpenApi,
          ),
        ),
        weights: $checkedConvert(
          'weights',
          (v) => ActivityWeights.fromJson(v as Map<String, dynamic>),
        ),
        reason: $checkedConvert('reason', (v) => v as String),
        changedBy: $checkedConvert('changedBy', (v) => v as String),
        createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
      );
      return val;
    });

Map<String, dynamic> _$ActivityPolicyToJson(ActivityPolicy instance) =>
    <String, dynamic>{
      'version': instance.version,
      'timezone': _$ActivityPolicyTimezoneEnumEnumMap[instance.timezone]!,
      'weights': instance.weights.toJson(),
      'reason': instance.reason,
      'changedBy': instance.changedBy,
      'createdAt': instance.createdAt,
    };

const _$ActivityPolicyTimezoneEnumEnumMap = {
  ActivityPolicyTimezoneEnum.asiaSlashShanghai: 'Asia/Shanghai',
  ActivityPolicyTimezoneEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
