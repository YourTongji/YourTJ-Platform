// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'admin_course_create_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AdminCourseCreateInput _$AdminCourseCreateInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('AdminCourseCreateInput', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['code', 'name', 'reason']);
  final val = AdminCourseCreateInput(
    code: $checkedConvert('code', (v) => v as String),
    name: $checkedConvert('name', (v) => v as String),
    credit: $checkedConvert('credit', (v) => v as num?),
    department: $checkedConvert('department', (v) => v as String?),
    teacherName: $checkedConvert('teacherName', (v) => v as String?),
    reason: $checkedConvert('reason', (v) => v as String),
  );
  return val;
});

Map<String, dynamic> _$AdminCourseCreateInputToJson(
  AdminCourseCreateInput instance,
) => <String, dynamic>{
  'code': instance.code,
  'name': instance.name,
  'credit': ?instance.credit,
  'department': ?instance.department,
  'teacherName': ?instance.teacherName,
  'reason': instance.reason,
};
