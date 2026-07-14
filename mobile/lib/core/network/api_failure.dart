import 'dart:convert';
import 'dart:io' show HttpDate;

import 'package:dio/dio.dart';

enum ApiFailureKind {
  cancelled,
  offline,
  timeout,
  unauthorized,
  forbidden,
  notFound,
  conflict,
  invalidInput,
  rateLimited,
  unavailable,
  server,
  unexpected,
}

class ApiFailure implements Exception {
  const ApiFailure({
    required this.kind,
    required this.message,
    this.code,
    this.statusCode,
    this.retryAfter,
  });

  factory ApiFailure.fromDio(DioException exception) {
    final int? statusCode = exception.response?.statusCode;
    final ({String? code, String? message}) envelope = _readEnvelope(
      exception.response?.data,
    );
    final ApiFailureKind kind = switch (exception.type) {
      DioExceptionType.cancel => ApiFailureKind.cancelled,
      DioExceptionType.connectionTimeout ||
      DioExceptionType.sendTimeout ||
      DioExceptionType.receiveTimeout => ApiFailureKind.timeout,
      DioExceptionType.connectionError => ApiFailureKind.offline,
      _ => _kindForStatus(statusCode),
    };
    return ApiFailure(
      kind: kind,
      message: envelope.message ?? _defaultMessage(kind),
      code: envelope.code,
      statusCode: statusCode,
      retryAfter: _parseRetryAfter(
        exception.response?.headers.value('retry-after'),
      ),
    );
  }

  final ApiFailureKind kind;
  final String message;
  final String? code;
  final int? statusCode;
  final Duration? retryAfter;

  bool get invalidatesRefreshCredential =>
      kind == ApiFailureKind.unauthorized ||
      code == 'INVALID_REFRESH_TOKEN' ||
      code == 'SESSION_REVOKED';

  static ApiFailureKind _kindForStatus(int? statusCode) {
    if (statusCode == 503) {
      return ApiFailureKind.unavailable;
    }
    if (statusCode != null && statusCode >= 500) {
      return ApiFailureKind.server;
    }
    return switch (statusCode) {
      400 || 422 => ApiFailureKind.invalidInput,
      401 => ApiFailureKind.unauthorized,
      403 => ApiFailureKind.forbidden,
      404 => ApiFailureKind.notFound,
      409 => ApiFailureKind.conflict,
      429 => ApiFailureKind.rateLimited,
      _ => ApiFailureKind.unexpected,
    };
  }

  static String _defaultMessage(ApiFailureKind kind) {
    return switch (kind) {
      ApiFailureKind.cancelled => '操作已取消',
      ApiFailureKind.offline => '网络不可用，请检查连接后重试',
      ApiFailureKind.timeout => '请求超时，请稍后重试',
      ApiFailureKind.unauthorized => '登录状态已失效，请重新登录',
      ApiFailureKind.forbidden => '当前账号没有执行此操作的权限',
      ApiFailureKind.notFound => '请求的内容不存在或已不可见',
      ApiFailureKind.conflict => '内容已发生变化，请刷新后重试',
      ApiFailureKind.invalidInput => '提交内容不符合要求，请检查后重试',
      ApiFailureKind.rateLimited => '操作过于频繁，请稍后重试',
      ApiFailureKind.unavailable => '服务正在维护或暂时不可用，请稍后重试',
      ApiFailureKind.server => '服务暂时不可用，请稍后重试',
      ApiFailureKind.unexpected => '请求失败，请稍后重试',
    };
  }

  static Duration? _parseRetryAfter(String? value) {
    if (value == null) {
      return null;
    }
    final int? seconds = int.tryParse(value.trim());
    if (seconds != null && seconds >= 0) {
      return Duration(seconds: seconds);
    }
    DateTime date;
    try {
      date = HttpDate.parse(value);
    } on FormatException {
      return null;
    }
    final Duration delay = date.toUtc().difference(DateTime.now().toUtc());
    return delay.isNegative ? Duration.zero : delay;
  }

  static ({String? code, String? message}) _readEnvelope(Object? data) {
    Object? decoded = data;
    if (data is String) {
      try {
        decoded = jsonDecode(data);
      } on FormatException {
        return (code: null, message: null);
      }
    }
    if (decoded is! Map) {
      return (code: null, message: null);
    }
    final Object? rawError = decoded['error'];
    if (rawError is! Map) {
      return (code: null, message: null);
    }
    final Object? rawCode = rawError['code'];
    final Object? rawMessage = rawError['message'];
    return (
      code: rawCode is String ? rawCode : null,
      message: rawMessage is String && rawMessage.trim().isNotEmpty
          ? rawMessage
          : null,
    );
  }

  @override
  String toString() => message;
}
