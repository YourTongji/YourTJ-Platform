import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../app/app_services.dart';
import '../../../core/network/api_failure.dart';

class HomeGrowth {
  const HomeGrowth({
    required this.activity,
    required this.checkIn,
    required this.trust,
  });

  final ActivityCalendar activity;
  final CheckInStatus checkIn;
  final TrustProgress trust;
}

class HomeRepository {
  HomeRepository(this._activityApi, this._platformApi);

  final ActivityApi _activityApi;
  final PlatformApi _platformApi;

  Future<HomeGrowth> growth() async {
    try {
      final List<Response<dynamic>> responses = await Future.wait(
        <Future<Response<dynamic>>>[
          // Omitting date parameters avoids the generator's DateTime query
          // serialization bug and uses the server's bounded default range.
          _activityApi.meActivityGet(),
          _activityApi.meCheckInGet(),
          _activityApi.meTrustProgressGet(),
        ],
      );
      final ActivityCalendar? activity = responses[0].data as ActivityCalendar?;
      final CheckInStatus? checkIn = responses[1].data as CheckInStatus?;
      final TrustProgress? trust = responses[2].data as TrustProgress?;
      if (activity == null || checkIn == null || trust == null) {
        throw const ApiFailure(
          kind: ApiFailureKind.unexpected,
          message: '成长数据响应不完整',
        );
      }
      return HomeGrowth(activity: activity, checkIn: checkIn, trust: trust);
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    }
  }

  Future<CheckInStatus> checkIn() async {
    try {
      final Response<CheckInStatus> response = await _activityApi
          .meCheckInPost();
      final CheckInStatus? status = response.data;
      if (status == null) {
        throw const ApiFailure(
          kind: ApiFailureKind.unexpected,
          message: '签到响应不完整',
        );
      }
      return status;
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    }
  }

  Future<List<Announcement>> announcements() async {
    try {
      final Response<List<Announcement>> response = await _platformApi
          .announcementsGet();
      return response.data ?? <Announcement>[];
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    }
  }

  Future<List<Promotion>> promotions() async {
    try {
      final Response<List<Promotion>> response = await _platformApi
          .promotionsGet();
      return response.data ?? <Promotion>[];
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    }
  }

  Future<void> recordPromotionEvent({
    required Promotion promotion,
    required PromotionEventInputEventTypeEnum eventType,
  }) async {
    final String? trackingToken = promotion.trackingToken;
    if (trackingToken == null || trackingToken.isEmpty) {
      return;
    }
    try {
      await _platformApi.promotionsIdEventsPost(
        id: promotion.id,
        promotionEventInput: PromotionEventInput(
          eventType: eventType,
          trackingToken: trackingToken,
        ),
      );
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    }
  }
}

final Provider<HomeRepository> homeRepositoryProvider =
    Provider<HomeRepository>((Ref ref) {
      final YourtjApi api = ref.watch(apiProvider);
      return HomeRepository(api.getActivityApi(), api.getPlatformApi());
    });
