import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/features/schedule/domain/schedule_models.dart';

void main() {
  group('schedule conflict detection', () {
    test(
      'treats inclusive slot overlap in intersecting weeks as confirmed',
      () {
        final ScheduleConflict? conflict = findScheduleConflict(
          existing: <ScheduledCourse>[
            _scheduled(_slot(start: 3, end: 4, weeks: '1-16')),
          ],
          candidate: <TimeSlot>[_slot(start: 4, end: 5, weeks: '2-8')],
        );

        expect(conflict?.kind, ScheduleConflictKind.confirmed);
      },
    );

    test('treats missing week facts as a possible conflict', () {
      final ScheduleConflict? conflict = findScheduleConflict(
        existing: <ScheduledCourse>[
          _scheduled(_slot(start: 3, end: 4, weeks: null)),
        ],
        candidate: <TimeSlot>[_slot(start: 4, end: 5, weeks: '1-16')],
      );

      expect(conflict?.kind, ScheduleConflictKind.possible);
    });

    test('does not report a conflict for disjoint parsed weeks', () {
      final ScheduleConflict? conflict = findScheduleConflict(
        existing: <ScheduledCourse>[
          _scheduled(_slot(start: 3, end: 4, weeks: '1-15单')),
        ],
        candidate: <TimeSlot>[_slot(start: 4, end: 5, weeks: '2-16双')],
      );

      expect(conflict, isNull);
    });

    test('falls back to possible conflict for an unknown week grammar', () {
      final ScheduleConflict? conflict = findScheduleConflict(
        existing: <ScheduledCourse>[
          _scheduled(_slot(start: 3, end: 4, weeks: '前八周')),
        ],
        candidate: <TimeSlot>[_slot(start: 4, end: 5, weeks: '后八周')],
      );

      expect(conflict?.kind, ScheduleConflictKind.possible);
    });

    test('keeps identical unknown week grammar as a possible conflict', () {
      final ScheduleConflict? conflict = findScheduleConflict(
        existing: <ScheduledCourse>[
          _scheduled(_slot(start: 3, end: 4, weeks: '前八周')),
        ],
        candidate: <TimeSlot>[_slot(start: 4, end: 5, weeks: '前八周')],
      );

      expect(conflict?.kind, ScheduleConflictKind.possible);
    });
  });

  test('parses ranges, lists, and odd-even week suffixes', () {
    expect(parseCourseWeeks('1-5, 8'), <int>{1, 2, 3, 4, 5, 8});
    expect(parseCourseWeeks('1-6单'), <int>{1, 3, 5});
    expect(parseCourseWeeks('2-8双'), <int>{2, 4, 6, 8});
    expect(parseCourseWeeks('任意'), isNull);
  });

  test('marks unparseable non-empty week facts as unknown', () {
    final ScheduledCourse course = _scheduled(
      _slot(start: 3, end: 4, weeks: '前八周'),
    );

    expect(course.hasUnknownWeeks, isTrue);
  });
}

ScheduledCourse _scheduled(TimeSlot slot) {
  return ScheduledCourse(
    course: _course('EXISTING'),
    timeslots: <TimeSlot>[slot],
    colorIndex: 0,
  );
}

SelectionCourse _course(String code) {
  return SelectionCourse(
    id: code,
    code: code,
    name: '测试课程',
    credit: 2,
    natureId: null,
    calendarId: 'calendar-1',
    campusId: null,
    teacherName: null,
    teacherNames: const <String>[],
  );
}

TimeSlot _slot({required int start, required int end, required String? weeks}) {
  return TimeSlot(
    courseId: '1',
    teacherName: null,
    weekday: 1,
    startSlot: start,
    endSlot: end,
    weeks: weeks,
    location: null,
  );
}
