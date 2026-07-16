// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'campus.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

Campus _$CampusFromJson(Map<String, dynamic> json) =>
    $checkedCreate('Campus', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['id', 'name']);
      final val = Campus(
        id: $checkedConvert('id', (v) => v as String),
        name: $checkedConvert('name', (v) => v as String),
      );
      return val;
    });

Map<String, dynamic> _$CampusToJson(Campus instance) => <String, dynamic>{
  'id': instance.id,
  'name': instance.name,
};
