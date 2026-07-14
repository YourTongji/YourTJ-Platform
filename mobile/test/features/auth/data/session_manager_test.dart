import 'dart:async';

import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/core/network/api_failure.dart';
import 'package:yourtj_mobile/features/auth/domain/session_state.dart';

import '../support/session_test_support.dart';

void main() {
  test(
    'cold start rotates the stored refresh token before authenticating',
    () async {
      final MemorySessionStorage storage = MemorySessionStorage(
        activeAccountId: 'account-a',
        refreshTokens: <String, String>{'account-a': 'refresh-old'},
      );
      late SessionHarness harness;
      harness = SessionHarness(
        storage: storage,
        handler: (RequestOptions options) {
          expect(options.path, '/auth/refresh');
          return jsonResponse(
            authTokensJson(
              accountId: 'account-a',
              accessToken: 'access-new',
              refreshToken: 'refresh-new',
            ),
          );
        },
      );
      addTearDown(harness.dispose);

      await harness.manager.initialize();

      expect(harness.manager.state.phase, SessionPhase.authenticated);
      expect(harness.manager.state.account?.id, 'account-a');
      expect(harness.manager.generation, 1);
      expect(harness.manager.accessToken, 'access-new');
      expect(storage.activeAccountId, 'account-a');
      expect(storage.refreshTokens, <String, String>{
        'account-a': 'refresh-new',
      });
    },
  );

  test(
    'password login trims email and persists only the refresh token',
    () async {
      late SessionHarness harness;
      harness = SessionHarness(
        handler: (RequestOptions options) {
          expect(options.path, '/auth/password/login');
          return jsonResponse(
            authTokensJson(
              accountId: 'account-a',
              accessToken: 'access-a',
              refreshToken: 'refresh-a',
            ),
          );
        },
      );
      addTearDown(harness.dispose);

      await harness.manager.passwordLogin(
        email: '  student@tongji.edu.cn  ',
        password: 'correct horse battery staple',
      );

      final Map<String, dynamic> body = requestJson(
        harness.adapter.requests.single,
      );
      expect(body, <String, Object?>{
        'email': 'student@tongji.edu.cn',
        'password': 'correct horse battery staple',
        'clientInstallationId': harness.installationStore.installationId,
      });
      expect(harness.manager.state.phase, SessionPhase.authenticated);
      expect(harness.manager.accessToken, 'access-a');
      expect(harness.storage.activeAccountId, 'account-a');
      expect(harness.storage.refreshTokens, <String, String>{
        'account-a': 'refresh-a',
      });
      expect(harness.storage.refreshTokens.values, isNot(contains('access-a')));
    },
  );

  test('password reset adopts the replacement session atomically', () async {
    late SessionHarness harness;
    harness = SessionHarness(
      handler: (RequestOptions options) {
        if (options.path == '/auth/password/forgot') {
          return jsonResponse(null, statusCode: 204);
        }
        expect(options.path, '/auth/password/reset');
        return jsonResponse(
          authTokensJson(
            accountId: 'account-a',
            accessToken: 'access-after-reset',
            refreshToken: 'refresh-after-reset',
          ),
        );
      },
    );
    addTearDown(harness.dispose);

    await harness.manager.requestPasswordReset(
      email: '  student@tongji.edu.cn  ',
      captchaToken: 'captcha-proof',
    );
    await harness.manager.resetPassword(
      email: '  student@tongji.edu.cn  ',
      code: '  123456  ',
      newPassword: 'replacement-password',
    );

    expect(requestJson(harness.adapter.requests[0]), <String, Object?>{
      'email': 'student@tongji.edu.cn',
      'captchaToken': 'captcha-proof',
    });
    expect(requestJson(harness.adapter.requests[1]), <String, Object?>{
      'email': 'student@tongji.edu.cn',
      'code': '123456',
      'newPassword': 'replacement-password',
      'clientInstallationId': harness.installationStore.installationId,
    });
    expect(harness.manager.state.phase, SessionPhase.authenticated);
    expect(harness.manager.accessToken, 'access-after-reset');
    expect(harness.storage.refreshTokens, <String, String>{
      'account-a': 'refresh-after-reset',
    });
  });

  test('concurrent refresh callers share one network rotation', () async {
    final Completer<ResponseBody> refreshResponse = Completer<ResponseBody>();
    final Completer<void> refreshStarted = Completer<void>();
    int refreshCount = 0;
    late SessionHarness harness;
    harness = SessionHarness(
      handler: (RequestOptions options) {
        if (options.path == '/auth/password/login') {
          return jsonResponse(
            authTokensJson(
              accountId: 'account-a',
              accessToken: 'access-old',
              refreshToken: 'refresh-old',
            ),
          );
        }
        expect(options.path, '/auth/refresh');
        refreshCount += 1;
        refreshStarted.complete();
        return refreshResponse.future;
      },
    );
    addTearDown(harness.dispose);
    await harness.manager.passwordLogin(
      email: 'student@tongji.edu.cn',
      password: 'password',
    );
    final int generation = harness.manager.generation;

    final Future<String?> first = harness.manager.refreshForRequest(generation);
    final Future<String?> second = harness.manager.refreshForRequest(
      generation,
    );
    await refreshStarted.future;

    expect(refreshCount, 1);
    refreshResponse.complete(
      jsonResponse(
        authTokensJson(
          accountId: 'account-a',
          accessToken: 'access-new',
          refreshToken: 'refresh-new',
        ),
      ),
    );
    expect(await first, 'access-new');
    expect(await second, 'access-new');
    expect(harness.manager.generation, generation);
    expect(harness.storage.refreshTokens['account-a'], 'refresh-new');
  });

  test('terminal refresh rejection clears the local credential', () async {
    final MemorySessionStorage storage = MemorySessionStorage(
      activeAccountId: 'account-a',
      refreshTokens: <String, String>{'account-a': 'refresh-revoked'},
    );
    late SessionHarness harness;
    harness = SessionHarness(
      storage: storage,
      handler: (RequestOptions options) => jsonResponse(<String, Object?>{
        'error': <String, String>{
          'code': 'INVALID_REFRESH_TOKEN',
          'message': 'refresh token invalid',
        },
      }, statusCode: 401),
    );
    addTearDown(harness.dispose);

    await harness.manager.initialize();

    expect(harness.manager.state.phase, SessionPhase.anonymous);
    expect(harness.manager.generation, 1);
    expect(harness.manager.accessToken, isNull);
    expect(storage.activeAccountId, isNull);
    expect(storage.refreshTokens, isEmpty);
    expect(storage.clearedAccounts, <String>['account-a']);
  });

  test(
    'network refresh failure keeps the refresh token for explicit retry',
    () async {
      final MemorySessionStorage storage = MemorySessionStorage(
        activeAccountId: 'account-a',
        refreshTokens: <String, String>{'account-a': 'refresh-offline'},
      );
      late SessionHarness harness;
      harness = SessionHarness(
        storage: storage,
        handler: (RequestOptions options) {
          throw DioException(
            requestOptions: options,
            type: DioExceptionType.connectionError,
            error: StateError('offline'),
          );
        },
      );
      addTearDown(harness.dispose);

      await harness.manager.initialize();

      expect(harness.manager.state.phase, SessionPhase.reconnectRequired);
      expect(harness.manager.generation, 0);
      expect(harness.manager.accessToken, isNull);
      expect(storage.activeAccountId, 'account-a');
      expect(storage.refreshTokens['account-a'], 'refresh-offline');
      expect(storage.clearedAccounts, isEmpty);
    },
  );

  test(
    'the latest login wins when an older account response arrives late',
    () async {
      final Completer<ResponseBody> firstResponse = Completer<ResponseBody>();
      final Completer<ResponseBody> secondResponse = Completer<ResponseBody>();
      int loginCount = 0;
      late SessionHarness harness;
      harness = SessionHarness(
        handler: (RequestOptions options) {
          loginCount += 1;
          return loginCount == 1 ? firstResponse.future : secondResponse.future;
        },
      );
      addTearDown(harness.dispose);

      final Future<void> firstLogin = harness.manager.passwordLogin(
        email: 'first@tongji.edu.cn',
        password: 'first-password',
      );
      final Future<void> secondLogin = harness.manager.passwordLogin(
        email: 'second@tongji.edu.cn',
        password: 'second-password',
      );
      await Future<void>.delayed(Duration.zero);

      secondResponse.complete(
        jsonResponse(
          authTokensJson(
            accountId: 'account-b',
            accessToken: 'access-b',
            refreshToken: 'refresh-b',
          ),
        ),
      );
      await secondLogin;
      firstResponse.complete(
        jsonResponse(
          authTokensJson(
            accountId: 'account-a',
            accessToken: 'access-a',
            refreshToken: 'refresh-a',
          ),
        ),
      );

      await expectLater(
        firstLogin,
        throwsA(
          isA<ApiFailure>().having(
            (ApiFailure failure) => failure.kind,
            'kind',
            ApiFailureKind.cancelled,
          ),
        ),
      );
      expect(harness.manager.state.account?.id, 'account-b');
      expect(harness.manager.accessToken, 'access-b');
      expect(harness.manager.generation, 1);
      expect(harness.storage.activeAccountId, 'account-b');
      expect(harness.storage.refreshTokens, <String, String>{
        'account-b': 'refresh-b',
      });
    },
  );

  test('a late refresh cannot restore the account after logout', () async {
    final Completer<ResponseBody> refreshResponse = Completer<ResponseBody>();
    late SessionHarness harness;
    harness = SessionHarness(
      handler: (RequestOptions options) {
        if (options.path == '/auth/password/login') {
          return jsonResponse(
            authTokensJson(
              accountId: 'account-a',
              accessToken: 'access-old',
              refreshToken: 'refresh-old',
            ),
          );
        }
        return refreshResponse.future;
      },
    );
    addTearDown(harness.dispose);
    await harness.manager.passwordLogin(
      email: 'student@tongji.edu.cn',
      password: 'password',
    );
    final Future<String?> refresh = harness.manager.refreshForRequest(
      harness.manager.generation,
    );
    await Future<void>.delayed(Duration.zero);

    expect(await harness.manager.logout(), isFalse);
    refreshResponse.complete(
      jsonResponse(
        authTokensJson(
          accountId: 'account-a',
          accessToken: 'late-access',
          refreshToken: 'late-refresh',
        ),
      ),
    );

    expect(await refresh, isNull);
    expect(harness.manager.state.phase, SessionPhase.anonymous);
    expect(harness.manager.generation, 2);
    expect(harness.manager.accessToken, isNull);
    expect(harness.storage.activeAccountId, isNull);
    expect(harness.storage.refreshTokens, isEmpty);
  });

  test('a late refresh cannot overwrite a newer account login', () async {
    final Completer<ResponseBody> refreshResponse = Completer<ResponseBody>();
    final Completer<void> refreshStarted = Completer<void>();
    int loginCount = 0;
    late SessionHarness harness;
    harness = SessionHarness(
      handler: (RequestOptions options) {
        if (options.path == '/auth/password/login') {
          loginCount += 1;
          final bool isFirstLogin = loginCount == 1;
          return jsonResponse(
            authTokensJson(
              accountId: isFirstLogin ? 'account-a' : 'account-b',
              accessToken: isFirstLogin ? 'access-a' : 'access-b',
              refreshToken: isFirstLogin ? 'refresh-a' : 'refresh-b',
            ),
          );
        }
        if (!refreshStarted.isCompleted) {
          refreshStarted.complete();
        }
        return refreshResponse.future;
      },
    );
    addTearDown(harness.dispose);
    await harness.manager.passwordLogin(
      email: 'first@tongji.edu.cn',
      password: 'first-password',
    );
    final Future<String?> refresh = harness.manager.refreshForRequest(
      harness.manager.generation,
    );
    await refreshStarted.future;

    await harness.manager.passwordLogin(
      email: 'second@tongji.edu.cn',
      password: 'second-password',
    );
    refreshResponse.complete(
      jsonResponse(
        authTokensJson(
          accountId: 'account-a',
          accessToken: 'late-access-a',
          refreshToken: 'late-refresh-a',
        ),
      ),
    );

    expect(await refresh, isNull);
    expect(harness.manager.state.phase, SessionPhase.authenticated);
    expect(harness.manager.state.account?.id, 'account-b');
    expect(harness.manager.accessToken, 'access-b');
    expect(harness.manager.generation, 2);
    expect(harness.storage.activeAccountId, 'account-b');
    expect(harness.storage.refreshTokens, <String, String>{
      'account-b': 'refresh-b',
    });
  });

  test(
    'password set and change atomically publish replacement tokens',
    () async {
      late SessionHarness harness;
      harness = SessionHarness(
        handler: (RequestOptions options) {
          final ({String access, String refresh}) tokens =
              switch (options.path) {
                '/auth/password/login' => (
                  access: 'access-old',
                  refresh: 'refresh-old',
                ),
                '/auth/password/set' => (
                  access: 'access-after-set',
                  refresh: 'refresh-after-set',
                ),
                '/auth/password/change' => (
                  access: 'access-after-change',
                  refresh: 'refresh-after-change',
                ),
                _ => throw StateError('unexpected auth route: ${options.path}'),
              };
          return jsonResponse(
            authTokensJson(
              accountId: 'account-a',
              accessToken: tokens.access,
              refreshToken: tokens.refresh,
            ),
          );
        },
      );
      addTearDown(harness.dispose);
      harness.manager.attachAuthenticatedApi(AuthApi(harness.dio));
      await harness.manager.passwordLogin(
        email: 'student@tongji.edu.cn',
        password: 'initial-password',
      );

      await harness.manager.setPassword(newPassword: 'new-password');

      expect(harness.manager.accessToken, 'access-after-set');
      expect(harness.manager.generation, 2);
      expect(harness.storage.refreshTokens, <String, String>{
        'account-a': 'refresh-after-set',
      });
      expect(requestJson(harness.adapter.requests[1]), <String, Object?>{
        'newPassword': 'new-password',
        'clientInstallationId': harness.installationStore.installationId,
      });

      await harness.manager.changePassword(
        currentPassword: 'new-password',
        newPassword: 'newer-password',
      );

      expect(harness.manager.accessToken, 'access-after-change');
      expect(harness.manager.generation, 3);
      expect(harness.storage.refreshTokens, <String, String>{
        'account-a': 'refresh-after-change',
      });
      expect(requestJson(harness.adapter.requests[2]), <String, Object?>{
        'currentPassword': 'new-password',
        'newPassword': 'newer-password',
        'clientInstallationId': harness.installationStore.installationId,
      });
    },
  );

  test(
    'secure storage write failure fails closed without an access token',
    () async {
      final MemorySessionStorage storage = MemorySessionStorage()
        ..replaceError = StateError('keystore unavailable');
      late SessionHarness harness;
      harness = SessionHarness(
        storage: storage,
        handler: (RequestOptions options) => jsonResponse(
          authTokensJson(
            accountId: 'account-a',
            accessToken: 'must-not-escape',
            refreshToken: 'must-not-persist',
          ),
        ),
      );
      addTearDown(harness.dispose);

      await expectLater(
        harness.manager.passwordLogin(
          email: 'student@tongji.edu.cn',
          password: 'password',
        ),
        throwsA(
          isA<ApiFailure>().having(
            (ApiFailure failure) => failure.kind,
            'kind',
            ApiFailureKind.unexpected,
          ),
        ),
      );

      expect(
        harness.manager.state.phase,
        SessionPhase.secureStorageUnavailable,
      );
      expect(harness.manager.generation, 1);
      expect(harness.manager.accessToken, isNull);
      expect(storage.activeAccountId, isNull);
      expect(storage.refreshTokens, isEmpty);
    },
  );

  test(
    'logout cleanup failure remains explicit and the credential is restartable',
    () async {
      final MemorySessionStorage storage = MemorySessionStorage();
      final SessionHarness firstHarness = SessionHarness(
        storage: storage,
        handler: (RequestOptions options) => jsonResponse(
          authTokensJson(
            accountId: 'account-a',
            accessToken: 'access-a',
            refreshToken: 'refresh-a',
          ),
        ),
      );
      addTearDown(firstHarness.dispose);
      await firstHarness.manager.passwordLogin(
        email: 'student@tongji.edu.cn',
        password: 'password',
      );
      storage.clearError = StateError('keystore cleanup unavailable');

      expect(await firstHarness.manager.logout(), isFalse);

      expect(
        firstHarness.manager.state.phase,
        SessionPhase.secureStorageUnavailable,
      );
      expect(firstHarness.manager.state.message, contains('无法确认'));
      expect(firstHarness.manager.state.message, isNot(contains('登录已结束')));
      expect(storage.activeAccountId, 'account-a');
      expect(storage.refreshTokens['account-a'], 'refresh-a');

      storage.clearError = null;
      final SessionHarness restartHarness = SessionHarness(
        storage: storage,
        handler: (RequestOptions options) {
          expect(options.path, '/auth/refresh');
          return jsonResponse(
            authTokensJson(
              accountId: 'account-a',
              accessToken: 'access-after-restart',
              refreshToken: 'refresh-after-restart',
            ),
          );
        },
      );
      addTearDown(restartHarness.dispose);

      await restartHarness.manager.initialize();

      expect(restartHarness.manager.state.phase, SessionPhase.authenticated);
      expect(restartHarness.manager.accessToken, 'access-after-restart');
    },
  );

  test(
    'secure storage read failure disables local sign-in restoration',
    () async {
      final MemorySessionStorage storage = MemorySessionStorage()
        ..readError = StateError('keystore unavailable');
      late SessionHarness harness;
      harness = SessionHarness(
        storage: storage,
        handler: (RequestOptions options) {
          fail('no network request is allowed after secure-storage failure');
        },
      );
      addTearDown(harness.dispose);

      await harness.manager.initialize();

      expect(
        harness.manager.state.phase,
        SessionPhase.secureStorageUnavailable,
      );
      expect(harness.manager.generation, 1);
      expect(harness.manager.accessToken, isNull);
      expect(harness.adapter.requests, isEmpty);
    },
  );
}
