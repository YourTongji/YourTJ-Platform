import 'dart:convert';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../core/network/api_failure.dart';
import '../domain/schedule_models.dart';

abstract interface class ScheduleStorage {
  Future<String?> read(String key);

  Future<void> write(String key, String value);

  Future<void> remove(String key);
}

class SharedPreferencesScheduleStorage implements ScheduleStorage {
  SharedPreferencesScheduleStorage([SharedPreferencesAsync? preferences])
    : _preferences = preferences ?? SharedPreferencesAsync();

  final SharedPreferencesAsync _preferences;

  @override
  Future<String?> read(String key) => _preferences.getString(key);

  @override
  Future<void> write(String key, String value) =>
      _preferences.setString(key, value);

  @override
  Future<void> remove(String key) => _preferences.remove(key);
}

class ScheduleLocalRepository {
  const ScheduleLocalRepository(this._storage);

  static const int _schemaVersion = 3;
  final ScheduleStorage _storage;

  Future<List<ScheduledCourse>> load({
    required ScheduleNamespace namespace,
    required String calendarId,
  }) async {
    try {
      final String? encoded = await _storage.read(
        namespace.storageKey(calendarId),
      );
      if (encoded == null || encoded.isEmpty) {
        return const <ScheduledCourse>[];
      }
      final Object? decoded = jsonDecode(encoded);
      if (decoded is! Map || decoded['schemaVersion'] != _schemaVersion) {
        return const <ScheduledCourse>[];
      }
      final Object? rawItems = decoded['items'];
      if (rawItems is! List || rawItems.length > 100) {
        return const <ScheduledCourse>[];
      }
      final List<ScheduledCourse> items = <ScheduledCourse>[];
      for (final Object? rawItem in rawItems) {
        final ScheduledCourse? item = _decodeItem(rawItem);
        if (item != null &&
            item.course.calendarId == calendarId &&
            !items.any(
              (ScheduledCourse existing) =>
                  existing.course.id == item.course.id,
            )) {
          items.add(item);
        }
      }
      return items;
    } on FormatException {
      return const <ScheduledCourse>[];
    } on Object {
      throw const ApiFailure(
        kind: ApiFailureKind.unexpected,
        message: '无法读取本机课表，请检查系统存储后重试',
      );
    }
  }

  Future<void> save({
    required ScheduleNamespace namespace,
    required String calendarId,
    required List<ScheduledCourse> courses,
  }) async {
    if (courses.length > 100) {
      throw const ApiFailure(
        kind: ApiFailureKind.invalidInput,
        message: '本机课表最多保存 100 门课程',
      );
    }
    if (courses.any(
      (ScheduledCourse item) => item.course.calendarId != calendarId,
    )) {
      throw const ApiFailure(
        kind: ApiFailureKind.invalidInput,
        message: '教学班不属于当前学期，请刷新后重试',
      );
    }
    final String encoded = jsonEncode(<String, Object>{
      'schemaVersion': _schemaVersion,
      'items': courses.map(_encodeItem).toList(growable: false),
    });
    try {
      await _storage.write(namespace.storageKey(calendarId), encoded);
    } on Object {
      throw const ApiFailure(
        kind: ApiFailureKind.unexpected,
        message: '无法保存本机课表，请检查系统存储后重试',
      );
    }
  }

  Future<void> clear({
    required ScheduleNamespace namespace,
    required String calendarId,
  }) async {
    try {
      await _storage.remove(namespace.storageKey(calendarId));
    } on Object {
      throw const ApiFailure(
        kind: ApiFailureKind.unexpected,
        message: '无法清空本机课表，请检查系统存储后重试',
      );
    }
  }

  Map<String, Object> _encodeItem(ScheduledCourse item) {
    return <String, Object>{
      'course': item.course.toJson(),
      'timeslots': item.timeslots
          .map((TimeSlot timeslot) => timeslot.toJson())
          .toList(growable: false),
      'colorIndex': item.colorIndex,
    };
  }

  ScheduledCourse? _decodeItem(Object? rawItem) {
    if (rawItem is! Map<Object?, Object?>) {
      return null;
    }
    final Object? rawCourse = rawItem['course'];
    final Object? rawTimeslots = rawItem['timeslots'];
    final Object? rawColorIndex = rawItem['colorIndex'];
    if (rawCourse is! Map ||
        rawTimeslots is! List ||
        rawTimeslots.length > 64) {
      return null;
    }
    try {
      final SelectionCourse course = SelectionCourse.fromJson(
        Map<String, dynamic>.from(rawCourse),
      );
      if (course.id.isEmpty || course.code.isEmpty || course.name.isEmpty) {
        return null;
      }
      final List<TimeSlot> timeslots = rawTimeslots
          .whereType<Map<Object?, Object?>>()
          .map(
            (Map<Object?, Object?> rawTimeslot) =>
                TimeSlot.fromJson(Map<String, dynamic>.from(rawTimeslot)),
          )
          .where(_isValidTimeslot)
          .toList(growable: false);
      return ScheduledCourse(
        course: course,
        timeslots: timeslots,
        colorIndex: rawColorIndex is int ? rawColorIndex.clamp(0, 7) : 0,
      );
    } on Object {
      return null;
    }
  }

  bool _isValidTimeslot(TimeSlot timeslot) {
    return timeslot.weekday >= 1 &&
        timeslot.weekday <= 7 &&
        timeslot.startSlot >= 1 &&
        timeslot.startSlot <= 13 &&
        timeslot.endSlot >= timeslot.startSlot &&
        timeslot.endSlot <= 13;
  }
}

final Provider<ScheduleLocalRepository> scheduleLocalRepositoryProvider =
    Provider<ScheduleLocalRepository>((Ref ref) {
      return ScheduleLocalRepository(SharedPreferencesScheduleStorage());
    });
