// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'course_search_hit.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

CourseSearchHit _$CourseSearchHitFromJson(Map<String, dynamic> json) =>
    $checkedCreate('CourseSearchHit', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'id',
          'code',
          'name',
          'credit',
          'department',
          'teacherName',
          'reviewCount',
          'reviewAvg',
        ],
      );
      final val = CourseSearchHit(
        id: $checkedConvert('id', (v) => v as String),
        code: $checkedConvert('code', (v) => v as String),
        name: $checkedConvert('name', (v) => v as String),
        credit: $checkedConvert('credit', (v) => v as num?),
        department: $checkedConvert('department', (v) => v as String?),
        teacherName: $checkedConvert('teacherName', (v) => v as String?),
        reviewCount: $checkedConvert('reviewCount', (v) => (v as num).toInt()),
        reviewAvg: $checkedConvert('reviewAvg', (v) => v as num?),
      );
      return val;
    });

Map<String, dynamic> _$CourseSearchHitToJson(CourseSearchHit instance) =>
    <String, dynamic>{
      'id': instance.id,
      'code': instance.code,
      'name': instance.name,
      'credit': instance.credit,
      'department': instance.department,
      'teacherName': instance.teacherName,
      'reviewCount': instance.reviewCount,
      'reviewAvg': instance.reviewAvg,
    };
