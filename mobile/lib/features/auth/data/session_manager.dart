import 'dart:async';

import 'package:dio/dio.dart';
import 'package:flutter/foundation.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../core/network/api_failure.dart';
import '../../../core/storage/installation_store.dart';
import '../domain/session_state.dart';
import 'secure_session_storage.dart';

class SessionManager {
  SessionManager(this._publicAuthApi, this._storage, this._installationStore);

  final AuthApi _publicAuthApi;
  final SecureSessionStorage _storage;
  final InstallationStore _installationStore;
  final StreamController<SessionState> _changes =
      StreamController<SessionState>.broadcast(sync: true);
  final ValueNotifier<int> _routerRevision = ValueNotifier<int>(0);

  AuthApi? _authenticatedAuthApi;
  SessionState _state = const SessionState.restoring(generation: 0);
  String? _accessToken;
  String? _refreshToken;
  String? _activeAccountId;
  Future<String?>? _refreshFuture;
  Future<void> _storageTail = Future<void>.value();
  int _credentialOperation = 0;
  bool _isDisposed = false;

  SessionState get state => _state;
  Stream<SessionState> get changes => _changes.stream;
  Listenable get routerRefresh => _routerRevision;
  String? get accessToken => _accessToken;
  int get generation => _state.generation;

  void attachAuthenticatedApi(AuthApi authApi) {
    _authenticatedAuthApi = authApi;
  }

  Future<void> initialize() async {
    _emit(SessionState.restoring(generation: generation));
    try {
      final StoredSessionCredential? storedSession = await _storage
          .readSession();
      if (storedSession == null) {
        _emit(SessionState.anonymous(generation: generation));
        return;
      }
      _activeAccountId = storedSession.accountId;
      _refreshToken = storedSession.refreshToken;
      await _refresh(coldStart: true);
    } on ApiFailure {
      rethrow;
    } on Object {
      _clearMemory();
      _emit(
        SessionState(
          phase: SessionPhase.secureStorageUnavailable,
          generation: generation + 1,
          message: '无法使用系统安全存储。为保护账号，本机登录已停用。',
        ),
      );
    }
  }

  Future<void> passwordLogin({
    required String email,
    required String password,
  }) async {
    final int operation = ++_credentialOperation;
    try {
      final String installationId = await _installationStore.readOrCreateId();
      final Response<AuthTokens> response = await _publicAuthApi
          .authPasswordLoginPost(
            authPasswordLoginPostRequest: AuthPasswordLoginPostRequest(
              email: email.trim(),
              password: password,
              clientInstallationId: installationId,
            ),
          );
      final AuthTokens? tokens = response.data;
      if (tokens == null) {
        throw const ApiFailure(
          kind: ApiFailureKind.unexpected,
          message: '登录响应不完整，请稍后重试',
        );
      }
      await _acceptTokens(tokens, beginsNewSession: true, operation: operation);
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    }
  }

  Future<void> verifyEmailCode({
    required String email,
    required String code,
    required EmailCodePurpose purpose,
    String? handle,
    String? password,
  }) async {
    final int operation = ++_credentialOperation;
    try {
      final String installationId = await _installationStore.readOrCreateId();
      final Response<AuthTokens> response = await _publicAuthApi
          .authEmailVerifyPost(
            emailCodeVerification: EmailCodeVerification(
              email: email.trim(),
              code: code.trim(),
              purpose: purpose,
              handle: handle?.trim(),
              password: password,
              clientInstallationId: installationId,
            ),
          );
      final AuthTokens? tokens = response.data;
      if (tokens == null) {
        throw const ApiFailure(
          kind: ApiFailureKind.unexpected,
          message: '验证码登录响应不完整，请稍后重试',
        );
      }
      await _acceptTokens(tokens, beginsNewSession: true, operation: operation);
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    }
  }

  Future<void> requestEmailCode({
    required String email,
    required String captchaToken,
    required EmailCodePurpose purpose,
  }) async {
    try {
      await _publicAuthApi.authEmailRequestCodePost(
        emailCodeRequest: EmailCodeRequest(
          email: email.trim(),
          captchaToken: captchaToken,
          purpose: purpose,
        ),
      );
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    }
  }

  Future<void> requestPasswordReset({
    required String email,
    required String captchaToken,
  }) async {
    try {
      await _publicAuthApi.authPasswordForgotPost(
        authPasswordForgotPostRequest: AuthPasswordForgotPostRequest(
          email: email.trim(),
          captchaToken: captchaToken,
        ),
      );
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    }
  }

  Future<void> resetPassword({
    required String email,
    required String code,
    required String newPassword,
  }) async {
    final int operation = ++_credentialOperation;
    try {
      final String installationId = await _installationStore.readOrCreateId();
      final Response<AuthTokens> response = await _publicAuthApi
          .authPasswordResetPost(
            passwordResetInput: PasswordResetInput(
              email: email.trim(),
              code: code.trim(),
              newPassword: newPassword,
              clientInstallationId: installationId,
            ),
          );
      final AuthTokens? tokens = response.data;
      if (tokens == null) {
        throw const ApiFailure(
          kind: ApiFailureKind.unexpected,
          message: '密码重置响应不完整，请稍后重试',
        );
      }
      await _acceptTokens(tokens, beginsNewSession: true, operation: operation);
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    }
  }

