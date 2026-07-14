import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/features/appeals/data/appeals_repository.dart';

import '../auth/support/session_test_support.dart';

void main() {
  test(
    'restricted appeal bearer is scoped to the explicit owner route',
    () async {
      late final RecordingAdapter adapter;
      final Dio dio = Dio(BaseOptions(baseUrl: 'https://api.yourtj.de/api/v2'));
      adapter = RecordingAdapter((RequestOptions options) {
        expect(options.path, '/me/appeals');
        return jsonResponse(<String, Object?>{
          'items': <Object?>[],
          'nextCursor': null,
          'hasMore': false,
        });
      });
      dio.httpClientAdapter = adapter;
      final AppealsRepository repository = AppealsRepository(
        AuthApi(dio),
        AdminApi(dio),
        NotificationsApi(dio),
      );

      await repository.appeals(appealToken: 'purpose-bound-token');

      final RecordedRequest request = adapter.requests.single;
      expect(request.headers['Authorization'], 'Bearer purpose-bound-token');
      expect(request.extra['secure'], isEmpty);
      expect(request.extra['yourtj.disableSessionRetry'], isTrue);
    },
  );

  test(
    'appeal submission preserves idempotency and generated contract fields',
    () async {
      late final RecordingAdapter adapter;
      final Dio dio = Dio(BaseOptions(baseUrl: 'https://api.yourtj.de/api/v2'));
      adapter = RecordingAdapter((RequestOptions options) {
        return jsonResponse(_appealJson(), statusCode: 201);
      });
      dio.httpClientAdapter = adapter;
      final AppealsRepository repository = AppealsRepository(
        AuthApi(dio),
        AdminApi(dio),
        NotificationsApi(dio),
      );

      final Appeal result = await repository.submit(
        governanceEventId: ' 44 ',
        reason: ' 请重新核验 ',
        idempotencyKey: 'appeal-key-1',
        appealToken: 'purpose-bound-token',
      );

      final RecordedRequest request = adapter.requests.single;
      expect(result.governanceEventId, '44');
      expect(request.headers['Idempotency-Key'], 'appeal-key-1');
      expect(requestJson(request), <String, Object?>{
        'governanceEventId': '44',
        'reason': '请重新核验',
      });
    },
  );
}

Map<String, Object?> _appealJson() => <String, Object?>{
  'id': 'appeal-1',
  'governanceEventId': '44',
  'originalAction': 'forum.thread.hide',
  'originalReason': '违反社区规则',
  'targetKind': 'forum_thread',
  'targetId': '7',
  'dispositionKind': 'hide',
  'status': 'submitted',
  'submissionReason': '请重新核验',
  'submittedAt': 100,
  'appealableUntil': 200,
  'reviewStartedAt': null,
  'decisionReason': null,
  'amendment': null,
  'decidedAt': null,
  'version': 1,
  'history': <Object?>[
    <String, Object?>{
      'id': 'history-1',
      'fromStatus': null,
      'toStatus': 'submitted',
      'reason': '用户提交申诉',
      'metadata': null,
      'createdAt': 100,
    },
  ],
};
