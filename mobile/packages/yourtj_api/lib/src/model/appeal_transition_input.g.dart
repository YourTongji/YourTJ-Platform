// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'appeal_transition_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AppealTransitionInput _$AppealTransitionInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('AppealTransitionInput', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['expectedVersion', 'reason']);
  final val = AppealTransitionInput(
    expectedVersion: $checkedConvert(
      'expectedVersion',
      (v) => (v as num).toInt(),
    ),
    reason: $checkedConvert('reason', (v) => v as String),
  );
  return val;
});

Map<String, dynamic> _$AppealTransitionInputToJson(
  AppealTransitionInput instance,
) => <String, dynamic>{
  'expectedVersion': instance.expectedVersion,
  'reason': instance.reason,
};
