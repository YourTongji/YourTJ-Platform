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
          'offeringId',
          'code',
          'teachingClassCode',
          'name',
          'credit',
          'natureId',
          'calendarId',
          'campusId',
          'facultyName',
          'teachingLanguage',
          'teacherName',
          'teacherNames',
          'startWeek',
          'endWeek',
          'weeksUnknown',
          'scheduleUnknown',
          'status',
          'catalogueCourseId',
        ],
      );
      final val = SelectionCourse(
        id: $checkedConvert('id', (v) => v as String),
        offeringId: $checkedConvert('offeringId', (v) => v as String),
        code: $checkedConvert('code', (v) => v as String),
        teachingClassCode: $checkedConvert(
          'teachingClassCode',
          (v) => v as String?,
        ),
        name: $checkedConvert('name', (v) => v as String),
        credit: $checkedConvert('credit', (v) => v as num?),
        natureId: $checkedConvert('natureId', (v) => v as String?),
        calendarId: $checkedConvert('calendarId', (v) => v as String),
        campusId: $checkedConvert('campusId', (v) => v as String?),
        facultyName: $checkedConvert('facultyName', (v) => v as String?),
        teachingLanguage: $checkedConvert(
          'teachingLanguage',
          (v) => v as String?,
        ),
        teacherName: $checkedConvert('teacherName', (v) => v as String?),
        teacherNames: $checkedConvert(
          'teacherNames',
          (v) => (v as List<dynamic>).map((e) => e as String).toList(),
        ),
        startWeek: $checkedConvert('startWeek', (v) => (v as num?)?.toInt()),
        endWeek: $checkedConvert('endWeek', (v) => (v as num?)?.toInt()),
        weeksUnknown: $checkedConvert('weeksUnknown', (v) => v as bool),
        scheduleUnknown: $checkedConvert('scheduleUnknown', (v) => v as bool),
        status: $checkedConvert(
          'status',
          (v) => $enumDecode(
            _$SelectionCourseStatusEnumEnumMap,
            v,
            unknownValue: SelectionCourseStatusEnum.unknownDefaultOpenApi,
          ),
        ),
        catalogueCourseId: $checkedConvert(
          'catalogueCourseId',
          (v) => v as String?,
        ),
      );
      return val;
    });

Map<String, dynamic> _$SelectionCourseToJson(SelectionCourse instance) =>
    <String, dynamic>{
      'id': instance.id,
      'offeringId': instance.offeringId,
      'code': instance.code,
      'teachingClassCode': instance.teachingClassCode,
      'name': instance.name,
      'credit': instance.credit,
      'natureId': instance.natureId,
      'calendarId': instance.calendarId,
      'campusId': instance.campusId,
      'facultyName': instance.facultyName,
      'teachingLanguage': instance.teachingLanguage,
      'teacherName': instance.teacherName,
      'teacherNames': instance.teacherNames,
      'startWeek': instance.startWeek,
      'endWeek': instance.endWeek,
      'weeksUnknown': instance.weeksUnknown,
      'scheduleUnknown': instance.scheduleUnknown,
      'status': _$SelectionCourseStatusEnumEnumMap[instance.status]!,
      'catalogueCourseId': instance.catalogueCourseId,
    };

const _$SelectionCourseStatusEnumEnumMap = {
  SelectionCourseStatusEnum.unknown: 'unknown',
  SelectionCourseStatusEnum.active: 'active',
  SelectionCourseStatusEnum.cancelled: 'cancelled',
  SelectionCourseStatusEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
