import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/core/network/api_failure.dart';
import 'package:yourtj_mobile/features/schedule/data/schedule_local_repository.dart';
import 'package:yourtj_mobile/features/schedule/domain/schedule_models.dart';

void main() {
  test('isolates schedules by environment, principal, and calendar', () async {
    final _MemoryScheduleStorage storage = _MemoryScheduleStorage();
    final ScheduleLocalRepository repository = ScheduleLocalRepository(storage);
    const ScheduleNamespace accountA = ScheduleNamespace(
      environment: 'https://api.example/api/v2',
      principal: 'account-a',
    );
    const ScheduleNamespace accountB = ScheduleNamespace(
      environment: 'https://api.example/api/v2',
      principal: 'account-b',
    );
    final List<ScheduledCourse> courses = <ScheduledCourse>[_scheduled()];

    await repository.save(
      namespace: accountA,
      calendarId: '2026-spring',
      courses: courses,
    );

    expect(
      await repository.load(namespace: accountA, calendarId: '2026-spring'),
      hasLength(1),
    );
    expect(
      await repository.load(namespace: accountA, calendarId: '2026-autumn'),
      isEmpty,
    );
    expect(
      await repository.load(namespace: accountB, calendarId: '2026-spring'),
      isEmpty,
    );
    expect(
      await repository.load(
        namespace: const ScheduleNamespace(
          environment: 'https://preview.example/api/v2',
          principal: 'account-a',
        ),
        calendarId: '2026-spring',
      ),
      isEmpty,
    );
  });

  test('fails closed to an empty schedule for malformed local JSON', () async {
    final _MemoryScheduleStorage storage = _MemoryScheduleStorage();
    const ScheduleNamespace namespace = ScheduleNamespace(
      environment: 'https://api.example/api/v2',
      principal: 'anonymous',
    );
    await storage.write(namespace.storageKey('current'), '{broken');

    final List<ScheduledCourse> loaded = await ScheduleLocalRepository(
      storage,
    ).load(namespace: namespace, calendarId: 'current');

    expect(loaded, isEmpty);
  });

  test('rejects a teaching class outside the target calendar', () async {
    final ScheduleLocalRepository repository = ScheduleLocalRepository(
      _MemoryScheduleStorage(),
    );

    expect(
      repository.save(
        namespace: const ScheduleNamespace(
          environment: 'https://api.example/api/v2',
          principal: 'account-a',
        ),
        calendarId: '2026-autumn',
        courses: <ScheduledCourse>[_scheduled()],
      ),
      throwsA(isA<ApiFailure>()),
    );
  });
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

ScheduledCourse _scheduled() {
  return ScheduledCourse(
    course: SelectionCourse(
      id: '1',
      code: 'CS101',
      name: '程序设计',
      credit: 3,
      natureId: 'required',
      calendarId: '2026-spring',
      campusId: 'siping',
      teacherName: '张老师',
      teacherNames: const <String>['张老师'],
    ),
    timeslots: <TimeSlot>[
      TimeSlot(
        courseId: '1',
        teacherName: '张老师',
        weekday: 2,
        startSlot: 3,
        endSlot: 4,
        weeks: null,
        location: '教学楼',
      ),
    ],
    colorIndex: 2,
  );
}
