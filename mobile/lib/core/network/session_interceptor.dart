import 'package:dio/dio.dart';

import '../../features/auth/data/session_manager.dart';
import '../config/app_environment.dart';
import 'api_failure.dart';

class SessionInterceptor extends Interceptor {
  SessionInterceptor(this._dio, this._environment, this._sessionManager);

  static const String _generationKey = 'yourtj.sessionGeneration';
  static const String _retryKey = 'yourtj.sessionRetry';
  static const String _disableRetryKey = 'yourtj.disableSessionRetry';

  final Dio _dio;
  final AppEnvironment _environment;
  final SessionManager _sessionManager;

  @override
  void onRequest(RequestOptions options, RequestInterceptorHandler handler) {
    final bool requiresBearer = _requiresBearer(options);
    final bool carriesAuthorization = options.headers.keys.any(
      (String key) => key.toLowerCase() == 'authorization',
    );
    if ((requiresBearer || carriesAuthorization) &&
        !_environment.owns(options.uri)) {
      handler.reject(
        DioException(
          requestOptions: options,
          type: DioExceptionType.unknown,
          error: const ApiFailure(
            kind: ApiFailureKind.forbidden,
            message: '已阻止向非 YourTJ API 地址发送账号凭证',
          ),
        ),
      );
      return;
    }
    if (requiresBearer) {
      options.extra[_generationKey] = _sessionManager.generation;
      final String? token = _sessionManager.accessToken;
      if (token != null) {
        options.headers['Authorization'] = 'Bearer $token';
      }
    }
    handler.next(options);
  }

  @override
  void onResponse(
    Response<dynamic> response,
    ResponseInterceptorHandler handler,
  ) {
    if (_belongsToSupersededSession(response.requestOptions)) {
      handler.reject(_sessionChanged(response.requestOptions));
      return;
    }
    handler.next(response);
  }

  @override
  void onError(DioException err, ErrorInterceptorHandler handler) async {
    final RequestOptions options = err.requestOptions;
    if (_belongsToSupersededSession(options)) {
      handler.reject(_sessionChanged(options));
      return;
    }
    final bool canRefresh =
        err.response?.statusCode == 401 &&
        _requiresBearer(options) &&
        options.extra[_retryKey] != true &&
        options.extra[_disableRetryKey] != true &&
        _isSafeMethod(options.method) &&
        !_hasWalletSignature(options) &&
        _environment.owns(options.uri);
    if (!canRefresh) {
      handler.next(err);
      return;
    }
    final Object? rawGeneration = options.extra[_generationKey];
    if (rawGeneration is! int) {
      handler.next(err);
      return;
    }
    final String? token = await _sessionManager.refreshForRequest(
      rawGeneration,
    );
    if (token == null || rawGeneration != _sessionManager.generation) {
      handler.next(err);
      return;
    }
    options.extra[_retryKey] = true;
    options.extra[_generationKey] = _sessionManager.generation;
    options.headers['Authorization'] = 'Bearer $token';
    try {
      final Response<dynamic> retried = await _dio.fetch<dynamic>(options);
      handler.resolve(retried);
    } on DioException catch (retryError) {
      handler.next(retryError);
    }
  }

  bool _belongsToSupersededSession(RequestOptions options) {
    if (!_requiresBearer(options)) {
      return false;
    }
    final Object? requestGeneration = options.extra[_generationKey];
    return requestGeneration is int &&
        requestGeneration != _sessionManager.generation;
  }

  bool _requiresBearer(RequestOptions options) {
    final Object? secure = options.extra['secure'];
    if (secure is! List) {
      return false;
    }
    return secure.any((Object? entry) {
      return entry is Map &&
          entry['type'] == 'http' &&
          entry['scheme']?.toString().toLowerCase() == 'bearer';
    });
  }

  bool _hasWalletSignature(RequestOptions options) {
    return options.headers.keys.any(
      (String key) => key.toLowerCase() == 'x-wallet-sig',
    );
  }

  bool _isSafeMethod(String method) {
    return <String>{'GET', 'HEAD', 'OPTIONS'}.contains(method.toUpperCase());
  }

  DioException _sessionChanged(RequestOptions options) {
    return DioException(
      requestOptions: options,
      type: DioExceptionType.cancel,
      error: const ApiFailure(
        kind: ApiFailureKind.cancelled,
        message: '账号已切换，已丢弃旧账号请求结果',
      ),
    );
  }
}