  Future<void> setPassword({required String newPassword}) async {
    final int operation = ++_credentialOperation;
    final AuthApi? authenticatedAuthApi = _authenticatedAuthApi;
    if (authenticatedAuthApi == null || _accessToken == null) {
      throw const ApiFailure(
        kind: ApiFailureKind.unauthorized,
        message: '请先登录后再设置密码',
      );
    }
    try {
      final String installationId = await _installationStore.readOrCreateId();
      final Response<AuthTokens> response = await authenticatedAuthApi
          .authPasswordSetPost(
            passwordSetInput: PasswordSetInput(
              newPassword: newPassword,
              clientInstallationId: installationId,
            ),
          );
      final AuthTokens? tokens = response.data;
      if (tokens == null) {
        throw const ApiFailure(
          kind: ApiFailureKind.unexpected,
          message: '密码设置响应不完整，请稍后重试',
        );
      }
      await _acceptTokens(tokens, beginsNewSession: true, operation: operation);
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    }
  }

  Future<void> changePassword({
    required String currentPassword,
    required String newPassword,
  }) async {
    final int operation = ++_credentialOperation;
    final AuthApi? authenticatedAuthApi = _authenticatedAuthApi;
    if (authenticatedAuthApi == null || _accessToken == null) {
      throw const ApiFailure(
        kind: ApiFailureKind.unauthorized,
        message: '请先登录后再修改密码',
      );
    }
    try {
      final String installationId = await _installationStore.readOrCreateId();
      final Response<AuthTokens> response = await authenticatedAuthApi
          .authPasswordChangePost(
            passwordChangeInput: PasswordChangeInput(
              currentPassword: currentPassword,
              newPassword: newPassword,
              clientInstallationId: installationId,
            ),
          );
      final AuthTokens? tokens = response.data;
      if (tokens == null) {
        throw const ApiFailure(
          kind: ApiFailureKind.unexpected,
          message: '密码修改响应不完整，请稍后重试',
        );
      }
      await _acceptTokens(tokens, beginsNewSession: true, operation: operation);
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    }
  }

  Future<String?> refreshForRequest(int requestGeneration) async {
    if (requestGeneration != generation || _refreshToken == null) {
      return null;
    }
    return _refresh(coldStart: false);
  }

  Future<void> retrySession() async {
    if (_refreshToken == null) {
      _emit(SessionState.anonymous(generation: generation));
      return;
    }
    await _refresh(coldStart: false);
  }

  Future<bool> logout({bool revokeAll = false}) async {
    final int operation = ++_credentialOperation;
    final AuthApi? authenticatedAuthApi = _authenticatedAuthApi;
    final String? accountId = _activeAccountId;
    bool wasRevokedRemotely = false;
    if (_accessToken != null && authenticatedAuthApi != null) {
      try {
        if (revokeAll) {
          await authenticatedAuthApi.authLogoutAllPost(
            extra: const <String, Object>{'yourtj.disableSessionRetry': true},
          );
        } else {
          await authenticatedAuthApi.authLogoutPost(
            extra: const <String, Object>{'yourtj.disableSessionRetry': true},
          );
        }
        wasRevokedRemotely = true;
      } on DioException {
        wasRevokedRemotely = false;
      }
    }
    if (operation != _credentialOperation) {
      return wasRevokedRemotely;
    }
    if (accountId != null) {
      try {
        await _runStorageMutation(() async {
          if (operation == _credentialOperation) {
            await _storage.clearSession(accountId);
          }
        });
      } on Object {
        _clearMemory();
        _emit(
          SessionState(
            phase: SessionPhase.secureStorageUnavailable,
            generation: generation + 1,
            message: '已停止当前进程使用登录凭据，但无法确认本机持久凭据已清除。请勿将设备交给他人。',
          ),
        );
        return wasRevokedRemotely;
      }
    }
    if (operation != _credentialOperation) {
      return wasRevokedRemotely;
    }
    _clearMemory();
    _emit(SessionState.anonymous(generation: generation + 1));
    return wasRevokedRemotely;
  }

  Future<String?> _refresh({required bool coldStart}) {
    final Future<String?>? activeRefresh = _refreshFuture;
    if (activeRefresh != null) {
      return activeRefresh;
    }
    final Future<String?> refresh = _performRefresh(
      coldStart: coldStart,
      operation: _credentialOperation,
    );
    _refreshFuture = refresh;
    return refresh.whenComplete(() {
      if (identical(_refreshFuture, refresh)) {
        _refreshFuture = null;
      }
    });
  }

