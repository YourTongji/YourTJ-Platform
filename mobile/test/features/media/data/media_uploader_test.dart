import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:dio/dio.dart';
import 'package:file_selector/file_selector.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/core/config/app_environment.dart';
import 'package:yourtj_mobile/features/media/data/media_uploader.dart';

void main() {
  const MediaUploadOwner owner = MediaUploadOwner(
    accountId: 'account-1',
    generation: 7,
  );

  test(
    'uploads the exact key without forwarding the platform bearer',
    () async {
      final _RecordingAdapter apiAdapter = _RecordingAdapter(
        response: ResponseBody.fromString(
          jsonEncode(_credentialsJson()),
          200,
          headers: <String, List<String>>{
            Headers.contentTypeHeader: <String>['application/json'],
          },
        ),
      );
      final Dio apiDio = Dio(
        BaseOptions(baseUrl: 'https://api.example.test/api/v2'),
      )..httpClientAdapter = apiAdapter;
      final _RecordingAdapter ossAdapter = _RecordingAdapter(
        response: ResponseBody.fromString(
          '{"uploadId":"42"}',
          200,
          headers: <String, List<String>>{
            Headers.contentTypeHeader: <String>['application/json'],
          },
        ),
      );
      final Dio ossDio = Dio()..httpClientAdapter = ossAdapter;
      final MediaUploader uploader = MediaUploader(
        mediaApi: MediaApi(apiDio),
        environment: AppEnvironment(
          apiBaseUri: Uri.parse('https://api.example.test/api/v2'),
        ),
        ownerReader: () => owner,
        ossClient: ossDio,
        now: () => DateTime.utc(2026, 7, 11, 8, 9, 10),
      );

      final CompletedMediaUpload completed = await uploader.upload(
        owner: uploader.captureOwner(),
        file: XFile.fromData(
          Uint8List.fromList(utf8.encode('static image')),
          name: 'photo.png',
          mimeType: 'image/png',
        ),
        kind: MediaUploadKind.image,
      );

      expect(completed.uploadId, '42');
      expect(
        apiAdapter.requests.single.uri.toString(),
        'https://api.example.test/api/v2/media/upload-credentials',
      );
      final RequestOptions ossRequest = ossAdapter.requests.single;
      expect(
        ossRequest.uri.toString(),
        'https://yourtj-test.oss-cn-shanghai.aliyuncs.com/'
        'uploads/1/image/intent.png',
      );
      expect(
        ossRequest.headers['authorization'],
        startsWith('OSS4-HMAC-SHA256'),
      );
      expect(ossRequest.headers['x-oss-security-token'], 'temporary-token');
      expect(ossRequest.headers['x-oss-forbid-overwrite'], 'true');
      expect(ossRequest.headers.values, isNot(contains('platform-bearer')));
      expect(ossAdapter.requestBodies.single, utf8.encode('static image'));
    },
  );

  test('rejects unsupported media before issuing credentials', () async {
    final _RecordingAdapter apiAdapter = _RecordingAdapter(
      response: ResponseBody.fromString('{}', 200),
    );
    final Dio apiDio = Dio(
      BaseOptions(baseUrl: 'https://api.example.test/api/v2'),
    )..httpClientAdapter = apiAdapter;
    final MediaUploader uploader = MediaUploader(
      mediaApi: MediaApi(apiDio),
      environment: AppEnvironment(
        apiBaseUri: Uri.parse('https://api.example.test/api/v2'),
      ),
      ownerReader: () => owner,
    );

    expect(
      () => uploader.upload(
        owner: uploader.captureOwner(),
        file: XFile.fromData(
          Uint8List.fromList(<int>[1, 2, 3]),
          name: 'animated.gif',
          mimeType: 'image/gif',
        ),
        kind: MediaUploadKind.image,
      ),
      throwsA(
        isA<MediaUploadFailure>().having(
          (MediaUploadFailure failure) => failure.message,
          'message',
          contains('静态'),
        ),
      ),
    );
    expect(apiAdapter.requests, isEmpty);
  });

  test(
    'stops before OSS when the account changes while requesting credentials',
    () async {
      final Completer<ResponseBody> credentialsResponse =
          Completer<ResponseBody>();
      final _RecordingAdapter apiAdapter = _RecordingAdapter.deferred(
        credentialsResponse.future,
      );
      final Dio apiDio = Dio(
        BaseOptions(baseUrl: 'https://api.example.test/api/v2'),
      )..httpClientAdapter = apiAdapter;
      final _RecordingAdapter ossAdapter = _RecordingAdapter(
        response: ResponseBody.fromString('{"uploadId":"42"}', 200),
      );
      final Dio ossDio = Dio()..httpClientAdapter = ossAdapter;
      MediaUploadOwner? currentOwner = owner;
      final MediaUploader uploader = MediaUploader(
        mediaApi: MediaApi(apiDio),
        environment: AppEnvironment(
          apiBaseUri: Uri.parse('https://api.example.test/api/v2'),
        ),
        ownerReader: () => currentOwner,
        ossClient: ossDio,
        now: () => DateTime.utc(2026, 7, 11, 8, 9, 10),
      );

      final Future<void> expectation = expectLater(
        uploader.upload(
          owner: uploader.captureOwner(),
          file: XFile.fromData(
            Uint8List.fromList(utf8.encode('static image')),
            name: 'photo.png',
            mimeType: 'image/png',
          ),
          kind: MediaUploadKind.image,
        ),
        throwsA(_isOwnerChangeFailure),
      );
      await apiAdapter.whenRequested;
      currentOwner = const MediaUploadOwner(
        accountId: 'account-2',
        generation: 8,
      );
      credentialsResponse.complete(
        ResponseBody.fromString(
          jsonEncode(_credentialsJson()),
          200,
          headers: <String, List<String>>{
            Headers.contentTypeHeader: <String>['application/json'],
          },
        ),
      );

      await expectation;
      expect(ossAdapter.requests, isEmpty);
    },
  );

  test('rejects the OSS callback result after an account switch', () async {
    final _RecordingAdapter apiAdapter = _RecordingAdapter(
      response: ResponseBody.fromString(
        jsonEncode(_credentialsJson()),
        200,
        headers: <String, List<String>>{
          Headers.contentTypeHeader: <String>['application/json'],
        },
      ),
    );
    final Dio apiDio = Dio(
      BaseOptions(baseUrl: 'https://api.example.test/api/v2'),
    )..httpClientAdapter = apiAdapter;
    final Completer<ResponseBody> callbackResponse = Completer<ResponseBody>();
    final _RecordingAdapter ossAdapter = _RecordingAdapter.deferred(
      callbackResponse.future,
    );
    final Dio ossDio = Dio()..httpClientAdapter = ossAdapter;
    MediaUploadOwner? currentOwner = owner;
    final MediaUploader uploader = MediaUploader(
      mediaApi: MediaApi(apiDio),
      environment: AppEnvironment(
        apiBaseUri: Uri.parse('https://api.example.test/api/v2'),
      ),
      ownerReader: () => currentOwner,
      ossClient: ossDio,
      now: () => DateTime.utc(2026, 7, 11, 8, 9, 10),
    );

    final Future<void> expectation = expectLater(
      uploader.upload(
        owner: uploader.captureOwner(),
        file: XFile.fromData(
          Uint8List.fromList(utf8.encode('static image')),
          name: 'photo.png',
          mimeType: 'image/png',
        ),
        kind: MediaUploadKind.image,
      ),
      throwsA(_isOwnerChangeFailure),
    );
    await ossAdapter.whenRequested;
    currentOwner = const MediaUploadOwner(
      accountId: 'account-2',
      generation: 8,
    );
    callbackResponse.complete(
      ResponseBody.fromString(
        '{"uploadId":"42"}',
        200,
        headers: <String, List<String>>{
          Headers.contentTypeHeader: <String>['application/json'],
        },
      ),
    );

    await expectation;
  });

  test('does not deliver a completed upload to a replaced session', () {
    MediaUploadOwner? currentOwner = owner;
    final MediaUploader uploader = MediaUploader(
      mediaApi: MediaApi(Dio()),
      environment: AppEnvironment(
        apiBaseUri: Uri.parse('https://api.example.test/api/v2'),
      ),
      ownerReader: () => currentOwner,
    );
    final MediaUploadOwner capturedOwner = uploader.captureOwner();
    bool delivered = false;

    currentOwner = const MediaUploadOwner(
      accountId: 'account-1',
      generation: 8,
    );

    expect(() {
      uploader.ensureCurrentOwner(capturedOwner);
      delivered = true;
    }, throwsA(_isOwnerChangeFailure));
    expect(delivered, isFalse);
  });
}

