import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../app/app_services.dart';
import '../../../core/network/api_failure.dart';

final Provider<ProfileActivityRepository> profileActivityRepositoryProvider =
    Provider<ProfileActivityRepository>((Ref ref) {
      final YourtjApi api = ref.watch(apiProvider);
      return ProfileActivityRepository(api.getIdentityApi(), api.getForumApi());
    });

class ProfileActivityPage<T> {
  const ProfileActivityPage({
    required this.items,
    required this.nextCursor,
    required this.hasMore,
  });

  final List<T> items;
  final String? nextCursor;
  final bool hasMore;
}

abstract interface class ProfileActivitySource {
  Future<ProfileActivityPage<UserThread>> threads({
    required String handle,
    String? cursor,
  });

  Future<ProfileActivityPage<UserComment>> comments({
    required String handle,
    String? cursor,
  });

  Future<ProfileActivityPage<ProfileContent>> media({
    required String handle,
    String? cursor,
  });

  Future<ProfileActivityPage<ProfileContent>> likes({
    required String handle,
    String? cursor,
  });
}

class ProfileActivityRepository implements ProfileActivitySource {
  ProfileActivityRepository(this._identityApi, this._forumApi);

  final IdentityApi _identityApi;
  final ForumApi _forumApi;

  @override
  Future<ProfileActivityPage<UserThread>> threads({
    required String handle,
    String? cursor,
  }) async {
    final UserThreadPage page = await _required(
      _identityApi.usersHandleThreadsGet(
        handle: handle,
        cursor: cursor,
        limit: 20,
      ),
      incompleteMessage: '公开主题响应不完整，请重试',
    );
    return _page(
      items: page.items,
      nextCursor: page.nextCursor,
      hasMore: page.hasMore,
    );
  }

  @override
  Future<ProfileActivityPage<UserComment>> comments({
    required String handle,
    String? cursor,
  }) async {
    final UserCommentPage page = await _required(
      _identityApi.usersHandleCommentsGet(
        handle: handle,
        cursor: cursor,
        limit: 20,
      ),
      incompleteMessage: '公开回复响应不完整，请重试',
    );
    return _page(
      items: page.items,
      nextCursor: page.nextCursor,
      hasMore: page.hasMore,
    );
  }

  @override
  Future<ProfileActivityPage<ProfileContent>> media({
    required String handle,
    String? cursor,
  }) async {
    final ProfileContentPage page = await _required(
      _forumApi.usersHandleMediaGet(handle: handle, cursor: cursor, limit: 20),
      incompleteMessage: '公开媒体响应不完整，请重试',
    );
    return _page(
      items: page.items,
      nextCursor: page.nextCursor,
      hasMore: page.hasMore,
    );
  }

  @override
  Future<ProfileActivityPage<ProfileContent>> likes({
    required String handle,
    String? cursor,
  }) async {
    final ProfileContentPage page = await _required(
      _forumApi.usersHandleLikesGet(handle: handle, cursor: cursor, limit: 20),
      incompleteMessage: '公开喜欢响应不完整，请重试',
    );
    return _page(
      items: page.items,
      nextCursor: page.nextCursor,
      hasMore: page.hasMore,
    );
  }

  ProfileActivityPage<T> _page<T>({
    required List<T> items,
    required String? nextCursor,
    required bool hasMore,
  }) {
    if (hasMore && (nextCursor == null || nextCursor.trim().isEmpty)) {
      throw const ApiFailure(
        kind: ApiFailureKind.unexpected,
        message: '分页响应缺少继续游标，请刷新后重试',
      );
    }
    return ProfileActivityPage<T>(
      items: List<T>.unmodifiable(items),
      nextCursor: nextCursor,
      hasMore: hasMore,
    );
  }

  Future<T> _required<T>(
    Future<Response<T>> request, {
    required String incompleteMessage,
  }) async {
    try {
      final T? data = (await request).data;
      if (data == null) {
        throw ApiFailure(
          kind: ApiFailureKind.unexpected,
          message: incompleteMessage,
        );
      }
      return data;
    } on ApiFailure {
      rethrow;
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    } on Object {
      throw const ApiFailure(
        kind: ApiFailureKind.unexpected,
        message: '公开动态响应无法解析，请更新应用或稍后重试',
      );
    }
  }
}
