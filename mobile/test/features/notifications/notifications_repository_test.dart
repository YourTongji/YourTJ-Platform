import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/features/notifications/data/notifications_repository.dart';

import '../auth/support/session_test_support.dart';

void main() {
  test(
    'mark all uses the explicit all flag instead of the legacy empty body',
    () async {
      late final RecordingAdapter adapter;
      final Dio dio = Dio(BaseOptions(baseUrl: 'https://api.yourtj.de/api/v2'));
      adapter = RecordingAdapter((RequestOptions options) {
        expect(options.path, '/notifications/read');
        return jsonResponse(null, statusCode: 204);
      });
      dio.httpClientAdapter = adapter;
      final NotificationsRepository repository = NotificationsRepository(
        NotificationsApi(dio),
      );

      await repository.markAllNotificationsRead();

      expect(requestJson(adapter.requests.single), <String, Object?>{
        'all': true,
      });
    },
  );

  test(
    'notification targets retain only allowlisted routes and query keys',
    () {
      expect(
        NotificationTarget.resolve(
          '/messages?view=requests&conversation=42&token=secret',
        ),
        '/messages?view=requests&conversation=42',
      );
      expect(
        NotificationTarget.resolve('/appeals?event=9&staffEvidence=hidden'),
        '/appeals?event=9',
      );
      expect(
        NotificationTarget.resolve('/forum/threads/thread-1?unsafe=1'),
        '/forum/threads/thread-1',
      );
      expect(
        NotificationTarget.resolve('/profile/student.name-test_2026'),
        '/profile/student.name-test_2026',
      );
      expect(NotificationTarget.resolve('/profile/Student'), isNull);
      expect(NotificationTarget.resolve('//attacker.example/path'), isNull);
      expect(NotificationTarget.resolve('/\\attacker.example/path'), isNull);
      expect(
        NotificationTarget.resolve('/api/v2/me?accessToken=secret'),
        isNull,
      );
      expect(NotificationTarget.resolve('https://yourtj.de/forum'), isNull);
    },
  );
}
