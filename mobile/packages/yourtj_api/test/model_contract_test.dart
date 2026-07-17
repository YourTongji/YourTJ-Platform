import 'package:json_annotation/json_annotation.dart';
import 'package:test/test.dart';
import 'package:yourtj_api/yourtj_api.dart';

void main() {
  group('selection contract', () {
    test('decodes required nullable fields when they are explicitly null', () {
      final course = SelectionOffering.fromJson(<String, dynamic>{
        'id': 'offering-1',
        'offeringId': 'offering-1',
        'code': 'CS1001',
        'teachingClassCode': null,
        'name': '程序设计',
        'credit': null,
        'natureId': null,
        'calendarId': '122',
        'campusId': null,
        'facultyName': null,
        'teachingLanguage': null,
        'teacherName': null,
        'teacherNames': <String>[],
        'startWeek': null,
        'endWeek': null,
        'weeksUnknown': true,
        'scheduleUnknown': true,
        'status': 'unknown',
        'catalogueCourseId': null,
        'reviewCount': 0,
        'reviewAvg': null,
        'reviewScope': 'none',
      });

      expect(course.offeringId, 'offering-1');
      expect(course.calendarId, '122');
      expect(course.credit, isNull);
      expect(course.natureId, isNull);
      expect(course.campusId, isNull);
      expect(course.teacherName, isNull);
      expect(course.teacherNames, isEmpty);
      expect(course.toJson(), containsPair('credit', null));
    });

    test('rejects a missing required nullable field', () {
      final json = <String, dynamic>{
        'id': 'offering-1',
        'offeringId': 'offering-1',
        'code': 'CS1001',
        'teachingClassCode': null,
        'name': '程序设计',
        'natureId': null,
        'calendarId': '122',
        'campusId': null,
        'facultyName': null,
        'teachingLanguage': null,
        'teacherName': null,
        'teacherNames': <String>[],
        'startWeek': null,
        'endWeek': null,
        'weeksUnknown': true,
        'scheduleUnknown': true,
        'status': 'unknown',
        'catalogueCourseId': null,
        'reviewCount': 0,
        'reviewAvg': null,
        'reviewScope': 'none',
      };

      expect(
        () => SelectionOffering.fromJson(json),
        throwsA(isA<CheckedFromJsonException>()),
      );
    });

    test('decodes a time slot whose weeks value is null', () {
      final timeSlot = TimeSlot.fromJson(<String, dynamic>{
        'offeringId': 'offering-1',
        'courseId': 'offering-1',
        'teacherName': '张老师',
        'weekday': 2,
        'startSlot': 1,
        'endSlot': 2,
        'weeks': null,
        'weekNumbers': <int>[],
        'weeksUnknown': true,
        'location': '四平路校区',
        'locationUnknown': false,
      });

      expect(timeSlot.weeks, isNull);
      expect(timeSlot.weeksUnknown, isTrue);
      expect(timeSlot.toJson(), containsPair('weeks', null));
    });
  });

  group('latest update contract', () {
    test('decodes an RFC 3339 timestamp as a nullable DateTime', () {
      final latestUpdate = LatestUpdate.fromJson(<String, dynamic>{
        'updatedAt': '2026-07-14T09:10:11.123+08:00',
        'importedAt': '2026-07-15T01:00:00Z',
        'stale': false,
        'staleAfterHours': 168,
      });

      expect(
        latestUpdate.updatedAt,
        DateTime.parse('2026-07-14T01:10:11.123Z'),
      );
      expect(latestUpdate.toJson()['updatedAt'], '2026-07-14T01:10:11.123Z');
    });

    test('preserves an explicit null timestamp', () {
      final latestUpdate = LatestUpdate.fromJson(<String, dynamic>{
        'updatedAt': null,
        'importedAt': null,
        'stale': true,
        'staleAfterHours': 168,
      });

      expect(latestUpdate.updatedAt, isNull);
      expect(latestUpdate.toJson(), containsPair('updatedAt', null));
    });

    test('rejects a non-RFC-3339 timestamp', () {
      expect(
        () => LatestUpdate.fromJson(<String, dynamic>{
          'updatedAt': '14/07/2026 09:10',
          'importedAt': null,
          'stale': true,
          'staleAfterHours': 168,
        }),
        throwsA(isA<CheckedFromJsonException>()),
      );
    });
  });

  test('uses the media delivery wire value for display1280', () {
    expect(MediaDeliveryVariant.display1280.toString(), 'display_1280');
  });
}
