import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/features/home/data/home_repository.dart';

import '../auth/support/session_test_support.dart';

void main() {
  test(
    'records promotion events with the short-lived tracking token',
    () async {
      late final RecordingAdapter adapter;
      final Dio dio = Dio(BaseOptions(baseUrl: 'https://api.yourtj.de/api/v2'));
      adapter = RecordingAdapter((RequestOptions options) {
        expect(options.method, 'POST');
        expect(options.path, '/promotions/promotion-1/events');
        return jsonResponse(null, statusCode: 204);
      });
      dio.httpClientAdapter = adapter;
      final HomeRepository repository = HomeRepository(
        ActivityApi(dio),
        PlatformApi(dio),
      );

      await repository.recordPromotionEvent(
        promotion: _promotion(trackingToken: 'delivery-token'),
        eventType: PromotionEventInputEventTypeEnum.click,
      );

      expect(requestJson(adapter.requests.single), <String, Object?>{
        'eventType': 'click',
        'trackingToken': 'delivery-token',
      });
    },
  );

  test(
    'does not invent an event when delivery has no tracking token',
    () async {
      final Dio dio = Dio(BaseOptions(baseUrl: 'https://api.yourtj.de/api/v2'));
      final RecordingAdapter adapter = RecordingAdapter(
        (RequestOptions options) => throw StateError('unexpected request'),
      );
      dio.httpClientAdapter = adapter;
      final HomeRepository repository = HomeRepository(
        ActivityApi(dio),
        PlatformApi(dio),
      );

      await repository.recordPromotionEvent(
        promotion: _promotion(),
        eventType: PromotionEventInputEventTypeEnum.impression,
      );

      expect(adapter.requests, isEmpty);
    },
  );
}

Promotion _promotion({String? trackingToken}) => Promotion(
  id: 'promotion-1',
  placement: PromotionPlacementEnum.homeLeftPrimary,
  title: '站内推广',
  targetUrl: '/forum/threads/42',
  assetDelivery: null,
  status: PromotionStatusEnum.published,
  effectiveState: PromotionEffectiveStateEnum.active,
  priority: 10,
  audience: PromotionAudienceEnum.all,
  version: 1,
  createdAt: 1,
  updatedAt: 2,
  trackingToken: trackingToken,
  metrics: null,
);
