// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'calendar.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

Calendar _$CalendarFromJson(Map<String, dynamic> json) =>
    $checkedCreate('Calendar', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['id', 'name', 'isCurrent']);
      final val = Calendar(
        id: $checkedConvert('id', (v) => v as String),
        name: $checkedConvert('name', (v) => v as String),
        isCurrent: $checkedConvert('isCurrent', (v) => v as bool),
      );
      return val;
    });

Map<String, dynamic> _$CalendarToJson(Calendar instance) => <String, dynamic>{
  'id': instance.id,
  'name': instance.name,
  'isCurrent': instance.isCurrent,
};
