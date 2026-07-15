import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
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

  test('migrates schema v2 and keeps parallel teaching classes', () async {
    final _MemoryScheduleStorage storage = _MemoryScheduleStorage();
    const ScheduleNamespace namespace = ScheduleNamespace(
      environment: 'https://api.example/api/v2',
      principal: 'account-a',
    );
    final String legacy = jsonEncode(<String, Object>{
      'schemaVersion': 2,
      'items': <Object>[
        _legacyItem(id: 'offering-a', teacher: '张老师'),
        _legacyItem(id: 'offering-b', teacher: '李老师'),
      ],
    });
    await storage.write(namespace.legacyStorageKey('2026-spring'), legacy);

    final ScheduleLocalRepository repository = ScheduleLocalRepository(storage);
    final List<ScheduledCourse> loaded = await repository.load(
      namespace: namespace,
      calendarId: '2026-spring',
    );

    expect(
      loaded.map((ScheduledCourse item) => item.offering.offeringId),
      <String>['offering-a', 'offering-b'],
    );
    expect(
      loaded.map((ScheduledCourse item) => item.offering.code).toSet(),
      <String>{'CS101'},
    );
    expect(loaded.first.timeslots.single.weekNumbers, <int>{1, 2, 3, 4});
    expect(storage.values[namespace.storageKey('2026-spring')], isNotNull);
    expect(storage.values[namespace.legacyStorageKey('2026-spring')], legacy);
  });

  test(
    'falls back to schema v2 when the current envelope is corrupt',
    () async {
      final _MemoryScheduleStorage storage = _MemoryScheduleStorage();
      const ScheduleNamespace namespace = ScheduleNamespace(
        environment: 'https://api.example/api/v2',
        principal: 'account-a',
      );
      final String legacy = jsonEncode(<String, Object>{
        'schemaVersion': 2,
        'items': <Object>[_legacyItem(id: 'offering-a', teacher: '张老师')],
      });
      await storage.write(namespace.storageKey('2026-spring'), '{broken');
      await storage.write(namespace.legacyStorageKey('2026-spring'), legacy);

      final List<ScheduledCourse> loaded = await ScheduleLocalRepository(
        storage,
      ).load(namespace: namespace, calendarId: '2026-spring');

      expect(loaded.single.offering.offeringId, 'offering-a');
      final Object? rewritten = jsonDecode(
        storage.values[namespace.storageKey('2026-spring')]!,
      );
      expect(rewritten, isA<Map<String, dynamic>>());
      expect((rewritten! as Map<String, dynamic>)['schemaVersion'], 3);
    },
  );

  test('keeps an intentionally empty schema v3 schedule empty', () async {
    final _MemoryScheduleStorage storage = _MemoryScheduleStorage();
    const ScheduleNamespace namespace = ScheduleNamespace(
      environment: 'https://api.example/api/v2',
      principal: 'account-a',
    );
    final String current = jsonEncode(<String, Object>{
      'schemaVersion': 3,
      'items': const <Object>[],
    });
    await storage.write(namespace.storageKey('2026-spring'), current);
    await storage.write(
      namespace.legacyStorageKey('2026-spring'),
      jsonEncode(<String, Object>{
        'schemaVersion': 2,
        'items': <Object>[_legacyItem(id: 'offering-a', teacher: '张老师')],
      }),
    );

    final List<ScheduledCourse> loaded = await ScheduleLocalRepository(
      storage,
    ).load(namespace: namespace, calendarId: '2026-spring');

    expect(loaded, isEmpty);
    expect(storage.values[namespace.storageKey('2026-spring')], current);
  });

  test('clear removes current and rollback-safe legacy schedules', () async {
    final _MemoryScheduleStorage storage = _MemoryScheduleStorage();
    const ScheduleNamespace namespace = ScheduleNamespace(
      environment: 'https://api.example/api/v2',
      principal: 'account-a',
    );
    await storage.write(namespace.storageKey('current'), 'current');
    await storage.write(namespace.legacyStorageKey('current'), 'legacy');

    await ScheduleLocalRepository(
      storage,
    ).clear(namespace: namespace, calendarId: 'current');

    expect(storage.values, isEmpty);
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
    offering: SelectionOffering(
      id: '1',
      offeringId: '1',
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
    ),
    timeslots: <TimeSlot>[
      TimeSlot(
        offeringId: '1',
        courseId: '1',
        teacherName: '张老师',
        weekday: 2,
        startSlot: 3,
        endSlot: 4,
        weeks: null,
        weekNumbers: const <int>{1, 2, 3, 4},
        weeksUnknown: false,
        location: '教学楼',
        locationUnknown: false,
      ),
    ],
    colorIndex: 2,
  );
}

Map<String, Object?> _legacyItem({
  required String id,
  required String teacher,
}) {
  return <String, Object?>{
    'course': <String, Object?>{
      'id': id,
      'code': 'CS101',
      'name': '程序设计',
      'credit': 3,
      'natureId': 'required',
      'campusId': 'siping',
      'teacherName': teacher,
      'teacherNames': <String>[teacher],
    },
    'timeslots': <Object>[
      <String, Object?>{
        'courseId': id,
        'teacherName': teacher,
        'weekday': 2,
        'startSlot': 3,
        'endSlot': 4,
        'weeks': '1-4',
        'location': '教学楼',
      },
    ],
    'colorIndex': 2,
  };
}
