import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../app/app_services.dart';
import '../../../core/network/api_failure.dart';

final Provider<AccountRepository> accountRepositoryProvider =
    Provider<AccountRepository>((Ref ref) {
      final YourtjApi api = ref.watch(apiProvider);
      return AccountRepository(
        api.getIdentityApi(),
        api.getAuthApi(),
        api.getNotificationsApi(),
        api.getMediaApi(),
      );
    });

class AccountRepository {
  AccountRepository(
    this._identityApi,
    this._authApi,
    this._notificationsApi,
    this._mediaApi,
  );

  final IdentityApi _identityApi;
  final AuthApi _authApi;
  final NotificationsApi _notificationsApi;
  final MediaApi _mediaApi;

  Future<OnboardingState> getOnboarding() => _required(
    _identityApi.meOnboardingGet(),
    incompleteMessage: '首次设置响应不完整，请重试',
  );

  Future<OnboardingState> completeOnboarding(OnboardingCompleteInput input) =>
      _required(
        _identityApi.meOnboardingPut(onboardingCompleteInput: input),
        incompleteMessage: '首次设置响应不完整，请刷新后确认',
      );

  Future<MyProfile> getMyProfile() => _required(
    _identityApi.meProfileGet(),
    incompleteMessage: '个人资料响应不完整，请重试',
  );

  Future<MyProfile> updateMyProfile(ProfileUpdateInput input) => _required(
    _identityApi.meProfilePut(profileUpdateInput: input),
    incompleteMessage: '个人资料保存响应不完整，请刷新后确认',
  );

  Future<Account> updateHandle(String handle) => _required(
    _identityApi.mePatch(mePatchRequest: MePatchRequest(handle: handle)),
    incompleteMessage: 'Handle 保存响应不完整，请刷新后确认',
  );

  Future<void> bindProfileAvatar(String assetId) => _void(
    _mediaApi.meProfileAvatarPut(
      profileAssetInput: ProfileAssetInput(assetId: assetId),
    ),
  );

  Future<void> removeProfileAvatar() =>
      _void(_mediaApi.meProfileAvatarDelete());

  Future<void> bindProfileBanner(String assetId) => _void(
    _mediaApi.meProfileBannerPut(
      profileAssetInput: ProfileAssetInput(assetId: assetId),
    ),
  );

  Future<void> removeProfileBanner() =>
      _void(_mediaApi.meProfileBannerDelete());

  Future<ProfilePrivacy> getPrivacy() => _required(
    _identityApi.mePrivacyGet(),
    incompleteMessage: '隐私设置响应不完整，请重试',
  );

  Future<ProfilePrivacy> updatePrivacy(ProfilePrivacyUpdateInput input) =>
      _required(
        _identityApi.mePrivacyPut(profilePrivacyUpdateInput: input),
        incompleteMessage: '隐私设置保存响应不完整，请刷新后确认',
      );

  Future<SessionPage> getSessions({String? cursor}) => _required(
    _identityApi.meSessionsGet(cursor: cursor, limit: 30),
    incompleteMessage: '设备会话响应不完整，请重试',
  );

  Future<void> revokeSession(String id) =>
      _void(_identityApi.meSessionsIdDelete(id: id));

  Future<void> revokeOtherSessions() =>
      _void(_identityApi.meSessionsRevokeOthersPost());

  Future<UserProfile> getUserProfile(String handle) => _required(
    _identityApi.usersHandleGet(handle: handle),
    incompleteMessage: '公开资料响应不完整，请重试',
  );

  Future<UserRelationship> getRelationship(String handle) => _required(
    _identityApi.usersHandleRelationshipGet(handle: handle),
    incompleteMessage: '社交关系响应不完整，请重试',
  );

  Future<UserSummaryPage> getFollowers(String handle, {String? cursor}) =>
      _required(
        _identityApi.usersHandleFollowersGet(
          handle: handle,
          cursor: cursor,
          limit: 30,
        ),
        incompleteMessage: '关注者列表响应不完整，请重试',
      );

  Future<UserSummaryPage> getFollowing(String handle, {String? cursor}) =>
      _required(
        _identityApi.usersHandleFollowingGet(
          handle: handle,
          cursor: cursor,
          limit: 30,
        ),
        incompleteMessage: '正在关注列表响应不完整，请重试',
      );

  Future<void> follow(String handle) =>
      _void(_identityApi.usersHandleFollowPut(handle: handle));

  Future<void> unfollow(String handle) =>
      _void(_identityApi.usersHandleFollowDelete(handle: handle));

  Future<void> removeFollower(String handle) =>
      _void(_identityApi.meFollowersHandleDelete(handle: handle));

  Future<void> mute(String handle) =>
      _void(_identityApi.usersHandleMutePut(handle: handle));

  Future<void> unmute(String handle) =>
      _void(_identityApi.usersHandleMuteDelete(handle: handle));

  Future<void> block(String handle) =>
      _void(_identityApi.usersHandleBlockPut(handle: handle));

