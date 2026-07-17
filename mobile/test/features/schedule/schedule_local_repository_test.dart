import 'dart:convert';

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

  test('migrates current-main schema v3 from the per-scope v2 key', () async {
    final _MemoryScheduleStorage storage = _MemoryScheduleStorage();
    const ScheduleNamespace namespace = ScheduleNamespace(
      environment: 'https://api.example/api/v2',
      principal: 'account-a',
    );
    final String currentMain = jsonEncode(<String, Object>{
      'schemaVersion': 3,
      'items': <Object>[
        _legacyItem(
          id: 'offering-main',
          teacher: '张老师',
          calendarId: '2026-spring',
          startSlot: 20,
          endSlot: 20,
        ),
      ],
    });
    await storage.write(namespace.legacyStorageKey('2026-spring'), currentMain);

    final List<ScheduledCourse> loaded = await ScheduleLocalRepository(
      storage,
    ).load(namespace: namespace, calendarId: '2026-spring');

    expect(loaded.single.offering.offeringId, 'offering-main');
    expect(loaded.single.offering.calendarId, '2026-spring');
    expect(loaded.single.timeslots.single.startSlot, 20);
    expect(loaded.single.timeslots.single.endSlot, 20);
    final Object? rewritten = jsonDecode(
      storage.values[namespace.storageKey('2026-spring')]!,
    );
    expect((rewritten! as Map<String, dynamic>)['schemaVersion'], 4);
    expect(
      storage.values[namespace.legacyStorageKey('2026-spring')],
      currentMain,
    );
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
      expect((rewritten! as Map<String, dynamic>)['schemaVersion'], 4);
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

  test(
    'loads a schema v3 current schedule with safe review defaults',
    () async {
      final _MemoryScheduleStorage storage = _MemoryScheduleStorage();
      const ScheduleNamespace namespace = ScheduleNamespace(
        environment: 'https://api.example/api/v2',
        principal: 'account-a',
      );
      final ScheduledCourse source = _scheduled();
      final Map<String, dynamic> legacyOffering = source.offering.toJson()
        ..remove('reviewCount')
        ..remove('reviewAvg')
        ..remove('reviewScope');
      await storage.write(
        namespace.storageKey('2026-spring'),
        jsonEncode(<String, Object>{
          'schemaVersion': 3,
          'items': <Object>[
            <String, Object>{
              'offering': legacyOffering,
              'timeslots': <Object>[source.timeslots.single.toJson()],
              'colorIndex': 0,
            },
          ],
        }),
      );

      final List<ScheduledCourse> loaded = await ScheduleLocalRepository(
        storage,
      ).load(namespace: namespace, calendarId: '2026-spring');

      expect(loaded.single.offering.reviewCount, 0);
      expect(loaded.single.offering.reviewAvg, isNull);
      expect(
        loaded.single.offering.reviewScope,
        SelectionOfferingReviewScopeEnum.none,
      );
    },
  );

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

  test(
    'rejects a current envelope with a mismatched timeslot offering',
    () async {
      final _MemoryScheduleStorage storage = _MemoryScheduleStorage();
      const ScheduleNamespace namespace = ScheduleNamespace(
        environment: 'https://api.example/api/v2',
        principal: 'account-a',
      );
      final ScheduledCourse valid = _scheduled();
      final Map<String, dynamic> mismatchedTimeslot =
          valid.timeslots.single.toJson()..['offeringId'] = 'other-offering';
      await storage.write(
        namespace.storageKey('2026-spring'),
        jsonEncode(<String, Object>{
          'schemaVersion': 3,
          'items': <Object>[
            <String, Object>{
              'offering': valid.offering.toJson(),
              'timeslots': <Object>[valid.timeslots.single.toJson()],
              'colorIndex': 0,
            },
            <String, Object>{
              'offering': SelectionOffering(
                id: '2',
                offeringId: '2',
                code: 'CS102',
                teachingClassCode: null,
                name: '数据结构',
                credit: 3,
                natureId: 'required',
                calendarId: '2026-spring',
                campusId: null,
                facultyName: null,
                teachingLanguage: null,
                teacherName: null,
                teacherNames: const <String>[],
                startWeek: 1,
                endWeek: 16,
                weeksUnknown: false,
                scheduleUnknown: false,
                status: SelectionOfferingStatusEnum.unknown,
                catalogueCourseId: null,
                reviewCount: 0,
                reviewAvg: null,
                reviewScope: SelectionOfferingReviewScopeEnum.none,
              ).toJson(),
              'timeslots': <Object>[mismatchedTimeslot],
              'colorIndex': 1,
            },
          ],
        }),
      );

      final List<ScheduledCourse> loaded = await ScheduleLocalRepository(
        storage,
      ).load(namespace: namespace, calendarId: '2026-spring');

      expect(loaded, isEmpty);
    },
  );

  test('rejects saving week numbers outside the contract range', () async {
    final _MemoryScheduleStorage storage = _MemoryScheduleStorage();
    const ScheduleNamespace namespace = ScheduleNamespace(
      environment: 'https://api.example/api/v2',
      principal: 'account-a',
    );
    final ScheduledCourse valid = _scheduled();
    final TimeSlot sourceTimeslot = valid.timeslots.single;
    final ScheduledCourse invalid = ScheduledCourse(
      offering: valid.offering,
      timeslots: <TimeSlot>[
        TimeSlot(
          offeringId: sourceTimeslot.offeringId,
          courseId: sourceTimeslot.offeringId,
          teacherName: sourceTimeslot.teacherName,
          weekday: sourceTimeslot.weekday,
          startSlot: sourceTimeslot.startSlot,
          endSlot: sourceTimeslot.endSlot,
          weeks: sourceTimeslot.weeks,
          weekNumbers: const <int>{31},
          weeksUnknown: sourceTimeslot.weeksUnknown,
          location: sourceTimeslot.location,
          locationUnknown: sourceTimeslot.locationUnknown,
        ),
      ],
      colorIndex: valid.colorIndex,
    );

    await expectLater(
      ScheduleLocalRepository(storage).save(
        namespace: namespace,
        calendarId: '2026-spring',
        courses: <ScheduledCourse>[invalid],
      ),
      throwsA(
        isA<ApiFailure>().having(
          (ApiFailure failure) => failure.kind,
          'kind',
          ApiFailureKind.invalidInput,
        ),
      ),
    );

    expect(storage.values, isEmpty);
  });

  test('rejects contradictory known-week facts in current storage', () async {
    final _MemoryScheduleStorage storage = _MemoryScheduleStorage();
    const ScheduleNamespace namespace = ScheduleNamespace(
      environment: 'https://api.example/api/v2',
      principal: 'account-a',
    );
    final ScheduledCourse source = _scheduled();
    final Map<String, dynamic> timeslot = source.timeslots.single.toJson()
      ..['weekNumbers'] = const <int>[]
      ..['weeksUnknown'] = false;
    await storage.write(
      namespace.storageKey('2026-spring'),
      jsonEncode(<String, Object>{
        'schemaVersion': 3,
        'items': <Object>[
          <String, Object>{
            'offering': source.offering.toJson(),
            'timeslots': <Object>[timeslot],
            'colorIndex': 0,
          },
        ],
      }),
    );

    final List<ScheduledCourse> loaded = await ScheduleLocalRepository(
      storage,
    ).load(namespace: namespace, calendarId: '2026-spring');

    expect(loaded, isEmpty);
  });

  test(
    'rejects contradictory historical rating facts in current storage',
    () async {
      final _MemoryScheduleStorage storage = _MemoryScheduleStorage();
      const ScheduleNamespace namespace = ScheduleNamespace(
        environment: 'https://api.example/api/v2',
        principal: 'account-a',
      );
      final ScheduledCourse source = _scheduled();
      final Map<String, dynamic> offering = source.offering.toJson()
        ..['reviewCount'] = 2
        ..['reviewAvg'] = null
        ..['reviewScope'] = 'teacher';
      await storage.write(
        namespace.storageKey('2026-spring'),
        jsonEncode(<String, Object>{
          'schemaVersion': 4,
          'items': <Object>[
            <String, Object>{
              'offering': offering,
              'timeslots': <Object>[source.timeslots.single.toJson()],
              'colorIndex': 0,
            },
          ],
        }),
      );

      final List<ScheduledCourse> loaded = await ScheduleLocalRepository(
        storage,
      ).load(namespace: namespace, calendarId: '2026-spring');

      expect(loaded, isEmpty);
    },
  );

  test(
    'rejects contradictory aggregate week facts in current storage',
    () async {
      final _MemoryScheduleStorage storage = _MemoryScheduleStorage();
      const ScheduleNamespace namespace = ScheduleNamespace(
        environment: 'https://api.example/api/v2',
        principal: 'account-a',
      );
      final ScheduledCourse source = _scheduled();
      final Map<String, dynamic> offering = source.offering.toJson()
        ..['weeksUnknown'] = true;
      await storage.write(
        namespace.storageKey('2026-spring'),
        jsonEncode(<String, Object>{
          'schemaVersion': 3,
          'items': <Object>[
            <String, Object>{
              'offering': offering,
              'timeslots': <Object>[source.timeslots.single.toJson()],
              'colorIndex': 0,
            },
          ],
        }),
      );

      final List<ScheduledCourse> loaded = await ScheduleLocalRepository(
        storage,
      ).load(namespace: namespace, calendarId: '2026-spring');

      expect(loaded, isEmpty);
    },
  );

  test('rejects a legacy timeslot with a mismatched course alias', () async {
    final _MemoryScheduleStorage storage = _MemoryScheduleStorage();
    const ScheduleNamespace namespace = ScheduleNamespace(
      environment: 'https://api.example/api/v2',
      principal: 'account-a',
    );
    await storage.write(
      namespace.legacyStorageKey('2026-spring'),
      jsonEncode(<String, Object>{
        'schemaVersion': 3,
        'items': <Object>[
          _legacyItem(
            id: 'offering-a',
            teacher: '张老师',
            timeslotCourseId: 'offering-b',
          ),
        ],
      }),
    );

    final List<ScheduledCourse> loaded = await ScheduleLocalRepository(
      storage,
    ).load(namespace: namespace, calendarId: '2026-spring');

    expect(loaded, isEmpty);
    expect(storage.values[namespace.storageKey('2026-spring')], isNull);
  });

  test('round trips the contract maximum of one hundred timeslots', () async {
    final _MemoryScheduleStorage storage = _MemoryScheduleStorage();
    const ScheduleNamespace namespace = ScheduleNamespace(
      environment: 'https://api.example/api/v2',
      principal: 'account-a',
    );
    final ScheduledCourse source = _scheduled();
    final List<TimeSlot> timeslots = List<TimeSlot>.generate(100, (int index) {
      final int slot = index % 20 + 1;
      return TimeSlot(
        offeringId: source.offering.offeringId,
        courseId: source.offering.offeringId,
        teacherName: null,
        weekday: index % 7 + 1,
        startSlot: slot,
        endSlot: slot,
        weeks: '1',
        weekNumbers: const <int>{1},
        weeksUnknown: false,
        location: null,
        locationUnknown: true,
      );
    });

    await ScheduleLocalRepository(storage).save(
      namespace: namespace,
      calendarId: '2026-spring',
      courses: <ScheduledCourse>[
        ScheduledCourse(
          offering: source.offering,
          timeslots: timeslots,
          colorIndex: 0,
        ),
      ],
    );
    final List<ScheduledCourse> loaded = await ScheduleLocalRepository(
      storage,
    ).load(namespace: namespace, calendarId: '2026-spring');

    expect(loaded.single.timeslots, hasLength(100));
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
      reviewCount: 0,
      reviewAvg: null,
      reviewScope: SelectionOfferingReviewScopeEnum.none,
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
  String? calendarId,
  int startSlot = 3,
  int endSlot = 4,
  String? timeslotCourseId,
}) {
  final Map<String, Object?> course = <String, Object?>{
    'id': id,
    'code': 'CS101',
    'name': '程序设计',
    'credit': 3,
    'natureId': 'required',
    'campusId': 'siping',
    'teacherName': teacher,
    'teacherNames': <String>[teacher],
  };
  if (calendarId != null) {
    course['calendarId'] = calendarId;
  }
  return <String, Object?>{
    'course': course,
    'timeslots': <Object>[
      <String, Object?>{
        'courseId': timeslotCourseId ?? id,
        'teacherName': teacher,
        'weekday': 2,
        'startSlot': startSlot,
        'endSlot': endSlot,
        'weeks': '1-4',
        'location': '教学楼',
      },
    ],
    'colorIndex': 2,
  };
}
