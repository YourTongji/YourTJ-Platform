// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'appeal_decision_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AppealDecisionInput _$AppealDecisionInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('AppealDecisionInput', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const ['expectedVersion', 'outcome', 'reason'],
      );
      final val = AppealDecisionInput(
        expectedVersion: $checkedConvert(
          'expectedVersion',
          (v) => (v as num).toInt(),
        ),
        outcome: $checkedConvert(
          'outcome',
          (v) => $enumDecode(
            _$AppealDecisionInputOutcomeEnumEnumMap,
            v,
            unknownValue: AppealDecisionInputOutcomeEnum.unknownDefaultOpenApi,
          ),
        ),
        reason: $checkedConvert('reason', (v) => v as String),
        amendedEndsAt: $checkedConvert(
          'amendedEndsAt',
          (v) => (v as num?)?.toInt(),
        ),
      );
      return val;
    });

Map<String, dynamic> _$AppealDecisionInputToJson(
  AppealDecisionInput instance,
) => <String, dynamic>{
  'expectedVersion': instance.expectedVersion,
  'outcome': _$AppealDecisionInputOutcomeEnumEnumMap[instance.outcome]!,
  'reason': instance.reason,
  'amendedEndsAt': ?instance.amendedEndsAt,
};

const _$AppealDecisionInputOutcomeEnumEnumMap = {
  AppealDecisionInputOutcomeEnum.upheld: 'upheld',
  AppealDecisionInputOutcomeEnum.overturned: 'overturned',
  AppealDecisionInputOutcomeEnum.amended: 'amended',
  AppealDecisionInputOutcomeEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};
