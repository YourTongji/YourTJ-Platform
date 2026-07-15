import 'dart:async';

import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/core/network/api_failure.dart';
import 'package:yourtj_mobile/features/schedule/data/schedule_local_repository.dart';
import 'package:yourtj_mobile/features/schedule/data/selection_repository.dart';
import 'package:yourtj_mobile/features/schedule/domain/schedule_controller.dart';
import 'package:yourtj_mobile/features/schedule/domain/schedule_models.dart';

void main() {
  test('calendar changes discard an older in-flight course result', () async {
    final _FakeSelectionRepository selection = _FakeSelectionRepository();
    final ScheduleController controller = _controller(selection);

    await controller.selectCalendar('calendar-1');
    final Future<void> staleLoad = controller.selectNature('nature-1');
    await Future<void>.delayed(Duration.zero);
    await controller.selectCalendar('calendar-2');
    selection.firstNatureResult.complete(<SelectionCourse>[
      _course(calendarId: 'calendar-1'),
    ]);
    await staleLoad;

    expect(controller.calendarId, 'calendar-2');
    expect(controller.natureId, isNull);
    expect(controller.courses, isEmpty);
    expect(controller.areCoursesLoading, isFalse);
    controller.dispose();
  });

  test(
    'rejects a teaching class without the exact selected calendar',
    () async {
      final _FakeSelectionRepository selection = _FakeSelectionRepository();
      final ScheduleController controller = _controller(selection);
      await controller.selectCalendar('calendar-1');

      expect(
        controller.addCourse(_course(calendarId: null)),
        throwsA(isA<ApiFailure>()),
      );
      expect(
        controller.addCourse(_course(calendarId: 'calendar-2')),
        throwsA(isA<ApiFailure>()),
      );
      expect(selection.timeslotRequests, isEmpty);
      controller.dispose();
    },
  );

  test(
    'serializes different teaching-class additions without losing one',
    () async {
      final _FakeSelectionRepository selection = _FakeSelectionRepository();
      final _YieldingScheduleStorage storage = _YieldingScheduleStorage();
      final ScheduleController controller = _controller(
        selection,
        storage: storage,
      );
      await controller.selectCalendar('calendar-1');

      final List<ScheduleAddResult> results =
          await Future.wait(<Future<ScheduleAddResult>>[
            controller.addCourse(
              _course(id: 'teaching-class-1', calendarId: 'calendar-1'),
            ),
            controller.addCourse(
              _course(id: 'teaching-class-2', calendarId: 'calendar-1'),
            ),
          ]);

      expect(
        results.map((ScheduleAddResult result) => result.status),
        everyElement(ScheduleAddStatus.added),
      );
      expect(
        controller.scheduled
            .map((ScheduledCourse item) => item.course.id)
            .toSet(),
        <String>{'teaching-class-1', 'teaching-class-2'},
      );
      controller.dispose();
    },
  );
}

ScheduleController _controller(
  _FakeSelectionRepository selection, {
  ScheduleStorage? storage,
}) {
  return ScheduleController(
    scope: const ScheduleNamespace(
      environment: 'https://api.example/api/v2',
      principal: 'account-1',
    ),
    selectionSource: selection,
    localSource: ScheduleLocalRepository(storage ?? _MemoryScheduleStorage()),
  );
}

SelectionCourse _course({
  String id = 'teaching-class-1',
  required String? calendarId,
}) {
  return SelectionCourse(
    id: id,
    code: 'CS101',
    name: '程序设计',
    credit: 3,
    natureId: 'nature-1',
    calendarId: calendarId,
    campusId: null,
    teacherName: null,
    teacherNames: const <String>[],
  );
}

class _FakeSelectionRepository implements SelectionRepository {
  final Completer<List<SelectionCourse>> firstNatureResult =
      Completer<List<SelectionCourse>>();
  final List<String> timeslotRequests = <String>[];

  @override
  Future<List<SelectionCourse>> byMajor({
    required String calendarId,
    required String majorId,
    required String grade,
    CancelToken? cancelToken,
  }) async => const <SelectionCourse>[];

  @override
  Future<List<SelectionCourse>> byNature({
    required String calendarId,
    required String natureId,
    CancelToken? cancelToken,
  }) {
    if (calendarId == 'calendar-1') {
      return firstNatureResult.future;
    }
    return Future<List<SelectionCourse>>.value(const <SelectionCourse>[]);
  }

  @override
  Future<List<Calendar>> calendars({CancelToken? cancelToken}) async =>
      const <Calendar>[];

  @override
  Future<List<String>> grades(
    String calendarId, {
    CancelToken? cancelToken,
  }) async => const <String>[];

  @override
  Future<LatestUpdate?> latestUpdate({CancelToken? cancelToken}) async => null;

  @override
  Future<List<Major>> majors({
    required String calendarId,
    required String grade,
    CancelToken? cancelToken,
  }) async => const <Major>[];

  @override
  Future<List<CourseNature>> natures({CancelToken? cancelToken}) async =>
      const <CourseNature>[];

  @override
  Future<List<SelectionCourse>> search({
    required String calendarId,
    required String query,
    CancelToken? cancelToken,
  }) async => const <SelectionCourse>[];

  @override
  Future<List<TimeSlot>> timeslots(
    String teachingClassId, {
    CancelToken? cancelToken,
  }) async {
    timeslotRequests.add(teachingClassId);
    return const <TimeSlot>[];
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
