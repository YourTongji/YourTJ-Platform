import 'dart:async';

import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/features/schedule/data/schedule_local_repository.dart';
import 'package:yourtj_mobile/features/schedule/data/selection_repository.dart';
import 'package:yourtj_mobile/features/schedule/domain/schedule_controller.dart';
import 'package:yourtj_mobile/features/schedule/domain/schedule_models.dart';

void main() {
  group('schedule conflict detection', () {
    test(
      'treats inclusive slot overlap in intersecting weeks as confirmed',
      () {
        final ScheduleConflict? conflict = findScheduleConflict(
          existing: <ScheduledCourse>[
            _scheduled(_slot(start: 3, end: 4, weekNumbers: <int>{1, 2, 3, 4})),
          ],
          candidate: <TimeSlot>[
            _slot(start: 4, end: 5, weekNumbers: <int>{4, 5, 6}),
          ],
        );

        expect(conflict?.kind, ScheduleConflictKind.confirmed);
      },
    );

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

    test('does not report a conflict for disjoint parsed weeks', () {
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

    test('falls back to possible conflict for an unknown week grammar', () {
      final ScheduleConflict? conflict = findScheduleConflict(
        existing: <ScheduledCourse>[
          _scheduled(
            _slot(
              start: 3,
              end: 4,
              weekNumbers: const <int>{},
              weeksUnknown: true,
              weeks: '前八周',
            ),
          ),
        ],
        candidate: <TimeSlot>[
          _slot(
            start: 4,
            end: 5,
            weekNumbers: const <int>{},
            weeksUnknown: true,
            weeks: '后八周',
          ),
        ],
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

  test(
    'paginates and keeps parallel offerings with the same course code',
    () async {
      final _FakeSelectionRepository selection = _FakeSelectionRepository(
        pages: <String?, SelectionOfferingPage>{
          null: SelectionOfferingPage(
            items: <SelectionOffering>[_offering('offering-a')],
            nextCursor: 'next',
            hasMore: true,
          ),
          'next': SelectionOfferingPage(
            items: <SelectionOffering>[
              _offering('offering-a'),
              _offering('offering-b'),
            ],
            nextCursor: null,
            hasMore: false,
          ),
        },
      );
      final ScheduleController controller = _controller(selection);
      addTearDown(controller.dispose);

      await controller.initialize();
      await controller.selectGrade('2026');
      await controller.selectMajor('software');
      await controller.loadMore();

      expect(
        controller.offerings.map((SelectionOffering item) => item.offeringId),
        <String>['offering-a', 'offering-b'],
      );
      await controller.addOffering(controller.offerings.first);
      await controller.addOffering(controller.offerings.last);
      expect(controller.scheduled, hasLength(2));
    },
  );

  test(
    'keeps a conflicting teaching class pending after browse and add',
    () async {
      final _FakeSelectionRepository selection = _FakeSelectionRepository(
        pages: <String?, SelectionOfferingPage>{
          null: SelectionOfferingPage(
            items: <SelectionOffering>[
              _offering('offering-a'),
              _offering('parallel-a'),
            ],
            nextCursor: null,
            hasMore: false,
          ),
        },
      );
      final ScheduleController controller = _controller(selection);
      addTearDown(controller.dispose);

      await controller.initialize();
      await controller.selectGrade('2026');
      await controller.selectMajor('software');
      final ScheduleAddResult first = await controller.addOffering(
        controller.offerings.first,
      );
      final ScheduleAddResult second = await controller.addOffering(
        controller.offerings.last,
      );

      expect(first.status, ScheduleAddStatus.added);
      expect(second.status, ScheduleAddStatus.conflict);
      expect(second.conflict?.kind, ScheduleConflictKind.confirmed);
      expect(second.pendingOffering?.offeringId, 'parallel-a');
      expect(controller.scheduled.single.offering.offeringId, 'offering-a');
    },
  );

  test(
    'ignores an obsolete offering response after the query changes',
    () async {
      final Completer<SelectionOfferingPage> delayed =
          Completer<SelectionOfferingPage>();
      final _FakeSelectionRepository selection = _FakeSelectionRepository(
        pages: <String?, SelectionOfferingPage>{},
        delayedSearch: delayed,
      );
      final ScheduleController controller = _controller(selection);
      addTearDown(controller.dispose);

      await controller.initialize();
      controller.setMode(SelectionBrowseMode.search);
      final Future<void> obsolete = controller.submitSearch('first');
      final Future<void> current = controller.submitSearch('second');
      await current;
      delayed.complete(
        SelectionOfferingPage(
          items: <SelectionOffering>[_offering('obsolete')],
          nextCursor: null,
          hasMore: false,
        ),
      );
      await obsolete;

      expect(controller.offerings.single.offeringId, 'current');
      expect(selection.firstSearchRequest?.isCancelled, isTrue);
    },
  );
}

ScheduledCourse _scheduled(TimeSlot slot) {
  return ScheduledCourse(
    offering: _offering('EXISTING'),
    timeslots: <TimeSlot>[slot],
    colorIndex: 0,
  );
}

SelectionOffering _offering(String offeringId) {
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
    startWeek: null,
    endWeek: null,
    weeksUnknown: false,
    scheduleUnknown: false,
    status: SelectionOfferingStatusEnum.unknown,
    catalogueCourseId: null,
  );
}

TimeSlot _slot({
  required int start,
  required int end,
  required Set<int> weekNumbers,
  bool weeksUnknown = false,
  String? weeks,
}) {
  return TimeSlot(
    offeringId: '1',
    courseId: '1',
    teacherName: null,
    weekday: 1,
    startSlot: start,
    endSlot: end,
    weeks: weeks,
    weekNumbers: weekNumbers,
    weeksUnknown: weeksUnknown,
    location: null,
    locationUnknown: true,
  );
}

ScheduleController _controller(SelectionRepository selection) {
  return ScheduleController(
    scope: const ScheduleNamespace(
      environment: 'https://api.example/api/v2',
      principal: 'account-a',
    ),
    selectionSource: selection,
    localSource: ScheduleLocalRepository(_MemoryStorage()),
  );
}

class _FakeSelectionRepository implements SelectionRepository {
  _FakeSelectionRepository({required this.pages, this.delayedSearch});

  final Map<String?, SelectionOfferingPage> pages;
  final Completer<SelectionOfferingPage>? delayedSearch;
  CancelToken? firstSearchRequest;

  @override
  Future<List<Calendar>> calendars({CancelToken? cancelToken}) async =>
      <Calendar>[Calendar(id: '2026-spring', name: '2026 春', isCurrent: true)];

  @override
  Future<List<String>> grades(
    String calendarId, {
    CancelToken? cancelToken,
  }) async => <String>['2026'];

  @override
  Future<LatestUpdate?> latestUpdate({CancelToken? cancelToken}) async => null;

  @override
  Future<List<Major>> majors(String grade, {CancelToken? cancelToken}) async =>
      <Major>[Major(id: 'software', name: '软件工程', grade: grade)];

  @override
  Future<List<CourseNature>> natures({CancelToken? cancelToken}) async =>
      <CourseNature>[CourseNature(id: 'required', name: '必修')];

  @override
  Future<SelectionOfferingPage> offerings({
    required String calendarId,
    String? query,
    String? majorId,
    String? grade,
    String? natureId,
    String? cursor,
    int limit = 20,
    CancelToken? cancelToken,
  }) async {
    if (query == 'first') {
      firstSearchRequest = cancelToken;
      return delayedSearch!.future;
    }
    if (query == 'second') {
      return SelectionOfferingPage(
        items: <SelectionOffering>[_offering('current')],
        nextCursor: null,
        hasMore: false,
      );
    }
    return pages[cursor]!;
  }

  @override
  Future<List<TimeSlot>> timeslots(
    String offeringId, {
    CancelToken? cancelToken,
  }) async => <TimeSlot>[
    TimeSlot(
      offeringId: offeringId,
      courseId: offeringId,
      teacherName: null,
      weekday: offeringId.endsWith('a') ? 1 : 2,
      startSlot: 1,
      endSlot: 2,
      weeks: '1-4',
      weekNumbers: const <int>{1, 2, 3, 4},
      weeksUnknown: false,
      location: null,
      locationUnknown: true,
    ),
  ];
}

class _MemoryStorage implements ScheduleStorage {
  final Map<String, String> values = <String, String>{};

  @override
  Future<String?> read(String key) async => values[key];

  @override
  Future<void> remove(String key) async {
    values.remove(key);
  }

  @override
  Future<void> write(String key, String value) async {
    values[key] = value;
  }
}
