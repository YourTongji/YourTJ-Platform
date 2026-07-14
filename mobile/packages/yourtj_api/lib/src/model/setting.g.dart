// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'setting.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

Setting _$SettingFromJson(Map<String, dynamic> json) =>
    $checkedCreate('Setting', json, ($checkedConvert) {
      final val = Setting(
        key: $checkedConvert('key', (v) => v as String?),
        value: $checkedConvert('value', (v) => v as String?),
      );
      return val;
    });

Map<String, dynamic> _$SettingToJson(Setting instance) => <String, dynamic>{
  'key': ?instance.key,
  'value': ?instance.value,
};
