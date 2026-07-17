import 'dart:convert';

import 'package:yourtj_api/yourtj_api.dart';

import '../../../core/network/api_failure.dart';
import '../domain/schedule_models.dart';

const String scheduleExportSchema = 'yourtj.schedule';
const int scheduleExportVersion = 1;
const int scheduleExportMaxPayloadBytes = 2 * 1024 * 1024;

String encodeScheduleExport({
  required String environment,
  required String calendarId,
  required List<ScheduledCourse> courses,
  DateTime? exportedAt,
}) {
  final Uri? environmentUri = Uri.tryParse(environment);
  if (environmentUri == null ||
      !environmentUri.hasScheme ||
      environmentUri.host.isEmpty ||
      environmentUri.userInfo.isNotEmpty ||
      environmentUri.hasQuery ||
      environmentUri.hasFragment ||
      calendarId.isEmpty ||
      courses.length > 100 ||
      courses.any(
        (ScheduledCourse item) => !_isValidCourse(item, calendarId: calendarId),
      )) {
    throw const ApiFailure(
      kind: ApiFailureKind.invalidInput,
      message: '当前课表无法导出，请刷新后重试',
    );
  }
  final DateTime generatedAt = (exportedAt ?? DateTime.now()).toUtc();
  final Map<String, Object> payload = <String, Object>{
    'schema': scheduleExportSchema,
    'version': scheduleExportVersion,
    'scope': <String, String>{
      'environment': environmentUri.toString(),
      'calendarId': calendarId,
    },
    'exportedAt': generatedAt.toIso8601String(),
    'offerings': courses.map(_encodeItem).toList(growable: false),
  };
  final String encoded = '${jsonEncode(payload)}\n';
  if (utf8.encode(encoded).length > scheduleExportMaxPayloadBytes) {
    throw const ApiFailure(
      kind: ApiFailureKind.invalidInput,
      message: '课表导出内容过大，请减少教学班后重试',
    );
  }
  return encoded;
}

Map<String, Object> _encodeItem(ScheduledCourse item) {
  return <String, Object>{
    'course': _encodeOffering(item.offering),
    'timeslots': item.timeslots.map(_encodeTimeslot).toList(growable: false),
  };
}

Map<String, Object?> _encodeOffering(SelectionOffering offering) {
  return <String, Object?>{
    'id': offering.offeringId,
    'offeringId': offering.offeringId,
    'code': offering.code,
    'teachingClassCode': offering.teachingClassCode,
    'name': offering.name,
    'credit': offering.credit,
    'natureId': offering.natureId,
    'calendarId': offering.calendarId,
    'campusId': offering.campusId,
    'facultyName': offering.facultyName,
    'teachingLanguage': offering.teachingLanguage,
    'teacherName': offering.teacherName,
    'teacherNames': offering.teacherNames,
    'startWeek': offering.startWeek,
    'endWeek': offering.endWeek,
    'weeksUnknown': offering.weeksUnknown,
    'scheduleUnknown': offering.scheduleUnknown,
    'status': switch (offering.status) {
      SelectionOfferingStatusEnum.unknown => 'unknown',
      SelectionOfferingStatusEnum.active => 'active',
      SelectionOfferingStatusEnum.cancelled => 'cancelled',
      SelectionOfferingStatusEnum.unknownDefaultOpenApi => 'unknown',
    },
    'catalogueCourseId': offering.catalogueCourseId,
    'reviewCount': offering.reviewCount,
    'reviewAvg': offering.reviewAvg,
    'reviewScope': switch (offering.reviewScope) {
      SelectionOfferingReviewScopeEnum.none => 'none',
      SelectionOfferingReviewScopeEnum.course => 'course',
      SelectionOfferingReviewScopeEnum.teacher => 'teacher',
      SelectionOfferingReviewScopeEnum.unknownDefaultOpenApi => 'none',
    },
  };
}

Map<String, Object?> _encodeTimeslot(TimeSlot timeslot) {
  return <String, Object?>{
    'offeringId': timeslot.offeringId,
    'courseId': timeslot.offeringId,
    'teacherName': timeslot.teacherName,
    'weekday': timeslot.weekday,
    'startSlot': timeslot.startSlot,
    'endSlot': timeslot.endSlot,
    'weeks': timeslot.weeks,
    'weekNumbers': timeslot.weekNumbers.toList(growable: false)..sort(),
    'weeksUnknown': timeslot.weeksUnknown,
    'location': timeslot.location,
    'locationUnknown': timeslot.locationUnknown,
  };
}

bool _isValidCourse(ScheduledCourse course, {required String calendarId}) {
  final SelectionOffering offering = course.offering;
  final int? startWeek = offering.startWeek;
  final int? endWeek = offering.endWeek;
  final bool hasValidWeeks = offering.weeksUnknown
      ? startWeek == null && endWeek == null
      : startWeek != null &&
            endWeek != null &&
            startWeek >= 1 &&
            endWeek >= startWeek &&
            endWeek <= 30;
  final bool hasValidReview = offering.reviewCount == 0
      ? offering.reviewAvg == null &&
            offering.reviewScope == SelectionOfferingReviewScopeEnum.none
      : offering.reviewCount > 0 &&
            offering.reviewAvg != null &&
            offering.reviewAvg! >= 0 &&
            offering.reviewAvg! <= 5 &&
            offering.reviewScope != SelectionOfferingReviewScopeEnum.none &&
            offering.reviewScope !=
                SelectionOfferingReviewScopeEnum.unknownDefaultOpenApi;
  return offering.offeringId.isNotEmpty &&
      offering.code.isNotEmpty &&
      offering.name.isNotEmpty &&
      offering.calendarId == calendarId &&
      hasValidWeeks &&
      hasValidReview &&
      course.timeslots.length <= 100 &&
      course.timeslots.every(
        (TimeSlot timeslot) =>
            timeslot.offeringId == offering.offeringId &&
            timeslot.toJson()['courseId'] == offering.offeringId &&
            timeslot.weekday >= 1 &&
            timeslot.weekday <= 7 &&
            timeslot.startSlot >= 1 &&
            timeslot.startSlot <= 20 &&
            timeslot.endSlot >= timeslot.startSlot &&
            timeslot.endSlot <= 20 &&
            timeslot.weekNumbers.every((int week) => week >= 1 && week <= 30) &&
            (timeslot.weeksUnknown
                ? timeslot.weekNumbers.isEmpty
                : timeslot.weekNumbers.isNotEmpty) &&
            (timeslot.locationUnknown ||
                timeslot.location?.trim().isNotEmpty == true),
      );
}
