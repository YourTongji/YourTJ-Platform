// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'major.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

Major _$MajorFromJson(Map<String, dynamic> json) => $checkedCreate(
  'Major',
  json,
  ($checkedConvert) {
    $checkKeys(json, requiredKeys: const ['id', 'name', 'facultyId', 'grade']);
    final val = Major(
      id: $checkedConvert('id', (v) => v as String),
      name: $checkedConvert('name', (v) => v as String),
      facultyId: $checkedConvert('facultyId', (v) => v as String?),
      grade: $checkedConvert('grade', (v) => v as String?),
    );
    return val;
  },
);

Map<String, dynamic> _$MajorToJson(Major instance) => <String, dynamic>{
  'id': instance.id,
  'name': instance.name,
  'facultyId': instance.facultyId,
  'grade': instance.grade,
};