  Future<void> unblock(String handle) =>
      _void(_identityApi.usersHandleBlockDelete(handle: handle));

  Future<NotificationPrefs> getNotificationPreferences() => _required(
    _notificationsApi.meNotificationPrefsGet(),
    incompleteMessage: '通知偏好响应不完整，请重试',
  );

  Future<NotificationPrefs> updateNotificationPreferences(
    NotificationPrefsInput input,
  ) => _required(
    _notificationsApi.meNotificationPrefsPut(notificationPrefsInput: input),
    incompleteMessage: '通知偏好保存响应不完整，请刷新后确认',
  );

  Future<RecentAuthStatus> getRecentAuthStatus() => _required(
    _authApi.authRecentAuthGet(),
    incompleteMessage: '最近认证状态响应不完整，请重新登录',
  );

  Future<void> requestRecentAuthEmailCode() =>
      _void(_authApi.authRecentAuthEmailRequestCodePost());

  Future<RecentAuthStatus> verifyRecentAuth(RecentAuthVerifyInput input) =>
      _required(
        _authApi.authRecentAuthVerifyPost(recentAuthVerifyInput: input),
        incompleteMessage: '最近认证响应不完整，请重试',
      );

  Future<List<DataExportJob>> getDataExports() => _required(
    _identityApi.meDataExportsGet(),
    incompleteMessage: '数据导出列表响应不完整，请重试',
  );

  Future<DataExportJob> createDataExport(String idempotencyKey) => _required(
    _identityApi.meDataExportsPost(idempotencyKey: idempotencyKey),
    incompleteMessage: '数据导出响应不完整，请刷新列表确认',
  );

  Future<DataExportDownloadGrant> createDataExportGrant(String id) => _required(
    _identityApi.meDataExportsIdDownloadGrantPost(id: id),
    incompleteMessage: '下载授权响应不完整，请重试',
  );

  Future<AccountDataExport> downloadDataExport({
    required String id,
    required String grant,
  }) => _required(
    _identityApi.meDataExportsIdDownloadGet(id: id, xExportToken: grant),
    incompleteMessage: '导出文件响应不完整，请重新获取授权',
  );

  Future<AccountLifecycle> getLifecycle() => _required(
    _identityApi.meLifecycleGet(),
    incompleteMessage: '账号状态响应不完整，请重试',
  );

  Future<AccountLifecycleMutation> deactivateAccount(String idempotencyKey) =>
      _required(
        _identityApi.meLifecycleDeactivatePost(
          idempotencyKey: idempotencyKey,
          deactivateAccountInput: DeactivateAccountInput(
            confirmation: DeactivateAccountInputConfirmationEnum.DEACTIVATE,
          ),
        ),
        incompleteMessage: '停用响应不完整，请通过恢复入口确认账号状态',
      );

  Future<AccountLifecycleMutation> requestAccountDeletion(
    String idempotencyKey,
  ) => _required(
    _identityApi.meLifecycleDeletePost(
      idempotencyKey: idempotencyKey,
      deleteAccountInput: DeleteAccountInput(
        confirmation: DeleteAccountInputConfirmationEnum.DELETE,
      ),
    ),
    incompleteMessage: '删除请求响应不完整，请通过恢复入口确认账号状态',
  );

  Future<RecoveryCredential> proveRecoveryWithPassword({
    required String email,
    required String password,
  }) => _required(
    _authApi.authRecoveryPasswordPost(
      authAppealPasswordPostRequest: AuthAppealPasswordPostRequest(
        email: email.trim(),
        password: password,
      ),
    ),
    incompleteMessage: '恢复凭据响应不完整，请重试',
  );

  Future<RecoveryCredential> proveRecoveryWithEmailCode({
    required String email,
    required String code,
  }) => _required(
    _authApi.authRecoveryEmailVerifyPost(
      authRecoveryEmailVerifyPostRequest: AuthRecoveryEmailVerifyPostRequest(
        email: email.trim(),
        code: code.trim(),
      ),
    ),
    incompleteMessage: '恢复凭据响应不完整，请重试',
  );

  Future<AccountLifecycle> inspectRecovery(String recoveryToken) => _required(
    _authApi.authRecoveryGet(xRecoveryToken: recoveryToken),
    incompleteMessage: '可恢复状态响应不完整，请重新验证',
  );

  Future<AccountLifecycle> recoverAccount(String recoveryToken) => _required(
    _authApi.authRecoveryPost(xRecoveryToken: recoveryToken),
    incompleteMessage: '恢复响应不完整，请重新验证账号状态',
  );

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
        message: '响应无法解析，请更新应用或稍后重试',
      );
    }
  }

  Future<void> _void(Future<Response<void>> request) async {
    try {
      await request;
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    } on Object {
      throw const ApiFailure(
        kind: ApiFailureKind.unexpected,
        message: '无法确认操作结果，请刷新服务器状态',
      );
    }
  }
}