final Matcher _isOwnerChangeFailure = isA<MediaUploadFailure>().having(
  (MediaUploadFailure failure) => failure.message,
  'message',
  contains('账号或登录状态已变化'),
);

Map<String, Object> _credentialsJson() {
  return <String, Object>{
    'uploadIntentId': 'intent',
    'accessKeyId': 'temporary-id',
    'accessKeySecret': 'temporary-secret',
    'securityToken': 'temporary-token',
    'region': 'cn-shanghai',
    'bucket': 'yourtj-test',
    'prefix': 'uploads/1/image/',
    'ossKey': 'uploads/1/image/intent.png',
    'callbackUrl': 'https://api.example.test/api/v2/media/callback',
    'callbackBody':
        '{"uploadIntentId":"intent","ossKey":\${object},"bytes":\${size},'
        '"mime":\${mimeType},"sha256":\${x:sha256}}',
    'expiration': 2000000000,
  };
}

class _RecordingAdapter implements HttpClientAdapter {
  _RecordingAdapter({required ResponseBody response})
    : _response = Future<ResponseBody>.value(response);

  _RecordingAdapter.deferred(this._response);

  final Future<ResponseBody> _response;
  final Completer<void> _requestStarted = Completer<void>();
  final List<RequestOptions> requests = <RequestOptions>[];
  final List<List<int>> requestBodies = <List<int>>[];

  Future<void> get whenRequested => _requestStarted.future;

  @override
  void close({bool force = false}) {}

  @override
  Future<ResponseBody> fetch(
    RequestOptions options,
    Stream<Uint8List>? requestStream,
    Future<void>? cancelFuture,
  ) async {
    requests.add(options);
    final List<int> body = <int>[];
    if (requestStream != null) {
      await for (final Uint8List chunk in requestStream) {
        body.addAll(chunk);
      }
    }
    requestBodies.add(body);
    if (!_requestStarted.isCompleted) {
      _requestStarted.complete();
    }
    return _response;
  }
}
