// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'trust_level_policy_update_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

TrustLevelPolicyUpdateInput _$TrustLevelPolicyUpdateInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('TrustLevelPolicyUpdateInput', json, ($checkedConvert) {
  $checkKeys(
    json,
    requiredKeys: const [
      'expectedVersion',
      'thresholdLevel2',
      'thresholdLevel3',
      'thresholdLevel4',
      'thresholdLevel5',
      'thresholdLevel6',
      'likeDailyCap',
      'demotionCooldownDays',
      'reason',
    ],
  );
  final val = TrustLevelPolicyUpdateInput(
    expectedVersion: $checkedConvert(
      'expectedVersion',
      (v) => (v as num).toInt(),
    ),
    thresholdLevel2: $checkedConvert(
      'thresholdLevel2',
      (v) => (v as num).toInt(),
    ),
    thresholdLevel3: $checkedConvert(
      'thresholdLevel3',
      (v) => (v as num).toInt(),
    ),
    thresholdLevel4: $checkedConvert(
      'thresholdLevel4',
      (v) => (v as num).toInt(),
    ),
    thresholdLevel5: $checkedConvert(
      'thresholdLevel5',
      (v) => (v as num).toInt(),
    ),
    thresholdLevel6: $checkedConvert(
      'thresholdLevel6',
      (v) => (v as num).toInt(),
    ),
    likeDailyCap: $checkedConvert('likeDailyCap', (v) => (v as num).toInt()),
    demotionCooldownDays: $checkedConvert(
      'demotionCooldownDays',
      (v) => (v as num).toInt(),
    ),
    reason: $checkedConvert('reason', (v) => v as String),
  );
  return val;
});

Map<String, dynamic> _$TrustLevelPolicyUpdateInputToJson(
  TrustLevelPolicyUpdateInput instance,
) => <String, dynamic>{
  'expectedVersion': instance.expectedVersion,
  'thresholdLevel2': instance.thresholdLevel2,
  'thresholdLevel3': instance.thresholdLevel3,
  'thresholdLevel4': instance.thresholdLevel4,
  'thresholdLevel5': instance.thresholdLevel5,
  'thresholdLevel6': instance.thresholdLevel6,
  'likeDailyCap': instance.likeDailyCap,
  'demotionCooldownDays': instance.demotionCooldownDays,
  'reason': instance.reason,
};
