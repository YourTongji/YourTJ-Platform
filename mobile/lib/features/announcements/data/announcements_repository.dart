import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../app/app_services.dart';
import '../../../core/network/api_failure.dart';

final Provider<AnnouncementsRepository> announcementsRepositoryProvider =
    Provider<AnnouncementsRepository>((Ref ref) {
      return AnnouncementsRepository(ref.watch(apiProvider).getPlatformApi());
    });

class AnnouncementsRepository {
  AnnouncementsRepository(this._api);

  final PlatformApi _api;

  Future<List<Announcement>> active() =>
      _required(_api.announcementsGet(), '公告列表响应不完整，请稍后重试');

  Future<List<Announcement>> unread() =>
      _required(_api.announcementsUnreadGet(), '未查看公告响应不完整，请稍后重试');

  Future<AnnouncementReceipt> record({
    required Announcement announcement,
    required AnnouncementReceiptInputActionEnum action,
  }) => _required(
    _api.announcementsIdReceiptPost(
      id: announcement.id,
      announcementReceiptInput: AnnouncementReceiptInput(
        revision: announcement.revision,
        action: action,
      ),
    ),
    '公告回执响应不完整，请刷新确认',
  );

  Future<T> _required<T>(Future<Response<T>> request, String message) async {
    try {
      final T? value = (await request).data;
      if (value == null) {
        throw ApiFailure(kind: ApiFailureKind.unexpected, message: message);
      }
      return value;
    } on ApiFailure {
      rethrow;
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    } on Object {
      throw const ApiFailure(
        kind: ApiFailureKind.unexpected,
        message: '公告响应无法解析，请更新应用或稍后重试',
      );
    }
  }
}
