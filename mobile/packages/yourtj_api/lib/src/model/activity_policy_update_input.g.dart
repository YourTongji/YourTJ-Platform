// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'activity_policy_update_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ActivityPolicyUpdateInput _$ActivityPolicyUpdateInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('ActivityPolicyUpdateInput', json, ($checkedConvert) {
  $checkKeys(
    json,
    requiredKeys: const ['expectedVersion', 'weights', 'reason'],
  );
  final val = ActivityPolicyUpdateInput(
    expectedVersion: $checkedConvert(
      'expectedVersion',
      (v) => (v as num).toInt(),
    ),
    weights: $checkedConvert(
      'weights',
      (v) => ActivityWeights.fromJson(v as Map<String, dynamic>),
    ),
    reason: $checkedConvert('reason', (v) => v as String),
  );
  return val;
});

Map<String, dynamic> _$ActivityPolicyUpdateInputToJson(
  ActivityPolicyUpdateInput instance,
) => <String, dynamic>{
  'expectedVersion': instance.expectedVersion,
  'weights': instance.weights.toJson(),
  'reason': instance.reason,
};
