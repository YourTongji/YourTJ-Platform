import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/features/schedule/domain/schedule_models.dart';

void main() {
  group('schedule conflict detection', () {
    test('treats overlapping slots in intersecting weeks as confirmed', () {
      final ScheduleConflict? conflict = findScheduleConflict(
        existing: <ScheduledCourse>[
          _scheduled(_slot(start: 3, end: 4, weekNumbers: <int>{1, 2, 3, 4})),
        ],
        candidate: <TimeSlot>[
          _slot(start: 4, end: 5, weekNumbers: <int>{4, 5, 6}),
        ],
      );

      expect(conflict?.kind, ScheduleConflictKind.confirmed);
    });

    test('treats missing week facts as a possible conflict', () {
      final ScheduleConflict? conflict = findScheduleConflict(
        existing: <ScheduledCourse>[
          _scheduled(
            _slot(
              start: 3,
              end: 4,
              weekNumbers: const <int>{},
              weeksUnknown: true,
            ),
          ),
        ],
        candidate: <TimeSlot>[
          _slot(start: 4, end: 5, weekNumbers: <int>{1, 2, 3}),
        ],
      );

      expect(conflict?.kind, ScheduleConflictKind.possible);
    });

    test('does not report a conflict for disjoint weeks', () {
      final ScheduleConflict? conflict = findScheduleConflict(
        existing: <ScheduledCourse>[
          _scheduled(_slot(start: 3, end: 4, weekNumbers: <int>{1, 3, 5})),
        ],
        candidate: <TimeSlot>[
          _slot(start: 4, end: 5, weekNumbers: <int>{2, 4, 6}),
        ],
      );

      expect(conflict, isNull);
    });

    test('treats a completely unknown candidate as possible when occupied', () {
      final ScheduleConflict? conflict = findScheduleConflict(
        existing: <ScheduledCourse>[
          _scheduled(_slot(start: 3, end: 4, weekNumbers: <int>{1, 2})),
        ],
        candidate: const <TimeSlot>[],
        candidateScheduleUnknown: true,
      );

      expect(conflict?.kind, ScheduleConflictKind.possible);
      expect(conflict?.candidateSlot, isNull);
    });

    test(
      'treats an empty candidate response as unknown even without a flag',
      () {
        final ScheduleConflict? conflict = findScheduleConflict(
          existing: <ScheduledCourse>[
            _scheduled(_slot(start: 3, end: 4, weekNumbers: <int>{1, 2})),
          ],
          candidate: const <TimeSlot>[],
        );

        expect(conflict?.kind, ScheduleConflictKind.possible);
        expect(conflict?.candidateSlot, isNull);
      },
    );

    test('treats an existing unknown schedule as possible', () {
      final ScheduleConflict? conflict = findScheduleConflict(
        existing: <ScheduledCourse>[
          ScheduledCourse(
            offering: _offering('unknown', scheduleUnknown: true),
            timeslots: const <TimeSlot>[],
            colorIndex: 0,
          ),
        ],
        candidate: <TimeSlot>[
          _slot(start: 3, end: 4, weekNumbers: <int>{1, 2}),
        ],
      );

      expect(conflict?.kind, ScheduleConflictKind.possible);
    });

    test('treats an existing empty response as an unknown schedule', () {
      final ScheduleConflict? conflict = findScheduleConflict(
        existing: <ScheduledCourse>[
          ScheduledCourse(
            offering: _offering('empty'),
            timeslots: const <TimeSlot>[],
            colorIndex: 0,
          ),
        ],
        candidate: <TimeSlot>[
          _slot(start: 3, end: 4, weekNumbers: <int>{1, 2}),
        ],
      );

      expect(conflict?.kind, ScheduleConflictKind.possible);
    });

    test('keeps partially materialized unknown schedules conservative', () {
      final ScheduleConflict? conflict = findScheduleConflict(
        existing: <ScheduledCourse>[
          _scheduled(_slot(start: 1, end: 2, weekNumbers: <int>{1, 2})),
        ],
        candidate: <TimeSlot>[
          _slot(start: 10, end: 11, weekNumbers: <int>{1, 2}),
        ],
        candidateScheduleUnknown: true,
      );

      expect(conflict?.kind, ScheduleConflictKind.possible);
    });
  });

  test('parses bounded ranges, lists, and odd-even suffixes', () {
    expect(parseCourseWeeks('1-5, 8'), <int>{1, 2, 3, 4, 5, 8});
    expect(parseCourseWeeks('1-6单'), <int>{1, 3, 5});
    expect(parseCourseWeeks('2-8双'), <int>{2, 4, 6, 8});
    expect(parseCourseWeeks('31-32'), isNull);
    expect(parseCourseWeeks('任意'), isNull);
  });
}

ScheduledCourse _scheduled(TimeSlot slot) {
  return ScheduledCourse(
    offering: _offering('existing'),
    timeslots: <TimeSlot>[slot],
    colorIndex: 0,
  );
}

SelectionOffering _offering(String offeringId, {bool scheduleUnknown = false}) {
  return SelectionOffering(
    id: offeringId,
    offeringId: offeringId,
    code: 'CS101',
    teachingClassCode: offeringId,
    name: '测试课程',
    credit: 2,
    natureId: null,
    calendarId: '2026-spring',
    campusId: null,
    facultyName: null,
    teachingLanguage: null,
    teacherName: null,
    teacherNames: const <String>[],
    startWeek: scheduleUnknown ? null : 1,
    endWeek: scheduleUnknown ? null : 16,
    weeksUnknown: scheduleUnknown,
    scheduleUnknown: scheduleUnknown,
    status: SelectionOfferingStatusEnum.unknown,
    catalogueCourseId: null,
    reviewCount: 0,
    reviewAvg: null,
    reviewScope: SelectionOfferingReviewScopeEnum.none,
  );
}

TimeSlot _slot({
  required int start,
  required int end,
  required Set<int> weekNumbers,
  bool weeksUnknown = false,
}) {
  return TimeSlot(
    offeringId: 'existing',
    courseId: 'existing',
    teacherName: null,
    weekday: 1,
    startSlot: start,
    endSlot: end,
    weeks: null,
    weekNumbers: weekNumbers,
    weeksUnknown: weeksUnknown,
    location: null,
    locationUnknown: true,
  );
}
