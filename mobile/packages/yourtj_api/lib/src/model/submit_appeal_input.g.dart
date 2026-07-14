// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'submit_appeal_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

SubmitAppealInput _$SubmitAppealInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('SubmitAppealInput', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['governanceEventId', 'reason']);
      final val = SubmitAppealInput(
        governanceEventId: $checkedConvert(
          'governanceEventId',
          (v) => v as String,
        ),
        reason: $checkedConvert('reason', (v) => v as String),
      );
      return val;
    });

Map<String, dynamic> _$SubmitAppealInputToJson(SubmitAppealInput instance) =>
    <String, dynamic>{
      'governanceEventId': instance.governanceEventId,
      'reason': instance.reason,
    };
