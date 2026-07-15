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
  static const int _legacySchemaVersion = 2;
  final ScheduleStorage _storage;

  Future<List<ScheduledCourse>> load({
    required ScheduleNamespace namespace,
    required String calendarId,
  }) async {
    try {
      final String? current = await _storage.read(
        namespace.storageKey(calendarId),
      );
      if (current != null && current.isNotEmpty) {
        final List<ScheduledCourse>? decoded = _tryDecodeEnvelope(
          current,
          calendarId: calendarId,
        );
        if (decoded != null) {
          return decoded;
        }
      }

      final String? legacy = await _storage.read(
        namespace.legacyStorageKey(calendarId),
      );
      if (legacy == null || legacy.isEmpty) {
        return const <ScheduledCourse>[];
      }
      final List<ScheduledCourse> migrated =
          _tryDecodeEnvelope(
            legacy,
            calendarId: calendarId,
            expectedSchemaVersion: _legacySchemaVersion,
          ) ??
          const <ScheduledCourse>[];
      if (migrated.isNotEmpty) {
        await save(
          namespace: namespace,
          calendarId: calendarId,
          courses: migrated,
        );
      }
      return migrated;
    } on ApiFailure {
      rethrow;
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
      await _storage.remove(namespace.legacyStorageKey(calendarId));
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
      'offering': item.offering.toJson(),
      'timeslots': item.timeslots
          .map((TimeSlot timeslot) => timeslot.toJson())
          .toList(growable: false),
      'colorIndex': item.colorIndex,
    };
  }

  List<ScheduledCourse>? _tryDecodeEnvelope(
    String encoded, {
    required String calendarId,
    int expectedSchemaVersion = _schemaVersion,
  }) {
    try {
      final Object? decoded = jsonDecode(encoded);
      if (decoded is! Map ||
          decoded['schemaVersion'] != expectedSchemaVersion) {
        return null;
      }
      final Object? rawItems = decoded['items'];
      if (rawItems is! List || rawItems.length > 100) {
        return null;
      }
      final List<ScheduledCourse> items = <ScheduledCourse>[];
      for (final Object? rawItem in rawItems) {
        final ScheduledCourse? item = expectedSchemaVersion == _schemaVersion
            ? _decodeItem(rawItem, calendarId: calendarId)
            : _decodeLegacyItem(rawItem, calendarId: calendarId);
        if (item != null &&
            !items.any(
              (ScheduledCourse existing) =>
                  existing.offering.offeringId == item.offering.offeringId,
            )) {
          items.add(item);
        }
      }
      return rawItems.isNotEmpty && items.isEmpty ? null : items;
    } on FormatException {
      return null;
    }
  }

  ScheduledCourse? _decodeItem(Object? rawItem, {required String calendarId}) {
    if (rawItem is! Map<Object?, Object?>) {
      return null;
    }
    final Object? rawOffering = rawItem['offering'];
    final Object? rawTimeslots = rawItem['timeslots'];
    final Object? rawColorIndex = rawItem['colorIndex'];
    if (rawOffering is! Map ||
        rawTimeslots is! List ||
        rawTimeslots.length > 64) {
      return null;
    }
    try {
      final SelectionOffering offering = SelectionOffering.fromJson(
        Map<String, dynamic>.from(rawOffering),
      );
      if (offering.offeringId.isEmpty ||
          offering.code.isEmpty ||
          offering.name.isEmpty ||
          offering.calendarId != calendarId) {
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
        offering: offering,
        timeslots: timeslots,
        colorIndex: rawColorIndex is int ? rawColorIndex.clamp(0, 7) : 0,
      );
    } on Object {
      return null;
    }
  }

  ScheduledCourse? _decodeLegacyItem(
    Object? rawItem, {
    required String calendarId,
  }) {
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
      final Map<String, dynamic> course = Map<String, dynamic>.from(rawCourse);
      final String offeringId = course['id'] as String;
      final String code = course['code'] as String;
      final String name = course['name'] as String;
      if (offeringId.isEmpty || code.isEmpty || name.isEmpty) {
        return null;
      }
      final List<TimeSlot> timeslots = rawTimeslots
          .whereType<Map<Object?, Object?>>()
          .map(
            (Map<Object?, Object?> rawTimeslot) =>
                _migrateLegacyTimeslot(rawTimeslot, offeringId: offeringId),
          )
          .whereType<TimeSlot>()
          .where(_isValidTimeslot)
          .toList(growable: false);
      final Iterable<int> knownWeeks = timeslots
          .where((TimeSlot item) => !item.weeksUnknown)
          .expand((TimeSlot item) => item.weekNumbers);
      final List<int> orderedWeeks = knownWeeks.toSet().toList()..sort();
      final bool weeksUnknown =
          timeslots.isEmpty ||
          timeslots.any((TimeSlot item) => item.weeksUnknown);
      final Object? rawTeacherNames = course['teacherNames'];
      final List<String> teacherNames = rawTeacherNames is List
          ? rawTeacherNames.whereType<String>().toList(growable: false)
          : const <String>[];
      final SelectionOffering offering = SelectionOffering(
        id: offeringId,
        offeringId: offeringId,
        code: code,
        teachingClassCode: null,
        name: name,
        credit: course['credit'] as num?,
        natureId: course['natureId'] as String?,
        calendarId: calendarId,
        campusId: course['campusId'] as String?,
        facultyName: null,
        teachingLanguage: null,
        teacherName: course['teacherName'] as String?,
        teacherNames: teacherNames,
        startWeek: orderedWeeks.isEmpty ? null : orderedWeeks.first,
        endWeek: orderedWeeks.isEmpty ? null : orderedWeeks.last,
        weeksUnknown: weeksUnknown,
        scheduleUnknown: timeslots.isEmpty,
        status: SelectionOfferingStatusEnum.unknown,
        catalogueCourseId: null,
      );
      return ScheduledCourse(
        offering: offering,
        timeslots: timeslots,
        colorIndex: rawColorIndex is int ? rawColorIndex.clamp(0, 7) : 0,
      );
    } on Object {
      return null;
    }
  }

  TimeSlot? _migrateLegacyTimeslot(
    Map<Object?, Object?> rawTimeslot, {
    required String offeringId,
  }) {
    try {
      final Map<String, dynamic> value = Map<String, dynamic>.from(rawTimeslot);
      final String? weeks = value['weeks'] as String?;
      final Set<int>? parsedWeeks = weeks == null
          ? null
          : parseCourseWeeks(weeks);
      final String? location = value['location'] as String?;
      return TimeSlot(
        offeringId: offeringId,
        courseId: offeringId,
        teacherName: value['teacherName'] as String?,
        weekday: (value['weekday'] as num).toInt(),
        startSlot: (value['startSlot'] as num).toInt(),
        endSlot: (value['endSlot'] as num).toInt(),
        weeks: weeks,
        weekNumbers: parsedWeeks ?? const <int>{},
        weeksUnknown: parsedWeeks == null,
        location: location,
        locationUnknown: location?.trim().isNotEmpty != true,
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
