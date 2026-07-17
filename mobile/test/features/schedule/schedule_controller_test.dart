import 'dart:async';
import 'dart:convert';

import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/core/network/api_failure.dart';
import 'package:yourtj_mobile/features/schedule/data/schedule_local_repository.dart';
import 'package:yourtj_mobile/features/schedule/data/selection_repository.dart';
import 'package:yourtj_mobile/features/schedule/domain/schedule_controller.dart';
import 'package:yourtj_mobile/features/schedule/domain/schedule_models.dart';

void main() {
  test(
    'calendar changes cancel stale results and reset every filter',
    () async {
      final Completer<SelectionOfferingPage> delayed =
          Completer<SelectionOfferingPage>();
      final _FakeSelectionRepository selection = _FakeSelectionRepository(
        delayedCalendarId: 'calendar-1',
        delayedOfferings: delayed,
      );
      final ScheduleController controller = _controller(selection);
      addTearDown(controller.dispose);

      await controller.selectCalendar('calendar-1');
      controller.setMode(SelectionBrowseMode.nature);
      final Future<void> staleLoad = controller.selectNature('required');
      await Future<void>.delayed(Duration.zero);
      await controller.selectCalendar('calendar-2');
      delayed.complete(
        _page(<SelectionOffering>[
          _offering('stale', calendarId: 'calendar-1'),
        ]),
      );
      await staleLoad;

      expect(controller.calendarId, 'calendar-2');
      expect(controller.mode, SelectionBrowseMode.major);
      expect(controller.natureId, isNull);
      expect(controller.query, isEmpty);
      expect(controller.weekday, isNull);
      expect(controller.startSlot, isNull);
      expect(controller.endSlot, isNull);
      expect(controller.week, isNull);
      expect(controller.includeUnknownSchedule, isTrue);
      expect(controller.offerings, isEmpty);
      expect(selection.requests.single.cancelToken?.isCancelled, isTrue);
    },
  );

  test('calendar changes clear active search and time filters', () async {
    final _FakeSelectionRepository selection = _FakeSelectionRepository();
    final ScheduleController controller = _controller(selection);
    addTearDown(controller.dispose);

    await controller.selectCalendar('calendar-1');
    controller.setMode(SelectionBrowseMode.search);
    await controller.submitSearch('数据');
    await controller.updateTimeFilters(
      weekday: 3,
      startSlot: 14,
      endSlot: 20,
      week: 8,
      includeUnknownSchedule: false,
    );
    await controller.selectCalendar('calendar-2');

    expect(controller.mode, SelectionBrowseMode.major);
    expect(controller.query, isEmpty);
    expect(controller.weekday, isNull);
    expect(controller.startSlot, isNull);
    expect(controller.endSlot, isNull);
    expect(controller.week, isNull);
    expect(controller.includeUnknownSchedule, isTrue);
    expect(controller.offerings, isEmpty);
  });

  test('changing grade preserves an active nature browse', () async {
    final _FakeSelectionRepository selection = _FakeSelectionRepository(
      offeringsPage: _page(<SelectionOffering>[_offering('nature-course')]),
    );
    final ScheduleController controller = _controller(selection);
    addTearDown(controller.dispose);

    await controller.selectCalendar('calendar-1');
    controller.setMode(SelectionBrowseMode.nature);
    await controller.selectNature('required');
    await controller.selectGrade('2026');

    expect(controller.mode, SelectionBrowseMode.nature);
    expect(controller.natureId, 'required');
    expect(controller.offerings.single.offeringId, 'nature-course');
    expect(selection.requests, hasLength(1));
  });

  test(
    'paginates and keeps parallel offerings with the same course code',
    () async {
      final _FakeSelectionRepository selection = _FakeSelectionRepository(
        offeringsPage: SelectionOfferingPage(
          items: <SelectionOffering>[_offering('offering-a')],
          nextCursor: 'next',
          hasMore: true,
        ),
        nextOfferingsPage: SelectionOfferingPage(
          items: <SelectionOffering>[
            _offering('offering-a'),
            _offering('offering-b'),
          ],
          nextCursor: null,
          hasMore: false,
        ),
      );
      final ScheduleController controller = _controller(selection);
      addTearDown(controller.dispose);

      await controller.selectCalendar('calendar-1');
      await controller.selectGrade('2026');
      await controller.selectMajor('software');
      await controller.loadMore();

      expect(
        controller.offerings.map((SelectionOffering item) => item.offeringId),
        <String>['offering-a', 'offering-b'],
      );
      expect(selection.requests.last.cursor, 'next');
      expect(controller.hasMore, isFalse);
    },
  );

  test('ignores an obsolete offering response after search changes', () async {
    final Completer<SelectionOfferingPage> delayed =
        Completer<SelectionOfferingPage>();
    final _FakeSelectionRepository selection = _FakeSelectionRepository(
      delayedSearch: delayed,
    );
    final ScheduleController controller = _controller(selection);
    addTearDown(controller.dispose);

    await controller.selectCalendar('calendar-1');
    controller.setMode(SelectionBrowseMode.search);
    final Future<void> obsolete = controller.submitSearch('first');
    await Future<void>.delayed(Duration.zero);
    await controller.submitSearch('second');
    delayed.complete(_page(<SelectionOffering>[_offering('obsolete')]));
    await obsolete;

    expect(controller.offerings.single.offeringId, 'current');
    expect(selection.firstSearchRequest?.isCancelled, isTrue);
  });

  test('grade selection cannot invalidate pending local hydration', () async {
    final _DelayedReadScheduleStorage storage = _DelayedReadScheduleStorage();
    storage.values[_namespace.storageKey('calendar-1')] = _encodedSchedule(
      offeringId: 'saved-offering',
    );
    final ScheduleController controller = _controller(
      _FakeSelectionRepository(),
      storage: storage,
    );
    addTearDown(controller.dispose);
    bool didNotifyHydratedSchedule = false;
    controller.addListener(() {
      didNotifyHydratedSchedule =
          didNotifyHydratedSchedule || controller.scheduled.isNotEmpty;
    });

    final Future<void> calendarLoad = controller.selectCalendar('calendar-1');
    await storage.readStarted.future;
    await controller.selectGrade('2026');
    storage.allowRead.complete();
    await calendarLoad;

    expect(controller.scheduled.single.offering.offeringId, 'saved-offering');
    expect(didNotifyHydratedSchedule, isTrue);
  });

  test('rejects an offering outside the selected calendar', () async {
    final _FakeSelectionRepository selection = _FakeSelectionRepository();
    final ScheduleController controller = _controller(selection);
    addTearDown(controller.dispose);
    await controller.selectCalendar('calendar-1');

    await expectLater(
      controller.addOffering(_offering('other', calendarId: 'calendar-2')),
      throwsA(isA<ApiFailure>()),
    );
    expect(selection.timeslotRequests, isEmpty);
  });

  test(
    'serializes concurrent additions and rejects a confirmed overlap',
    () async {
      final _FakeSelectionRepository selection = _FakeSelectionRepository(
        timeslotsByOffering: <String, List<TimeSlot>>{
          'offering-a': <TimeSlot>[_slot('offering-a', start: 3, end: 4)],
          'offering-b': <TimeSlot>[_slot('offering-b', start: 4, end: 5)],
        },
      );
      final ScheduleController controller = _controller(
        selection,
        storage: _YieldingScheduleStorage(),
      );
      addTearDown(controller.dispose);
      await controller.selectCalendar('calendar-1');

      final List<ScheduleAddResult> results =
          await Future.wait(<Future<ScheduleAddResult>>[
            controller.addOffering(_offering('offering-a')),
            controller.addOffering(_offering('offering-b')),
          ]);

      expect(
        results.map((ScheduleAddResult result) => result.status).toSet(),
        <ScheduleAddStatus>{
          ScheduleAddStatus.added,
          ScheduleAddStatus.conflict,
        },
      );
      expect(
        results
            .singleWhere(
              (ScheduleAddResult result) =>
                  result.status == ScheduleAddStatus.conflict,
            )
            .conflict
            ?.kind,
        ScheduleConflictKind.confirmed,
      );
      expect(controller.scheduled, hasLength(1));
    },
  );

  test('serializes non-conflicting additions without losing either', () async {
    final _FakeSelectionRepository selection = _FakeSelectionRepository(
      timeslotsByOffering: <String, List<TimeSlot>>{
        'offering-a': <TimeSlot>[_slot('offering-a', start: 3, end: 4)],
        'offering-b': <TimeSlot>[
          _slot('offering-b', weekday: 2, start: 3, end: 4),
        ],
      },
    );
    final ScheduleController controller = _controller(
      selection,
      storage: _YieldingScheduleStorage(),
    );
    addTearDown(controller.dispose);
    await controller.selectCalendar('calendar-1');

    final List<ScheduleAddResult> results =
        await Future.wait(<Future<ScheduleAddResult>>[
          controller.addOffering(_offering('offering-a')),
          controller.addOffering(_offering('offering-b')),
        ]);

    expect(
      results.map((ScheduleAddResult result) => result.status),
      everyElement(ScheduleAddStatus.added),
    );
    expect(
      controller.scheduled
          .map((ScheduledCourse item) => item.offering.offeringId)
          .toSet(),
      <String>{'offering-a', 'offering-b'},
    );
  });

  test(
    'confirmAdd rechecks and never overrides a confirmed conflict',
    () async {
      final _FakeSelectionRepository selection = _FakeSelectionRepository(
        timeslotsByOffering: <String, List<TimeSlot>>{
          'offering-a': <TimeSlot>[_slot('offering-a', start: 3, end: 4)],
        },
      );
      final ScheduleController controller = _controller(selection);
      addTearDown(controller.dispose);
      await controller.selectCalendar('calendar-1');
      await controller.addOffering(_offering('offering-a'));

      await expectLater(
        controller.confirmAdd(_offering('offering-b'), <TimeSlot>[
          _slot('offering-b', start: 4, end: 5),
        ]),
        throwsA(
          isA<ApiFailure>().having(
            (ApiFailure failure) => failure.kind,
            'kind',
            ApiFailureKind.conflict,
          ),
        ),
      );
      expect(controller.scheduled, hasLength(1));
    },
  );

  test(
    'serializes concurrent confirmations and rechecks the queued conflict',
    () async {
      final _FakeSelectionRepository selection = _FakeSelectionRepository(
        timeslotsByOffering: <String, List<TimeSlot>>{
          'anchor': const <TimeSlot>[],
          'offering-a': <TimeSlot>[_slot('offering-a', start: 3, end: 4)],
          'offering-b': <TimeSlot>[_slot('offering-b', start: 4, end: 5)],
        },
      );
      final ScheduleController controller = _controller(
        selection,
        storage: _YieldingScheduleStorage(),
      );
      addTearDown(controller.dispose);
      await controller.selectCalendar('calendar-1');
      final ScheduleAddResult anchor = await controller.addOffering(
        _offering('anchor', scheduleUnknown: true),
      );
      expect(anchor.status, ScheduleAddStatus.added);
      final ScheduleAddResult firstPending = await controller.addOffering(
        _offering('offering-a'),
      );
      final ScheduleAddResult secondPending = await controller.addOffering(
        _offering('offering-b'),
      );
      expect(firstPending.conflict?.kind, ScheduleConflictKind.possible);
      expect(secondPending.conflict?.kind, ScheduleConflictKind.possible);

      Future<Object?> captureConfirmation(ScheduleAddResult pending) async {
        try {
          await controller.confirmAdd(
            pending.pendingOffering!,
            pending.pendingTimeslots!,
          );
          return null;
        } on Object catch (error) {
          return error;
        }
      }

      final List<Object?> outcomes = await Future.wait(<Future<Object?>>[
        captureConfirmation(firstPending),
        captureConfirmation(secondPending),
      ]);

      expect(outcomes.whereType<ApiFailure>(), hasLength(1));
      expect(
        outcomes.whereType<ApiFailure>().single.kind,
        ApiFailureKind.conflict,
      );
      expect(
        outcomes.where((Object? outcome) => outcome == null),
        hasLength(1),
      );
      expect(controller.scheduled, hasLength(2));
    },
  );

  test('requires confirmation for a completely unknown schedule', () async {
    final _FakeSelectionRepository selection = _FakeSelectionRepository(
      timeslotsByOffering: <String, List<TimeSlot>>{
        'known': <TimeSlot>[_slot('known', start: 3, end: 4)],
        'unknown': const <TimeSlot>[],
      },
    );
    final ScheduleController controller = _controller(selection);
    addTearDown(controller.dispose);
    await controller.selectCalendar('calendar-1');
    await controller.addOffering(_offering('known'));

    final ScheduleAddResult result = await controller.addOffering(
      _offering('unknown', scheduleUnknown: true),
    );

    expect(result.status, ScheduleAddStatus.conflict);
    expect(result.conflict?.kind, ScheduleConflictKind.possible);
    expect(result.conflict?.candidateSlot, isNull);
    await controller.confirmAdd(
      result.pendingOffering!,
      result.pendingTimeslots!,
    );
    expect(controller.scheduled, hasLength(2));
  });

  test('propagates free-time filters and preserves slot twenty', () async {
    final _FakeSelectionRepository selection = _FakeSelectionRepository(
      timeslotsByOffering: <String, List<TimeSlot>>{
        'late': <TimeSlot>[_slot('late', start: 20, end: 20)],
      },
    );
    final ScheduleController controller = _controller(selection);
    addTearDown(controller.dispose);
    await controller.selectCalendar('calendar-1');
    await controller.selectGrade('2026');
    await controller.selectMajor('software');

    await controller.updateTimeFilters(
      weekday: 5,
      startSlot: 14,
      endSlot: 20,
      week: 16,
      includeUnknownSchedule: false,
    );
    final _OfferingRequest request = selection.requests.last;

    expect(request.calendarId, 'calendar-1');
    expect(request.majorId, 'software');
    expect(request.grade, '2026');
    expect(request.weekday, 5);
    expect(request.startSlot, 14);
    expect(request.endSlot, 20);
    expect(request.week, 16);
    expect(request.includeUnknownSchedule, isFalse);

    final ScheduleAddResult result = await controller.addOffering(
      _offering('late'),
    );
    expect(result.status, ScheduleAddStatus.added);
    expect(controller.scheduled.single.timeslots.single.startSlot, 20);
  });

  test('rejects partial slot filters before making a request', () async {
    final _FakeSelectionRepository selection = _FakeSelectionRepository();
    final ScheduleController controller = _controller(selection);
    addTearDown(controller.dispose);
    await controller.selectCalendar('calendar-1');
    await controller.selectGrade('2026');
    await controller.selectMajor('software');
    final int initialRequests = selection.requests.length;

    await expectLater(
      controller.updateTimeFilters(
        weekday: 1,
        startSlot: null,
        endSlot: null,
        week: null,
        includeUnknownSchedule: true,
      ),
      throwsA(isA<ApiFailure>()),
    );

    expect(selection.requests, hasLength(initialRequests));
  });

  test('rejects excluding unknown schedules without a time range', () async {
    final _FakeSelectionRepository selection = _FakeSelectionRepository();
    final ScheduleController controller = _controller(selection);
    addTearDown(controller.dispose);
    await controller.selectCalendar('calendar-1');
    controller.setMode(SelectionBrowseMode.search);
    await controller.submitSearch('数据');
    final int initialRequests = selection.requests.length;

    await expectLater(
      controller.updateTimeFilters(
        weekday: null,
        startSlot: null,
        endSlot: null,
        week: null,
        includeUnknownSchedule: false,
      ),
      throwsA(
        isA<ApiFailure>().having(
          (ApiFailure failure) => failure.kind,
          'kind',
          ApiFailureKind.invalidInput,
        ),
      ),
    );

    expect(selection.requests, hasLength(initialRequests));
    expect(controller.includeUnknownSchedule, isTrue);
  });

  test(
    'rejects malformed timeslots instead of adding without conflicts',
    () async {
      final _FakeSelectionRepository selection = _FakeSelectionRepository(
        timeslotsByOffering: <String, List<TimeSlot>>{
          'offering-a': <TimeSlot>[
            _slot('different-offering', start: 3, end: 4),
          ],
        },
      );
      final ScheduleController controller = _controller(selection);
      addTearDown(controller.dispose);
      await controller.selectCalendar('calendar-1');

      await expectLater(
        controller.addOffering(_offering('offering-a')),
        throwsA(
          isA<ApiFailure>().having(
            (ApiFailure failure) => failure.kind,
            'kind',
            ApiFailureKind.unexpected,
          ),
        ),
      );

      expect(controller.scheduled, isEmpty);
    },
  );

  test('rejects a timeslot with a mismatched compatibility alias', () async {
    final _FakeSelectionRepository selection = _FakeSelectionRepository(
      timeslotsByOffering: <String, List<TimeSlot>>{
        'offering-a': <TimeSlot>[
          _slot('offering-a', courseId: 'different-offering', start: 3, end: 4),
        ],
      },
    );
    final ScheduleController controller = _controller(selection);
    addTearDown(controller.dispose);
    await controller.selectCalendar('calendar-1');

    await expectLater(
      controller.addOffering(_offering('offering-a')),
      throwsA(
        isA<ApiFailure>().having(
          (ApiFailure failure) => failure.kind,
          'kind',
          ApiFailureKind.unexpected,
        ),
      ),
    );

    expect(controller.scheduled, isEmpty);
  });
}

