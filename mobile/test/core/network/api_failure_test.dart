import 'dart:convert';
import 'dart:io';

import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_mobile/core/network/api_failure.dart';

void main() {
  group('ApiFailure', () {
    test('preserves the bounded platform error envelope', () {
      final ApiFailure failure = ApiFailure.fromDio(
        responseException(
          statusCode: 422,
          data: <String, Object?>{
            'error': <String, Object?>{
              'code': 'HANDLE_INVALID',
              'message': '昵称格式不正确',
              'details': <String, Object?>{'ignored': true},
            },
          },
        ),
      );

      expect(failure.kind, ApiFailureKind.invalidInput);
      expect(failure.statusCode, 422);
      expect(failure.code, 'HANDLE_INVALID');
      expect(failure.message, '昵称格式不正确');
    });

    test('parses a JSON-string envelope without retaining raw data', () {
      final ApiFailure failure = ApiFailure.fromDio(
        responseException(
          statusCode: 409,
          data: jsonEncode(<String, Object?>{
            'error': <String, Object?>{
              'code': 'VERSION_CONFLICT',
              'message': '内容已更新',
            },
          }),
        ),
      );

      expect(failure.kind, ApiFailureKind.conflict);
      expect(failure.code, 'VERSION_CONFLICT');
      expect(failure.toString(), '内容已更新');
    });

    test('maps transport failures before status handling', () {
      expect(
        failureForType(DioExceptionType.cancel).kind,
        ApiFailureKind.cancelled,
      );
      expect(
        failureForType(DioExceptionType.connectionError).kind,
        ApiFailureKind.offline,
      );
      for (final DioExceptionType type in <DioExceptionType>[
        DioExceptionType.connectionTimeout,
        DioExceptionType.sendTimeout,
        DioExceptionType.receiveTimeout,
      ]) {
        expect(
          failureForType(type).kind,
          ApiFailureKind.timeout,
          reason: '$type',
        );
      }
    });

    test('maps important HTTP statuses to recovery kinds', () {
      final Map<int, ApiFailureKind> cases = <int, ApiFailureKind>{
        400: ApiFailureKind.invalidInput,
        401: ApiFailureKind.unauthorized,
        403: ApiFailureKind.forbidden,
        404: ApiFailureKind.notFound,
        409: ApiFailureKind.conflict,
        429: ApiFailureKind.rateLimited,
        500: ApiFailureKind.server,
        503: ApiFailureKind.unavailable,
        418: ApiFailureKind.unexpected,
      };

      for (final MapEntry<int, ApiFailureKind> entry in cases.entries) {
        final ApiFailure failure = ApiFailure.fromDio(
          responseException(statusCode: entry.key),
        );
        expect(failure.kind, entry.value, reason: '${entry.key}');
        expect(failure.statusCode, entry.key);
      }
    });

    test('parses Retry-After seconds and HTTP dates for rate limits', () {
      final ApiFailure seconds = ApiFailure.fromDio(
        responseException(
          statusCode: 429,
          headers: Headers.fromMap(<String, List<String>>{
            'retry-after': <String>['17'],
          }),
        ),
      );
      final DateTime retryDate = DateTime.now().toUtc().add(
        const Duration(minutes: 2),
      );
      final ApiFailure httpDate = ApiFailure.fromDio(
        responseException(
          statusCode: 429,
          headers: Headers.fromMap(<String, List<String>>{
            'retry-after': <String>[HttpDate.format(retryDate)],
          }),
        ),
      );

      expect(seconds.kind, ApiFailureKind.rateLimited);
      expect(seconds.retryAfter, const Duration(seconds: 17));
      expect(httpDate.kind, ApiFailureKind.rateLimited);
      expect(
        httpDate.retryAfter,
        isA<Duration>()
            .having(
              (Duration duration) => duration,
              'lower bound',
              greaterThan(const Duration(minutes: 1, seconds: 55)),
            )
            .having(
              (Duration duration) => duration,
              'upper bound',
              lessThanOrEqualTo(const Duration(minutes: 2)),
            ),
      );
    });

    test('recognizes terminal refresh failures by status or code', () {
      final ApiFailure unauthorized = ApiFailure.fromDio(
        responseException(statusCode: 401),
      );
      final ApiFailure revoked = ApiFailure.fromDio(
        responseException(
          statusCode: 409,
          data: <String, Object?>{
            'error': <String, String>{
              'code': 'SESSION_REVOKED',
              'message': 'revoked',
            },
          },
        ),
      );
      final ApiFailure offline = failureForType(
        DioExceptionType.connectionError,
      );

      expect(unauthorized.invalidatesRefreshCredential, isTrue);
      expect(revoked.invalidatesRefreshCredential, isTrue);
      expect(offline.invalidatesRefreshCredential, isFalse);
    });

    test('uses safe defaults for malformed envelopes', () {
      for (final Object? data in <Object?>[
        'not-json',
        <String, Object?>{'error': 'raw database error'},
        <String, Object?>{
          'error': <String, Object?>{'message': '   ', 'code': 123},
        },
      ]) {
        final ApiFailure failure = ApiFailure.fromDio(
          responseException(statusCode: 500, data: data),
        );
        expect(failure.kind, ApiFailureKind.server);
        expect(failure.code, isNull);
        expect(failure.message, '服务暂时不可用，请稍后重试');
      }
    });
  });
}

DioException responseException({
  required int statusCode,
  Object? data,
  Headers? headers,
}) {
  final RequestOptions options = RequestOptions(
    path: '/test',
    baseUrl: 'https://api.yourtj.de/api/v2',
  );
  return DioException.badResponse(
    statusCode: statusCode,
    requestOptions: options,
    response: Response<Object?>(
      requestOptions: options,
      statusCode: statusCode,
      data: data,
      headers: headers,
    ),
  );
}

ApiFailure failureForType(DioExceptionType type) {
  return ApiFailure.fromDio(
    DioException(
      requestOptions: RequestOptions(path: '/test'),
      type: type,
    ),
  );
}
