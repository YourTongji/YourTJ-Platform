import 'dart:convert';

import 'package:flutter/foundation.dart';

@immutable
class AppEnvironment {
  AppEnvironment({
    required Uri apiBaseUri,
    Uri? captchaBaseUri,
    Uri? mediaCdnBaseUri,
  }) : apiBaseUri = _validateApi(apiBaseUri),
       captchaBaseUri = _validateExternal(
         captchaBaseUri ?? Uri.parse('https://captcha.07211024.xyz'),
         name: '验证码',
       ),
       mediaCdnBaseUri = _validateExternal(
         mediaCdnBaseUri ?? _originOf(_validateApi(apiBaseUri)),
         name: '媒体 CDN',
       ),
       storageNamespace = _storageNamespace(_validateApi(apiBaseUri));

  factory AppEnvironment.fromCompileTime() {
    const String configuredBase = String.fromEnvironment(
      'YOURTJ_API_BASE_URL',
      defaultValue: 'https://api.yourtj.de/api/v2',
    );
    const String configuredCaptcha = String.fromEnvironment(
      'YOURTJ_CAPTCHA_BASE_URL',
      defaultValue: 'https://captcha.07211024.xyz',
    );
    const String configuredMediaCdn = String.fromEnvironment(
      'YOURTJ_MEDIA_CDN_BASE_URL',
      defaultValue: 'https://media.yourtj.de',
    );
    return AppEnvironment(
      apiBaseUri: Uri.parse(configuredBase),
      captchaBaseUri: Uri.parse(configuredCaptcha),
      mediaCdnBaseUri: Uri.parse(configuredMediaCdn),
    );
  }

  final Uri apiBaseUri;
  final Uri captchaBaseUri;
  final Uri mediaCdnBaseUri;
  final String storageNamespace;

  static String _storageNamespace(Uri apiBaseUri) {
    return base64Url
        .encode(utf8.encode(apiBaseUri.toString()))
        .replaceAll('=', '');
  }

  static Uri _validateApi(Uri uri) {
    final bool isLoopback =
        uri.host == 'localhost' || uri.host == '127.0.0.1' || uri.host == '::1';
    final bool hasExpectedPath =
        uri.path == '/api/v2' || uri.path.endsWith('/api/v2');
    if (!uri.hasScheme ||
        uri.host.isEmpty ||
        uri.userInfo.isNotEmpty ||
        uri.hasQuery ||
        uri.hasFragment) {
      throw const FormatException('API 地址必须是绝对 URL，且不能包含查询或片段');
    }
    if (uri.scheme != 'https' &&
        !(kDebugMode && uri.scheme == 'http' && isLoopback)) {
      throw const FormatException('API 地址必须使用 HTTPS');
    }
    if (!hasExpectedPath) {
      throw const FormatException('API 地址必须以 /api/v2 结尾');
    }
    return uri.replace(path: uri.path.replaceFirst(RegExp(r'/$'), ''));
  }

  static Uri _validateExternal(Uri uri, {required String name}) {
    final bool isLoopback =
        uri.host == 'localhost' || uri.host == '127.0.0.1' || uri.host == '::1';
    if (!uri.hasScheme ||
        uri.host.isEmpty ||
        uri.userInfo.isNotEmpty ||
        uri.hasQuery ||
        uri.hasFragment ||
        (uri.path.isNotEmpty && uri.path != '/')) {
      throw FormatException('$name服务地址必须是 origin URL');
    }
    if (uri.scheme != 'https' &&
        !(kDebugMode && uri.scheme == 'http' && isLoopback)) {
      throw FormatException('$name服务地址必须使用 HTTPS');
    }
    return uri.replace(path: '');
  }

  bool owns(Uri requestUri) {
    return requestUri.scheme == apiBaseUri.scheme &&
        requestUri.host == apiBaseUri.host &&
        requestUri.port == apiBaseUri.port &&
        (requestUri.path == apiBaseUri.path ||
            requestUri.path.startsWith('${apiBaseUri.path}/'));
  }

  bool ownsCaptcha(Uri requestUri) {
    return requestUri.scheme == captchaBaseUri.scheme &&
        requestUri.host == captchaBaseUri.host &&
        requestUri.port == captchaBaseUri.port;
  }

  /// Allows platform images only from this environment's API or configured CDN origin.
  bool ownsPlatformMedia(Uri requestUri) {
    return _sameOrigin(requestUri, apiBaseUri) ||
        _sameOrigin(requestUri, mediaCdnBaseUri);
  }

  static Uri _originOf(Uri uri) {
    return uri.replace(path: '', query: null, fragment: null);
  }

  static bool _sameOrigin(Uri requestUri, Uri configuredUri) {
    return requestUri.scheme == configuredUri.scheme &&
        requestUri.host == configuredUri.host &&
        requestUri.port == configuredUri.port;
  }
}