const ScheduleNamespace _namespace = ScheduleNamespace(
  environment: 'https://api.example/api/v2',
  principal: 'account-1',
);

ScheduleController _controller(
  _FakeSelectionRepository selection, {
  ScheduleStorage? storage,
}) {
  return ScheduleController(
    scope: _namespace,
    selectionSource: selection,
    localSource: ScheduleLocalRepository(storage ?? _MemoryScheduleStorage()),
  );
}

SelectionOffering _offering(
  String offeringId, {
  String calendarId = 'calendar-1',
  bool scheduleUnknown = false,
}) {
  return SelectionOffering(
    id: offeringId,
    offeringId: offeringId,
    code: 'CS101',
    teachingClassCode: offeringId,
    name: '程序设计',
    credit: 3,
    natureId: 'required',
    calendarId: calendarId,
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

TimeSlot _slot(
  String offeringId, {
  String? courseId,
  int weekday = 1,
  required int start,
  required int end,
}) {
  return TimeSlot(
    offeringId: offeringId,
    courseId: courseId ?? offeringId,
    teacherName: null,
    weekday: weekday,
    startSlot: start,
    endSlot: end,
    weeks: '1-16',
    weekNumbers: const <int>{1, 2, 3, 4},
    weeksUnknown: false,
    location: null,
    locationUnknown: true,
  );
}

SelectionOfferingPage _page(List<SelectionOffering> items) {
  return SelectionOfferingPage(items: items, nextCursor: null, hasMore: false);
}

String _encodedSchedule({required String offeringId}) {
  final SelectionOffering offering = _offering(offeringId);
  final TimeSlot timeslot = _slot(offeringId, start: 3, end: 4);
  return jsonEncode(<String, Object>{
    'schemaVersion': 3,
    'items': <Object>[
      <String, Object>{
        'offering': offering.toJson(),
        'timeslots': <Object>[timeslot.toJson()],
        'colorIndex': 0,
      },
    ],
  });
}

class _OfferingRequest {
  const _OfferingRequest({
    required this.calendarId,
    required this.query,
    required this.majorId,
    required this.grade,
    required this.natureId,
    required this.weekday,
    required this.startSlot,
    required this.endSlot,
    required this.week,
    required this.includeUnknownSchedule,
    required this.cursor,
    required this.cancelToken,
  });

  final String calendarId;
  final String? query;
  final String? majorId;
  final String? grade;
  final String? natureId;
  final int? weekday;
  final int? startSlot;
  final int? endSlot;
  final int? week;
  final bool includeUnknownSchedule;
  final String? cursor;
  final CancelToken? cancelToken;
}

class _FakeSelectionRepository implements SelectionRepository {
  _FakeSelectionRepository({
    this.delayedCalendarId,
    this.delayedOfferings,
    this.offeringsPage,
    this.nextOfferingsPage,
    this.delayedSearch,
    Map<String, List<TimeSlot>>? timeslotsByOffering,
  }) : timeslotsByOffering = timeslotsByOffering ?? <String, List<TimeSlot>>{};

  final String? delayedCalendarId;
  final Completer<SelectionOfferingPage>? delayedOfferings;
  final SelectionOfferingPage? offeringsPage;
  final SelectionOfferingPage? nextOfferingsPage;
  final Completer<SelectionOfferingPage>? delayedSearch;
  final Map<String, List<TimeSlot>> timeslotsByOffering;
  final List<_OfferingRequest> requests = <_OfferingRequest>[];
  final List<String> timeslotRequests = <String>[];
  CancelToken? firstSearchRequest;

  @override
  Future<List<Calendar>> calendars({CancelToken? cancelToken}) async =>
      const <Calendar>[];

  @override
  Future<List<String>> grades(
    String calendarId, {
    CancelToken? cancelToken,
  }) async => <String>['2026'];

  @override
  Future<LatestUpdate?> latestUpdate({CancelToken? cancelToken}) async => null;

  @override
  Future<List<Major>> majors({
    required String calendarId,
    required String grade,
    CancelToken? cancelToken,
  }) async => <Major>[
    Major(id: 'software', name: '软件工程', facultyId: null, grade: grade),
  ];

  @override
  Future<List<CourseNature>> natures(
    String calendarId, {
    CancelToken? cancelToken,
  }) async => const <CourseNature>[];

  @override
  Future<SelectionOfferingPage> offerings({
    required String calendarId,
    String? query,
    String? majorId,
    String? grade,
    String? natureId,
    int? weekday,
    int? startSlot,
    int? endSlot,
    int? week,
    bool includeUnknownSchedule = true,
    String? cursor,
    int limit = 20,
    CancelToken? cancelToken,
  }) {
    requests.add(
      _OfferingRequest(
        calendarId: calendarId,
        query: query,
        majorId: majorId,
        grade: grade,
        natureId: natureId,
        weekday: weekday,
        startSlot: startSlot,
        endSlot: endSlot,
        week: week,
        includeUnknownSchedule: includeUnknownSchedule,
        cursor: cursor,
        cancelToken: cancelToken,
      ),
    );
    if (query == 'first' && delayedSearch != null) {
      firstSearchRequest = cancelToken;
      return delayedSearch!.future;
    }
    if (query == 'second') {
      return Future<SelectionOfferingPage>.value(
        _page(<SelectionOffering>[_offering('current')]),
      );
    }
    if (calendarId == delayedCalendarId) {
      return delayedOfferings!.future;
    }
    if (cursor != null && nextOfferingsPage != null) {
      return Future<SelectionOfferingPage>.value(nextOfferingsPage);
    }
    return Future<SelectionOfferingPage>.value(
      offeringsPage ?? _page(const <SelectionOffering>[]),
    );
  }

  @override
  Future<List<TimeSlot>> timeslots(
    String offeringId, {
    CancelToken? cancelToken,
  }) async {
    timeslotRequests.add(offeringId);
    return timeslotsByOffering[offeringId] ?? const <TimeSlot>[];
  }
}

class _MemoryScheduleStorage implements ScheduleStorage {
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

class _YieldingScheduleStorage extends _MemoryScheduleStorage {
  @override
  Future<void> write(String key, String value) async {
    await Future<void>.delayed(Duration.zero);
    await super.write(key, value);
  }
}

class _DelayedReadScheduleStorage extends _MemoryScheduleStorage {
  final Completer<void> readStarted = Completer<void>();
  final Completer<void> allowRead = Completer<void>();
  bool _hasDelayedRead = false;

  @override
  Future<String?> read(String key) async {
    if (!_hasDelayedRead) {
      _hasDelayedRead = true;
      readStarted.complete();
      await allowRead.future;
    }
    return super.read(key);
  }
}
