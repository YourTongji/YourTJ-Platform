// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'course_nature.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

CourseNature _$CourseNatureFromJson(Map<String, dynamic> json) =>
    $checkedCreate('CourseNature', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['id', 'name']);
      final val = CourseNature(
        id: $checkedConvert('id', (v) => v as String),
        name: $checkedConvert('name', (v) => v as String),
      );
      return val;
    });

Map<String, dynamic> _$CourseNatureToJson(CourseNature instance) =>
    <String, dynamic>{'id': instance.id, 'name': instance.name};
