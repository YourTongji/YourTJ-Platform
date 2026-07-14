// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'course_detail.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

CourseDetail _$CourseDetailFromJson(Map<String, dynamic> json) =>
    $checkedCreate('CourseDetail', json, ($checkedConvert) {
      final val = CourseDetail(
        id: $checkedConvert('id', (v) => v as String?),
        code: $checkedConvert('code', (v) => v as String?),
        name: $checkedConvert('name', (v) => v as String?),
        credit: $checkedConvert('credit', (v) => v as num?),
        department: $checkedConvert('department', (v) => v as String?),
        teacherName: $checkedConvert('teacherName', (v) => v as String?),
        reviewCount: $checkedConvert(
          'reviewCount',
          (v) => (v as num?)?.toInt(),
        ),
        reviewAvg: $checkedConvert('reviewAvg', (v) => v as num?),
        teachers: $checkedConvert(
          'teachers',
          (v) => (v as List<dynamic>?)
              ?.map((e) => Teacher.fromJson(e as Map<String, dynamic>))
              .toList(),
        ),
        aliases: $checkedConvert(
          'aliases',
          (v) => (v as List<dynamic>?)?.map((e) => e as String).toList(),
        ),
      );
      return val;
    });

Map<String, dynamic> _$CourseDetailToJson(CourseDetail instance) =>
    <String, dynamic>{
      'id': ?instance.id,
      'code': ?instance.code,
      'name': ?instance.name,
      'credit': ?instance.credit,
      'department': ?instance.department,
      'teacherName': ?instance.teacherName,
      'reviewCount': ?instance.reviewCount,
      'reviewAvg': ?instance.reviewAvg,
      'teachers': ?instance.teachers?.map((e) => e.toJson()).toList(),
      'aliases': ?instance.aliases,
    };
