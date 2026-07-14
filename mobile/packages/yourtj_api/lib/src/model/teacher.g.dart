// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'teacher.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

Teacher _$TeacherFromJson(Map<String, dynamic> json) =>
    $checkedCreate('Teacher', json, ($checkedConvert) {
      final val = Teacher(
        id: $checkedConvert('id', (v) => v as String?),
        name: $checkedConvert('name', (v) => v as String?),
        title: $checkedConvert('title', (v) => v as String?),
        department: $checkedConvert('department', (v) => v as String?),
      );
      return val;
    });

Map<String, dynamic> _$TeacherToJson(Teacher instance) => <String, dynamic>{
  'id': ?instance.id,
  'name': ?instance.name,
  'title': ?instance.title,
  'department': ?instance.department,
};
