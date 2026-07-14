import 'dart:async';

import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_mobile/core/config/app_environment.dart';
import 'package:yourtj_mobile/core/network/api_failure.dart';
import 'package:yourtj_mobile/core/network/session_interceptor.dart';

import '../../features/auth/support/session_test_support.dart';

void main() {
  test('adds bearer only to generated secure operations', () async {
    final SessionHarness session = authenticatedSession();
    addTearDown(session.dispose);
    await logIn(session);
    final RecordingAdapter apiAdapter = RecordingAdapter(
      (RequestOptions options) => jsonResponse(<String, Object?>{'ok': true}),
    );
    final Dio apiDio = createApiDio(session, apiAdapter);
    addTearDown(() => apiDio.close(force: true));

    await apiDio.get<Object>('/public');
    await apiDio.get<Object>('/private', options: secureOptions());

    expect(apiAdapter.requests, hasLength(2));
    expect(authorization(apiAdapter.requests[0]), isNull);
    expect(authorization(apiAdapter.requests[1]), 'Bearer access-a');
  });

  test('rejects credentials bound for a different origin', () async {
    final SessionHarness session = authenticatedSession();
    addTearDown(session.dispose);
    await logIn(session);
    final RecordingAdapter apiAdapter = RecordingAdapter(
      (RequestOptions options) => jsonResponse(<String, Object?>{'ok': true}),
    );
    final Dio apiDio = createApiDio(session, apiAdapter);
    addTearDown(() => apiDio.close(force: true));

    await expectLater(
      apiDio.get<Object>(
        'https://evil.example/api/v2/private',
        options: secureOptions(),
      ),
      throwsA(
        isA<DioException>().having(
          (DioException exception) => exception.error,
          'error',
          isA<ApiFailure>().having(
            (ApiFailure failure) => failure.kind,
            'kind',
            ApiFailureKind.forbidden,
          ),
        ),
      ),
    );
    await expectLater(
      apiDio.get<Object>(
        'https://cdn.example/asset',
        options: Options(
          headers: <String, Object>{'Authorization': 'Bearer manual'},
        ),
      ),
      throwsA(isA<DioException>()),
    );
    expect(apiAdapter.requests, isEmpty);
  });

  test('refreshes after 401 and replays the request exactly once', () async {
    int refreshCount = 0;
    final SessionHarness session = SessionHarness(
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
        refreshCount += 1;
        return jsonResponse(
          authTokensJson(
            accountId: 'account-a',
            accessToken: 'access-new',
            refreshToken: 'refresh-new',
          ),
        );
      },
    );
    addTearDown(session.dispose);
    await logIn(session);
    final RecordingAdapter apiAdapter = RecordingAdapter((
      RequestOptions options,
    ) {
      final String? bearer = header(options.headers, 'authorization');
      if (bearer == 'Bearer access-old') {
        return jsonResponse(<String, Object?>{
          'error': <String, String>{
            'code': 'UNAUTHORIZED',
            'message': 'expired',
          },
        }, statusCode: 401);
      }
      return jsonResponse(<String, Object?>{'ok': true});
    });
    final Dio apiDio = createApiDio(session, apiAdapter);
    addTearDown(() => apiDio.close(force: true));

    final Response<Object> response = await apiDio.get<Object>(
      '/private',
      options: secureOptions(),
    );

    expect(response.statusCode, 200);
    expect(refreshCount, 1);
    expect(apiAdapter.requests, hasLength(2));
    expect(authorization(apiAdapter.requests[0]), 'Bearer access-old');
    expect(authorization(apiAdapter.requests[1]), 'Bearer access-new');
    expect(session.manager.accessToken, 'access-new');
    expect(session.storage.refreshTokens['account-a'], 'refresh-new');
  });

  test('a retried 401 does not trigger another refresh or replay', () async {
    int refreshCount = 0;
    final SessionHarness session = SessionHarness(
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
        refreshCount += 1;
        return jsonResponse(
          authTokensJson(
            accountId: 'account-a',
            accessToken: 'access-new',
            refreshToken: 'refresh-new',
          ),
        );
      },
    );
    addTearDown(session.dispose);
    await logIn(session);
    final RecordingAdapter apiAdapter = RecordingAdapter(
      (RequestOptions options) => jsonResponse(<String, Object?>{
        'error': <String, String>{
          'code': 'UNAUTHORIZED',
          'message': 'still unauthorized',
        },
      }, statusCode: 401),
    );
    final Dio apiDio = createApiDio(session, apiAdapter);
    addTearDown(() => apiDio.close(force: true));

    await expectLater(
      apiDio.get<Object>('/private', options: secureOptions()),
      throwsA(
        isA<DioException>().having(
          (DioException exception) => exception.response?.statusCode,
          'status',
          401,
        ),
      ),
    );

    expect(refreshCount, 1);
    expect(apiAdapter.requests, hasLength(2));
  });

  test('concurrent 401 responses share one refresh', () async {
    final Completer<ResponseBody> refreshResponse = Completer<ResponseBody>();
    final Completer<void> refreshStarted = Completer<void>();
    int refreshCount = 0;
    final SessionHarness session = SessionHarness(
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
        refreshCount += 1;
        if (!refreshStarted.isCompleted) {
          refreshStarted.complete();
        }
        return refreshResponse.future;
      },
    );
    addTearDown(session.dispose);
    await logIn(session);
    final RecordingAdapter apiAdapter = RecordingAdapter((
      RequestOptions options,
    ) {
      if (header(options.headers, 'authorization') == 'Bearer access-old') {
        return jsonResponse(<String, Object?>{
          'error': <String, String>{
            'code': 'UNAUTHORIZED',
            'message': 'expired',
          },
        }, statusCode: 401);
      }
      return jsonResponse(<String, Object?>{'ok': true});
    });
    final Dio apiDio = createApiDio(session, apiAdapter);
    addTearDown(() => apiDio.close(force: true));

    final Future<Response<Object>> first = apiDio.get<Object>(
      '/private/one',
      options: secureOptions(),
    );
    final Future<Response<Object>> second = apiDio.get<Object>(
      '/private/two',
      options: secureOptions(),
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
    await Future.wait(<Future<Response<Object>>>[first, second]);

    expect(refreshCount, 1);
    expect(apiAdapter.requests, hasLength(4));
    expect(
      apiAdapter.requests
          .map(authorization)
          .where((String? value) => value == 'Bearer access-old'),
      hasLength(2),
    );
    expect(
      apiAdapter.requests
          .map(authorization)
          .where((String? value) => value == 'Bearer access-new'),
      hasLength(2),
    );
  });

  test('drops a successful response from a superseded generation', () async {
    final SessionHarness session = authenticatedSession();
    addTearDown(session.dispose);
    await logIn(session);
    final Completer<ResponseBody> response = Completer<ResponseBody>();
    final Completer<void> requestStarted = Completer<void>();
    final RecordingAdapter apiAdapter = RecordingAdapter((
      RequestOptions options,
    ) {
      requestStarted.complete();
      return response.future;
    });
    final Dio apiDio = createApiDio(session, apiAdapter);
    addTearDown(() => apiDio.close(force: true));

    final Future<Response<Object>> oldRequest = apiDio.get<Object>(
      '/private',
      options: secureOptions(),
    );
    await requestStarted.future;
    expect(await session.manager.logout(), isFalse);
    response.complete(jsonResponse(<String, Object?>{'private': 'old'}));

    await expectLater(
      oldRequest,
      throwsA(
        isA<DioException>()
            .having(
              (DioException exception) => exception.type,
              'type',
              DioExceptionType.cancel,
            )
            .having(
              (DioException exception) => exception.error,
              'error',
              isA<ApiFailure>().having(
                (ApiFailure failure) => failure.kind,
                'kind',
                ApiFailureKind.cancelled,
              ),
            ),
      ),
    );
  });

  test('never retries a wallet-signed request', () async {
    int refreshCount = 0;
    final SessionHarness session = SessionHarness(
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
        refreshCount += 1;
        return jsonResponse(
          authTokensJson(
            accountId: 'account-a',
            accessToken: 'access-new',
            refreshToken: 'refresh-new',
          ),
        );
      },
    );
    addTearDown(session.dispose);
    await logIn(session);
    final RecordingAdapter apiAdapter = RecordingAdapter(
      (RequestOptions options) => jsonResponse(<String, Object?>{
        'error': <String, String>{'code': 'UNAUTHORIZED', 'message': 'expired'},
      }, statusCode: 401),
    );
    final Dio apiDio = createApiDio(session, apiAdapter);
    addTearDown(() => apiDio.close(force: true));

    await expectLater(
      apiDio.post<Object>(
        '/wallet/mutation',
        options: secureOptions(
          headers: <String, Object>{'X-Wallet-Sig': 'signature'},
        ),
      ),
      throwsA(isA<DioException>()),
    );

    expect(refreshCount, 0);
    expect(apiAdapter.requests, hasLength(1));
    expect(
      header(apiAdapter.requests.single.headers, 'x-wallet-sig'),
      'signature',
    );
  });

  test('never retries an admin mutation after 401', () async {
    int refreshCount = 0;
    final SessionHarness session = SessionHarness(
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
        refreshCount += 1;
        return jsonResponse(
          authTokensJson(
            accountId: 'account-a',
            accessToken: 'access-new',
            refreshToken: 'refresh-new',
          ),
        );
      },
    );
    addTearDown(session.dispose);
    await logIn(session);
    final RecordingAdapter apiAdapter = RecordingAdapter(
      (RequestOptions options) => jsonResponse(<String, Object?>{
        'error': <String, String>{'code': 'UNAUTHORIZED', 'message': 'expired'},
      }, statusCode: 401),
    );
    final Dio apiDio = createApiDio(session, apiAdapter);
    addTearDown(() => apiDio.close(force: true));

    await expectLater(
      apiDio.post<Object>('/admin/settings/example', options: secureOptions()),
      throwsA(isA<DioException>()),
    );

    expect(refreshCount, 0);
    expect(apiAdapter.requests, hasLength(1));
  });

  test('never retries a generic mutation after 401', () async {
    int refreshCount = 0;
    final SessionHarness session = SessionHarness(
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
        refreshCount += 1;
        return jsonResponse(
          authTokensJson(
            accountId: 'account-a',
            accessToken: 'access-new',
            refreshToken: 'refresh-new',
          ),
        );
      },
    );
    addTearDown(session.dispose);
    await logIn(session);
    final RecordingAdapter apiAdapter = RecordingAdapter(
      (RequestOptions options) => jsonResponse(<String, Object?>{
        'error': <String, String>{'code': 'UNAUTHORIZED', 'message': 'expired'},
      }, statusCode: 401),
    );
    final Dio apiDio = createApiDio(session, apiAdapter);
    addTearDown(() => apiDio.close(force: true));

    await expectLater(
      apiDio.post<Object>('/forum/threads', options: secureOptions()),
      throwsA(isA<DioException>()),
    );

    expect(refreshCount, 0);
    expect(apiAdapter.requests, hasLength(1));
  });
}

