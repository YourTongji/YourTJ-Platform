import 'package:dio/dio.dart';
import 'package:flutter/foundation.dart';

import '../../../core/config/app_environment.dart';

@immutable
class CaptchaChallenge {
  const CaptchaChallenge({
    required this.puzzleToken,
    required this.prompt,
    required this.imageUris,
  });

  final String puzzleToken;
  final String prompt;
  final List<Uri> imageUris;
}

class CaptchaFailure implements Exception {
  const CaptchaFailure(this.message);

  final String message;

  @override
  String toString() => message;
}

class CaptchaClient {
  CaptchaClient({required AppEnvironment environment, Dio? dio})
    : _environment = environment,
      _dio =
          dio ??
          Dio(
            BaseOptions(
              baseUrl: environment.captchaBaseUri.toString(),
              connectTimeout: const Duration(seconds: 10),
              receiveTimeout: const Duration(seconds: 20),
              sendTimeout: const Duration(seconds: 20),
              followRedirects: false,
              maxRedirects: 0,
              headers: const <String, Object>{'Accept': 'application/json'},
            ),
          ) {
    _dio.interceptors.add(_CaptchaOriginInterceptor(environment));
  }

  final AppEnvironment _environment;
  final Dio _dio;

  Future<CaptchaChallenge> loadChallenge() async {
    try {
      final Response<Object?> response = await _dio.get<Object?>(
        '/api/captcha',
        options: Options(extra: const <String, Object>{'cache': 'no-store'}),
      );
      return _parseChallenge(response.data);
    } on CaptchaFailure {
      rethrow;
    } on DioException {
      throw const CaptchaFailure('验证码加载失败，请检查网络后重试');
    }
  }

  Future<String> verify({
    required CaptchaChallenge challenge,
    required Set<int> selectedIndices,
  }) async {
    final List<int> sortedIndices = selectedIndices.toList()..sort();
    if (sortedIndices.any(
      (int index) => index < 0 || index >= challenge.imageUris.length,
    )) {
      throw const CaptchaFailure('验证码选项无效，请刷新后重试');
    }
    try {
      final Response<Object?> response = await _dio.post<Object?>(
        '/api/verify',
        data: <String, Object>{
          'puzzle_token': challenge.puzzleToken,
          'selected_indices': sortedIndices,
        },
      );
      final Object? data = response.data;
      if (data is! Map || data['success'] != true) {
        final Object? message = data is Map ? data['message'] : null;
        throw CaptchaFailure(
          message is String && message.trim().isNotEmpty
              ? message
              : '选择不正确，请换一组后重试',
        );
      }
      final Object? token = data['token'];
      if (token is! String || token.isEmpty || token.length > 4096) {
        throw const CaptchaFailure('验证码服务返回了无效令牌');
      }
      return token;
    } on CaptchaFailure {
      rethrow;
    } on DioException {
      throw const CaptchaFailure('验证请求失败，请稍后重试');
    }
  }

  CaptchaChallenge _parseChallenge(Object? data) {
    if (data is! Map) {
      throw const CaptchaFailure('验证码服务返回了无效挑战');
    }
    final Object? rawToken = data['puzzle_token'];
    final Object? rawImages = data['images'];
    if (rawToken is! String ||
        rawToken.isEmpty ||
        rawToken.length > 4096 ||
        rawImages is! List ||
        rawImages.isEmpty ||
        rawImages.length > 12 ||
        rawImages.any((Object? image) => image is! String)) {
      throw const CaptchaFailure('验证码服务返回了不完整挑战');
    }
    final List<Uri> imageUris = rawImages
        .map((Object? image) {
          final Uri imageUri = _environment.captchaBaseUri.resolve(
            image! as String,
          );
          if (!_environment.ownsCaptcha(imageUri)) {
            throw const CaptchaFailure('验证码图片来自未授权地址');
          }
          return imageUri;
        })
        .toList(growable: false);
    final Object? rawPrompt = data['prompt'];
    final Object? questionType = data['questionType'];
    final String fallbackPrompt = questionType == 'TONGJI_NOT_IN'
        ? '选择下列不在同济校内的图片：'
        : '选择下列在同济校内的图片：';
    return CaptchaChallenge(
      puzzleToken: rawToken,
      prompt: rawPrompt is String && rawPrompt.trim().isNotEmpty
          ? rawPrompt.trim()
          : fallbackPrompt,
      imageUris: imageUris,
    );
  }
}

class _CaptchaOriginInterceptor extends Interceptor {
  _CaptchaOriginInterceptor(this._environment);

  final AppEnvironment _environment;

  @override
  void onRequest(RequestOptions options, RequestInterceptorHandler handler) {
    if (!_environment.ownsCaptcha(options.uri)) {
      handler.reject(
        DioException(
          requestOptions: options,
          type: DioExceptionType.unknown,
          error: const CaptchaFailure('已阻止访问未授权的验证码地址'),
        ),
      );
      return;
    }
    options.headers.removeWhere(
      (String key, Object? value) => key.toLowerCase() == 'authorization',
    );
    handler.next(options);
  }
}
