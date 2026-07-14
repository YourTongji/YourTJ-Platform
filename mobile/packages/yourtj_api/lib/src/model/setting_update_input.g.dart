// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'setting_update_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

SettingUpdateInput _$SettingUpdateInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('SettingUpdateInput', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['value', 'reason']);
      final val = SettingUpdateInput(
        value: $checkedConvert('value', (v) => v as String),
        reason: $checkedConvert('reason', (v) => v as String),
      );
      return val;
    });

Map<String, dynamic> _$SettingUpdateInputToJson(SettingUpdateInput instance) =>
    <String, dynamic>{'value': instance.value, 'reason': instance.reason};
