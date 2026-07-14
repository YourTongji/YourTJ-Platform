import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../app/app_services.dart';
import '../../../core/network/api_failure.dart';
import '../../../core/validation/identity_inputs.dart';

final Provider<NotificationsRepository> notificationsRepositoryProvider =
    Provider<NotificationsRepository>((Ref ref) {
      return NotificationsRepository(
        ref.watch(apiProvider).getNotificationsApi(),
      );
    });

class NotificationsRepository {
  NotificationsRepository(this._api);

  final NotificationsApi _api;

  Future<NotificationPage> notifications({
    required bool unreadOnly,
    String? cursor,
  }) => _required(
    _api.notificationsGet(
      unread: unreadOnly ? true : null,
      cursor: cursor,
      limit: 20,
    ),
    '通知列表响应不完整，请重试',
  );

  Future<GovernanceNoticePage> governanceNotices({
    required bool unreadOnly,
    String? cursor,
  }) => _required(
    _api.meGovernanceNoticesGet(unread: unreadOnly, cursor: cursor, limit: 20),
    '平台通知响应不完整，请重试',
  );

  Future<int> unreadCount() async {
    final NotificationUnreadCount count = await _required(
      _api.notificationsUnreadCountGet(),
      '通知未读数响应不完整，请重试',
    );
    return count.count;
  }

  Future<int> governanceUnreadCount() async {
    final MeGovernanceNoticesUnreadCountGet200Response count = await _required(
      _api.meGovernanceNoticesUnreadCountGet(),
      '平台通知未读数响应不完整，请重试',
    );
    return count.count;
  }

  Future<void> markNotificationsRead(List<String> ids) => _empty(
    _api.notificationsReadPost(
      notificationReadInput: NotificationReadInput(ids: ids),
    ),
  );

  Future<void> markAllNotificationsRead() => _empty(
    _api.notificationsReadPost(
      notificationReadInput: NotificationReadInput(all: true),
    ),
  );

  Future<void> markGovernanceNoticesRead(List<String> ids) => _empty(
    _api.meGovernanceNoticesReadPost(
      governanceNoticeReadInput: GovernanceNoticeReadInput(ids: ids),
    ),
  );

  Future<void> markAllGovernanceNoticesRead() => _empty(
    _api.meGovernanceNoticesReadPost(
      governanceNoticeReadInput: GovernanceNoticeReadInput(all: true),
    ),
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
        message: '通知响应无法解析，请更新应用或稍后重试',
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
        message: '无法确认通知操作结果，请刷新服务器状态',
      );
    }
  }
}

abstract final class NotificationTarget {
  static String? resolve(String? rawTarget) {
    if (rawTarget == null ||
        rawTarget.isEmpty ||
        rawTarget.contains(r'\') ||
        rawTarget.codeUnits.any((int code) => code < 32 || code == 127)) {
      return null;
    }
    final Uri? target = Uri.tryParse(rawTarget);
    if (target == null ||
        target.hasScheme ||
        target.hasAuthority ||
        !target.path.startsWith('/') ||
        target.path.startsWith('//')) {
      return null;
    }
    final List<String> segments = target.pathSegments;
    final bool allowed = switch (segments) {
      [] => true,
      ['forum'] => true,
      ['forum', 'threads', final String id] => _isSafeOpaqueId(id),
      ['messages'] => true,
      ['profile', final String handle] => IdentityInputs.isValidPublicHandle(
        handle,
      ),
      ['appeals'] => true,
      ['settings'] => true,
      ['settings', 'notifications'] => true,
      ['courses'] => true,
      ['courses', final String id] => id.isNotEmpty && !id.contains('/'),
      ['wallet'] => true,
      ['account'] => true,
      ['announcements'] => true,
      _ => false,
    };
    if (!allowed) {
      return null;
    }
    final Map<String, String> safeQuery = <String, String>{};
    if (segments case ['messages']) {
      final String? view = target.queryParameters['view'];
      if (<String>{'requests', 'sent', 'archived', 'deleted'}.contains(view)) {
        safeQuery['view'] = view!;
      }
      final String? conversation = target.queryParameters['conversation'];
      if (_isSafeOpaqueId(conversation)) {
        safeQuery['conversation'] = conversation!;
      }
    } else if (segments case ['appeals']) {
      final String? event = target.queryParameters['event'];
      final String? appeal = target.queryParameters['appeal'];
      if (_isSafeOpaqueId(event)) {
        safeQuery['event'] = event!;
      }
      if (_isSafeOpaqueId(appeal)) {
        safeQuery['appeal'] = appeal!;
      }
    }
    return Uri(
      path: target.path,
      queryParameters: safeQuery.isEmpty ? null : safeQuery,
    ).toString();
  }

  static bool _isSafeOpaqueId(String? value) =>
      value != null && RegExp(r'^[A-Za-z0-9_-]{1,128}$').hasMatch(value);
}
