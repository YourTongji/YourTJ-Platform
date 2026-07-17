import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/core/network/api_failure.dart';
import 'package:yourtj_mobile/features/schedule/data/schedule_export.dart';
import 'package:yourtj_mobile/features/schedule/domain/schedule_models.dart';

void main() {
  test(
    'exports only current-calendar schedule facts without scope secrets',
    () {
      final String encoded = encodeScheduleExport(
        environment: 'https://api.example/api/v2',
        calendarId: '2026-spring',
        courses: <ScheduledCourse>[_scheduled()],
        exportedAt: DateTime.fromMillisecondsSinceEpoch(
          1_800_000_000_000,
          isUtc: true,
        ),
      );

      final Map<String, dynamic> payload =
          jsonDecode(encoded) as Map<String, dynamic>;
      expect(payload['schema'], scheduleExportSchema);
      expect(payload['version'], scheduleExportVersion);
      expect(payload['scope'], <String, Object>{
        'environment': 'https://api.example/api/v2',
        'calendarId': '2026-spring',
      });
      expect(payload['exportedAt'], '2027-01-15T08:00:00.000Z');
      expect(payload['offerings'], hasLength(1));
      expect(encoded, isNot(contains('account-a')));
      expect(encoded, isNot(contains('accessToken')));
      expect(payload.keys, isNot(contains('principal')));
      final Map<String, dynamic> exportedOffering =
          (payload['offerings'] as List<dynamic>).single
              as Map<String, dynamic>;
      expect(exportedOffering.keys.toSet(), <String>{'course', 'timeslots'});
      expect(exportedOffering.keys, isNot(contains('colorIndex')));
      expect(
        (exportedOffering['course'] as Map<String, dynamic>)['id'],
        'offering-a',
      );
      expect(
        exportedOffering['course'],
        containsPair('reviewScope', 'teacher'),
      );
      expect(exportedOffering['course'], containsPair('reviewAvg', 4.5));
      expect(
        ((exportedOffering['timeslots'] as List<dynamic>).single
            as Map<String, dynamic>)['weekNumbers'],
        <int>[1, 2, 3, 4],
      );
    },
  );

  test('refuses to export a teaching class from another calendar', () {
    expect(
      () => encodeScheduleExport(
        environment: 'https://api.example/api/v2',
        calendarId: '2026-autumn',
        courses: <ScheduledCourse>[_scheduled()],
      ),
      throwsA(isA<ApiFailure>()),
    );
  });

  test('refuses an environment URL that could leak credentials', () {
    expect(
      () => encodeScheduleExport(
        environment: 'https://account:secret@api.example/api/v2',
        calendarId: '2026-spring',
        courses: <ScheduledCourse>[_scheduled()],
      ),
      throwsA(
        isA<ApiFailure>().having(
          (ApiFailure failure) => failure.kind,
          'kind',
          ApiFailureKind.invalidInput,
        ),
      ),
    );
  });

  test('refuses malformed time facts instead of sharing them', () {
    final ScheduledCourse source = _scheduled();
    final TimeSlot slot = source.timeslots.single;
    final ScheduledCourse malformed = ScheduledCourse(
      offering: source.offering,
      timeslots: <TimeSlot>[
        TimeSlot(
          offeringId: slot.offeringId,
          courseId: slot.offeringId,
          teacherName: slot.teacherName,
          weekday: slot.weekday,
          startSlot: slot.startSlot,
          endSlot: slot.endSlot,
          weeks: null,
          weekNumbers: const <int>{},
          weeksUnknown: false,
          location: slot.location,
          locationUnknown: slot.locationUnknown,
        ),
      ],
      colorIndex: source.colorIndex,
    );

    expect(
      () => encodeScheduleExport(
        environment: 'https://api.example/api/v2',
        calendarId: '2026-spring',
        courses: <ScheduledCourse>[malformed],
      ),
      throwsA(isA<ApiFailure>()),
    );
  });
}

ScheduledCourse _scheduled() {
  return ScheduledCourse(
    offering: SelectionOffering(
      id: 'offering-a',
      offeringId: 'offering-a',
      code: 'CS101',
      teachingClassCode: 'CS101-01',
      name: '程序设计',
      credit: 3,
      natureId: 'required',
      calendarId: '2026-spring',
      campusId: 'siping',
      facultyName: '电子与信息工程学院',
      teachingLanguage: '中文',
      teacherName: '张老师',
      teacherNames: const <String>['张老师'],
      startWeek: 1,
      endWeek: 16,
      weeksUnknown: false,
      scheduleUnknown: false,
      status: SelectionOfferingStatusEnum.unknown,
      catalogueCourseId: null,
      reviewCount: 12,
      reviewAvg: 4.5,
      reviewScope: SelectionOfferingReviewScopeEnum.teacher,
    ),
    timeslots: <TimeSlot>[
      TimeSlot(
        offeringId: 'offering-a',
        courseId: 'offering-a',
        teacherName: '张老师',
        weekday: 1,
        startSlot: 19,
        endSlot: 20,
        weeks: '1-16',
        weekNumbers: const <int>{1, 2, 3, 4},
        weeksUnknown: false,
        location: '教学楼',
        locationUnknown: false,
      ),
    ],
    colorIndex: 0,
  );
}
