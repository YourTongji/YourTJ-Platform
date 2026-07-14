import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../app/app_services.dart';
import '../../../core/network/api_failure.dart';

final Provider<AppealsRepository> appealsRepositoryProvider =
    Provider<AppealsRepository>((Ref ref) {
      final YourtjApi api = ref.watch(apiProvider);
      return AppealsRepository(
        api.getAuthApi(),
        api.getAdminApi(),
        api.getNotificationsApi(),
      );
    });

class AppealsRepository {
  AppealsRepository(this._authApi, this._adminApi, this._notificationsApi);

  final AuthApi _authApi;
  final AdminApi _adminApi;
  final NotificationsApi _notificationsApi;

  Future<AppealAccessToken> passwordAccess({
    required String email,
    required String password,
  }) => _required(
    _authApi.authAppealPasswordPost(
      authAppealPasswordPostRequest: AuthAppealPasswordPostRequest(
        email: email.trim().toLowerCase(),
        password: password,
      ),
    ),
    '申诉访问凭据响应不完整，请重试',
  );

  Future<AppealAccessToken> emailCodeAccess({
    required String email,
    required String code,
  }) => _required(
    _authApi.authAppealEmailVerifyPost(
      appealEmailVerification: AppealEmailVerification(
        email: email.trim().toLowerCase(),
        code: code.trim(),
      ),
    ),
    '申诉访问凭据响应不完整，请重试',
  );

  Future<AppealPage> appeals({String? cursor, String? appealToken}) =>
      _required(
        _adminApi.meAppealsGet(
          cursor: cursor,
          limit: 20,
          headers: _headers(appealToken),
          extra: _extra(appealToken),
        ),
        '申诉列表响应不完整，请重试',
      );

  Future<GovernanceNoticePage> governanceNotices({
    String? cursor,
    String? appealToken,
  }) => _required(
    _notificationsApi.meGovernanceNoticesGet(
      cursor: cursor,
      limit: 20,
      headers: _headers(appealToken),
      extra: _extra(appealToken),
    ),
    '治理通知响应不完整，请重试',
  );

  Future<Appeal> submit({
    required String governanceEventId,
    required String reason,
    required String idempotencyKey,
    String? appealToken,
  }) => _required(
    _adminApi.meAppealsPost(
      idempotencyKey: idempotencyKey,
      submitAppealInput: SubmitAppealInput(
        governanceEventId: governanceEventId.trim(),
        reason: reason.trim(),
      ),
      headers: _headers(appealToken),
      extra: _extra(appealToken),
    ),
    '申诉提交响应不完整，请刷新申诉列表确认',
  );

  Future<Appeal> withdraw({
    required Appeal appeal,
    required String reason,
    String? appealToken,
  }) => _required(
    _adminApi.meAppealsIdWithdrawPost(
      id: appeal.id,
      appealTransitionInput: AppealTransitionInput(
        expectedVersion: appeal.version,
        reason: reason.trim(),
      ),
      headers: _headers(appealToken),
      extra: _extra(appealToken),
    ),
    '申诉撤回响应不完整，请刷新申诉列表确认',
  );

  Future<void> markGovernanceNoticeRead({
    required String id,
    String? appealToken,
  }) => _empty(
    _notificationsApi.meGovernanceNoticesReadPost(
      governanceNoticeReadInput: GovernanceNoticeReadInput(ids: <String>[id]),
      headers: _headers(appealToken),
      extra: _extra(appealToken),
    ),
  );

  static Map<String, dynamic>? _headers(String? appealToken) =>
      appealToken == null
      ? null
      : <String, dynamic>{'Authorization': 'Bearer $appealToken'};

  static Map<String, dynamic>? _extra(String? appealToken) =>
      appealToken == null
      ? null
      : <String, dynamic>{
          'secure': <Map<String, String>>[],
          'yourtj.disableSessionRetry': true,
        };

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
        message: '申诉响应无法解析，请更新应用或稍后重试',
      );
    }
  }

  Future<void> _empty(Future<Response<void>> request) async {
    try {
      await request;
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    } on Object {
      throw const ApiFailure(
        kind: ApiFailureKind.unexpected,
        message: '无法确认申诉通知操作结果，请刷新服务器状态',
      );
    }
  }
}