SessionHarness authenticatedSession() {
  return SessionHarness(
    handler: (RequestOptions options) => jsonResponse(
      authTokensJson(
        accountId: 'account-a',
        accessToken: 'access-a',
        refreshToken: 'refresh-a',
      ),
    ),
  );
}

Future<void> logIn(SessionHarness session) async {
  await session.manager.passwordLogin(
    email: 'student@tongji.edu.cn',
    password: 'password',
  );
  expect(session.manager.accessToken, isNotNull);
}

Dio createApiDio(SessionHarness session, RecordingAdapter adapter) {
  final AppEnvironment environment = AppEnvironment(
    apiBaseUri: Uri.parse('https://api.yourtj.de/api/v2'),
  );
  final Dio dio = Dio(BaseOptions(baseUrl: environment.apiBaseUri.toString()))
    ..httpClientAdapter = adapter;
  dio.interceptors.add(SessionInterceptor(dio, environment, session.manager));
  return dio;
}

Options secureOptions({Map<String, Object>? headers}) {
  return Options(
    headers: headers,
    extra: <String, Object>{
      'secure': <Map<String, String>>[bearerSecurity()],
    },
  );
}

String? authorization(RecordedRequest request) {
  return header(request.headers, 'authorization');
}

String? header(Map<String, dynamic> headers, String name) {
  for (final MapEntry<String, dynamic> entry in headers.entries) {
    if (entry.key.toLowerCase() == name.toLowerCase()) {
      return entry.value?.toString();
    }
  }
  return null;
}
