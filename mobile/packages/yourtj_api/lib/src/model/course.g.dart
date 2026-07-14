// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'course.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

Course _$CourseFromJson(Map<String, dynamic> json) => $checkedCreate(
  'Course',
  json,
  ($checkedConvert) {
    final val = Course(
      id: $checkedConvert('id', (v) => v as String?),
      code: $checkedConvert('code', (v) => v as String?),
      name: $checkedConvert('name', (v) => v as String?),
      credit: $checkedConvert('credit', (v) => v as num?),
      department: $checkedConvert('department', (v) => v as String?),
      teacherName: $checkedConvert('teacherName', (v) => v as String?),
      reviewCount: $checkedConvert('reviewCount', (v) => (v as num?)?.toInt()),
      reviewAvg: $checkedConvert('reviewAvg', (v) => v as num?),
    );
    return val;
  },
);

Map<String, dynamic> _$CourseToJson(Course instance) => <String, dynamic>{
  'id': ?instance.id,
  'code': ?instance.code,
  'name': ?instance.name,
  'credit': ?instance.credit,
  'department': ?instance.department,
  'teacherName': ?instance.teacherName,
  'reviewCount': ?instance.reviewCount,
  'reviewAvg': ?instance.reviewAvg,
};
