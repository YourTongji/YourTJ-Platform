// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'admin_course_update_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AdminCourseUpdateInput _$AdminCourseUpdateInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('AdminCourseUpdateInput', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['reason']);
  final val = AdminCourseUpdateInput(
    code: $checkedConvert('code', (v) => v as String?),
    name: $checkedConvert('name', (v) => v as String?),
    credit: $checkedConvert('credit', (v) => v as num?),
    department: $checkedConvert('department', (v) => v as String?),
    teacherName: $checkedConvert('teacherName', (v) => v as String?),
    reason: $checkedConvert('reason', (v) => v as String),
  );
  return val;
});

Map<String, dynamic> _$AdminCourseUpdateInputToJson(
  AdminCourseUpdateInput instance,
) => <String, dynamic>{
  'code': ?instance.code,
  'name': ?instance.name,
  'credit': ?instance.credit,
  'department': ?instance.department,
  'teacherName': ?instance.teacherName,
  'reason': instance.reason,
};
