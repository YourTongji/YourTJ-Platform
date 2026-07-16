import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/features/schedule/data/selection_repository.dart';

import '../auth/support/session_test_support.dart';

void main() {
  test(
    'forwards calendar and free-time filters to the canonical offering API',
    () async {
      late final RecordingAdapter adapter;
      final Dio dio = Dio(BaseOptions(baseUrl: 'https://api.yourtj.de/api/v2'));
      adapter = RecordingAdapter((RequestOptions options) {
        expect(options.path, '/selection/offerings');
        return jsonResponse(<String, Object?>{
          'items': const <Object>[],
          'nextCursor': null,
          'hasMore': false,
        });
      });
      dio.httpClientAdapter = adapter;
      final SelectionRepository repository = GeneratedSelectionRepository(
        SelectionApi(dio),
      );

      final SelectionOfferingPage page = await repository.offerings(
        calendarId: '2026-spring',
        majorId: 'software',
        grade: '2026',
        weekday: 5,
        startSlot: 14,
        endSlot: 20,
        week: 16,
        includeUnknownSchedule: false,
        cursor: 'next-page',
        limit: 25,
      );

      expect(page.items, isEmpty);
      expect(adapter.requests.single.uri.queryParameters, <String, String>{
        'calendarId': '2026-spring',
        'majorId': 'software',
        'grade': '2026',
        'weekday': '5',
        'startSlot': '14',
        'endSlot': '20',
        'week': '16',
        'includeUnknownSchedule': 'false',
        'cursor': 'next-page',
        'limit': '25',
      });
    },
  );

  test('scopes majors by both calendar and grade', () async {
    late final RecordingAdapter adapter;
    final Dio dio = Dio(BaseOptions(baseUrl: 'https://api.yourtj.de/api/v2'));
    adapter = RecordingAdapter((RequestOptions options) {
      expect(options.path, '/selection/majors');
      return jsonResponse(const <Object>[]);
    });
    dio.httpClientAdapter = adapter;
    final SelectionRepository repository = GeneratedSelectionRepository(
      SelectionApi(dio),
    );

    final List<Major> majors = await repository.majors(
      calendarId: '2026-autumn',
      grade: '2025',
    );

    expect(majors, isEmpty);
    expect(adapter.requests.single.uri.queryParameters, <String, String>{
      'calendarId': '2026-autumn',
      'grade': '2025',
    });
  });

  test('scopes course natures by calendar', () async {
    late final RecordingAdapter adapter;
    final Dio dio = Dio(BaseOptions(baseUrl: 'https://api.yourtj.de/api/v2'));
    adapter = RecordingAdapter((RequestOptions options) {
      expect(options.path, '/selection/course-natures');
      return jsonResponse(const <Object>[]);
    });
    dio.httpClientAdapter = adapter;
    final SelectionRepository repository = GeneratedSelectionRepository(
      SelectionApi(dio),
    );

    final List<CourseNature> natures = await repository.natures('2026-autumn');

    expect(natures, isEmpty);
    expect(adapter.requests.single.uri.queryParameters, <String, String>{
      'calendarId': '2026-autumn',
    });
  });
}
