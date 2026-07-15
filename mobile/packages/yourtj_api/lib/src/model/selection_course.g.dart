// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'selection_course.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

SelectionCourse _$SelectionCourseFromJson(Map<String, dynamic> json) =>
    $checkedCreate('SelectionCourse', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'id',
          'code',
          'name',
          'credit',
          'natureId',
          'calendarId',
          'campusId',
          'teacherName',
          'teacherNames',
        ],
      );
      final val = SelectionCourse(
        id: $checkedConvert('id', (v) => v as String),
        code: $checkedConvert('code', (v) => v as String),
        name: $checkedConvert('name', (v) => v as String),
        credit: $checkedConvert('credit', (v) => v as num?),
        natureId: $checkedConvert('natureId', (v) => v as String?),
        calendarId: $checkedConvert('calendarId', (v) => v as String?),
        campusId: $checkedConvert('campusId', (v) => v as String?),
        teacherName: $checkedConvert('teacherName', (v) => v as String?),
        teacherNames: $checkedConvert(
          'teacherNames',
          (v) => (v as List<dynamic>).map((e) => e as String).toList(),
        ),
      );
      return val;
    });

Map<String, dynamic> _$SelectionCourseToJson(SelectionCourse instance) =>
    <String, dynamic>{
      'id': instance.id,
      'code': instance.code,
      'name': instance.name,
      'credit': instance.credit,
      'natureId': instance.natureId,
      'calendarId': instance.calendarId,
      'campusId': instance.campusId,
      'teacherName': instance.teacherName,
      'teacherNames': instance.teacherNames,
    };
