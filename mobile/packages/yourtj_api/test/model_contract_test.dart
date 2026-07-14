import 'package:json_annotation/json_annotation.dart';
import 'package:test/test.dart';
import 'package:yourtj_api/yourtj_api.dart';

void main() {
  group('selection contract', () {
    test('decodes required nullable fields when they are explicitly null', () {
      final course = SelectionCourse.fromJson(<String, dynamic>{
        'id': 'course-1',
        'code': 'CS1001',
        'name': '程序设计',
        'credit': null,
        'natureId': null,
        'campusId': null,
        'teacherName': null,
        'teacherNames': <String>[],
      });

      expect(course.credit, isNull);
      expect(course.natureId, isNull);
      expect(course.campusId, isNull);
      expect(course.teacherName, isNull);
      expect(course.teacherNames, isEmpty);
      expect(course.toJson(), containsPair('credit', null));
    });

    test('rejects a missing required nullable field', () {
      final json = <String, dynamic>{
        'id': 'course-1',
        'code': 'CS1001',
        'name': '程序设计',
        'natureId': null,
        'campusId': null,
        'teacherName': null,
        'teacherNames': <String>[],
      };

      expect(
        () => SelectionCourse.fromJson(json),
        throwsA(isA<CheckedFromJsonException>()),
      );
    });

    test('decodes a time slot whose weeks value is null', () {
      final timeSlot = TimeSlot.fromJson(<String, dynamic>{
        'courseId': 'course-1',
        'teacherName': '张老师',
        'weekday': 2,
        'startSlot': 1,
        'endSlot': 2,
        'weeks': null,
        'location': '四平路校区',
      });

      expect(timeSlot.weeks, isNull);
      expect(timeSlot.toJson(), containsPair('weeks', null));
    });
  });

  group('latest update contract', () {
    test('decodes an RFC 3339 timestamp as a nullable DateTime', () {
      final latestUpdate = LatestUpdate.fromJson(<String, dynamic>{
        'updatedAt': '2026-07-14T09:10:11.123+08:00',
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
      });

      expect(latestUpdate.updatedAt, isNull);
      expect(latestUpdate.toJson(), containsPair('updatedAt', null));
    });

    test('rejects a non-RFC-3339 timestamp', () {
      expect(
        () => LatestUpdate.fromJson(<String, dynamic>{
          'updatedAt': '14/07/2026 09:10',
        }),
        throwsA(isA<CheckedFromJsonException>()),
      );
    });
  });

  test('uses the media delivery wire value for display1280', () {
    expect(MediaDeliveryVariant.display1280.toString(), 'display_1280');
  });
}