  Future<String?> _performRefresh({
    required bool coldStart,
    required int operation,
  }) async {
    final String? refreshToken = _refreshToken;
    final String? expectedAccountId = _activeAccountId;
    if (refreshToken == null || expectedAccountId == null) {
      if (coldStart) {
        _emit(SessionState.anonymous(generation: generation));
      }
      return null;
    }
    try {
      final Response<AuthTokens> response = await _publicAuthApi
          .authRefreshPost(
            refreshInput: RefreshInput(refreshToken: refreshToken),
          );
      if (operation != _credentialOperation) {
        return null;
      }
      final AuthTokens? tokens = response.data;
      if (tokens == null || tokens.account.id != expectedAccountId) {
        await _invalidateStoredSession(expectedAccountId, operation);
        return null;
      }
      await _acceptTokens(
        tokens,
        beginsNewSession: coldStart,
        operation: operation,
      );
      return tokens.accessToken;
    } on DioException catch (exception) {
      if (operation != _credentialOperation) {
        return null;
      }
      final ApiFailure failure = ApiFailure.fromDio(exception);
      if (failure.invalidatesRefreshCredential) {
        await _invalidateStoredSession(expectedAccountId, operation);
      } else {
        _accessToken = null;
        _emit(
          SessionState(
            phase: SessionPhase.reconnectRequired,
            generation: generation,
            account: state.account,
            message: failure.message,
          ),
        );
      }
      return null;
    } on Object {
      if (operation != _credentialOperation) {
        return null;
      }
      _clearMemory();
      _emit(
        SessionState(
          phase: SessionPhase.secureStorageUnavailable,
          generation: generation + 1,
          message: '无法更新系统安全存储。为保护账号，本机登录已停用。',
        ),
      );
      return null;
    }
  }

  Future<void> _acceptTokens(
    AuthTokens tokens, {
    required bool beginsNewSession,
    required int operation,
  }) async {
    if (operation != _credentialOperation) {
      throw const ApiFailure(
        kind: ApiFailureKind.cancelled,
        message: '已有更新的登录操作，已丢弃旧登录结果',
      );
    }
    try {
      await _runStorageMutation(() async {
        if (operation != _credentialOperation) {
          return;
        }
        await _storage.replaceSession(
          accountId: tokens.account.id,
          refreshToken: tokens.refreshToken,
        );
        if (operation != _credentialOperation) {
          await _storage.clearSession(tokens.account.id);
        }
      });
    } on Object {
      if (operation != _credentialOperation) {
        throw const ApiFailure(
          kind: ApiFailureKind.cancelled,
          message: '已有更新的登录操作，已丢弃旧登录结果',
        );
      }
      _clearMemory();
      _emit(
        SessionState(
          phase: SessionPhase.secureStorageUnavailable,
          generation: generation + 1,
          message: '无法安全保存登录凭证。账号未在本机保持登录。',
        ),
      );
      throw const ApiFailure(
        kind: ApiFailureKind.unexpected,
        message: '系统安全存储不可用，无法登录',
      );
    }
    if (operation != _credentialOperation) {
      throw const ApiFailure(
        kind: ApiFailureKind.cancelled,
        message: '已有更新的登录操作，已丢弃旧登录结果',
      );
    }
    final bool identityChanged = _activeAccountId != tokens.account.id;
    final int nextGeneration = beginsNewSession || identityChanged
        ? generation + 1
        : generation;
    _activeAccountId = tokens.account.id;
    _refreshToken = tokens.refreshToken;
    _accessToken = tokens.accessToken;
    _emit(
      SessionState.authenticated(
        generation: nextGeneration,
        account: tokens.account,
      ),
    );
  }

  Future<void> _invalidateStoredSession(String accountId, int operation) async {
    if (operation != _credentialOperation) {
      return;
    }
    try {
      await _runStorageMutation(() async {
        if (operation == _credentialOperation) {
          await _storage.clearSession(accountId);
        }
      });
    } on Object {
      _clearMemory();
      _emit(
        SessionState(
          phase: SessionPhase.secureStorageUnavailable,
          generation: generation + 1,
          message: '登录凭证已失效，但系统安全存储无法清理。请重新启动设备。',
        ),
      );
      return;
    }
    _clearMemory();
    _emit(SessionState.anonymous(generation: generation + 1));
  }

  Future<T> _runStorageMutation<T>(Future<T> Function() mutation) {
    final Completer<T> completer = Completer<T>();
    _storageTail = _storageTail.then((_) async {
      try {
        completer.complete(await mutation());
      } on Object catch (error, stackTrace) {
        completer.completeError(error, stackTrace);
      }
    });
    return completer.future;
  }

  void _clearMemory() {
    _accessToken = null;
    _refreshToken = null;
    _activeAccountId = null;
  }

  void _emit(SessionState nextState) {
    if (_isDisposed) {
      return;
    }
    _state = nextState;
    _changes.add(nextState);
    _routerRevision.value += 1;
  }

  Future<void> dispose() async {
    _isDisposed = true;
    _clearMemory();
    _routerRevision.dispose();
    await _changes.close();
  }
}
