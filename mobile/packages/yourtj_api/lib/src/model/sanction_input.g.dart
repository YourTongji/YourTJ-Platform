// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'sanction_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

SanctionInput _$SanctionInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('SanctionInput', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['reason']);
      final val = SanctionInput(
        reason: $checkedConvert('reason', (v) => v as String),
        endsAt: $checkedConvert('endsAt', (v) => (v as num?)?.toInt()),
      );
      return val;
    });

Map<String, dynamic> _$SanctionInputToJson(SanctionInput instance) =>
    <String, dynamic>{'reason': instance.reason, 'endsAt': ?instance.endsAt};
