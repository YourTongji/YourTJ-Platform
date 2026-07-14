// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'unsanction_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

UnsanctionInput _$UnsanctionInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('UnsanctionInput', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['sanctionId', 'reason']);
      final val = UnsanctionInput(
        sanctionId: $checkedConvert('sanctionId', (v) => v as String),
        reason: $checkedConvert('reason', (v) => v as String),
      );
      return val;
    });

Map<String, dynamic> _$UnsanctionInputToJson(UnsanctionInput instance) =>
    <String, dynamic>{
      'sanctionId': instance.sanctionId,
      'reason': instance.reason,
    };
