// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'trust_level_adjust_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

TrustLevelAdjustInput _$TrustLevelAdjustInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('TrustLevelAdjustInput', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['reason']);
  final val = TrustLevelAdjustInput(
    trustLevel: $checkedConvert('trustLevel', (v) => (v as num?)?.toInt()),
    clearOverride: $checkedConvert('clearOverride', (v) => v as bool? ?? false),
    reason: $checkedConvert('reason', (v) => v as String),
  );
  return val;
});

Map<String, dynamic> _$TrustLevelAdjustInputToJson(
  TrustLevelAdjustInput instance,
) => <String, dynamic>{
  'trustLevel': ?instance.trustLevel,
  'clearOverride': ?instance.clearOverride,
  'reason': instance.reason,
};
