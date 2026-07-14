import 'dart:typed_data';

import 'package:dio/dio.dart';
import 'package:test/test.dart';
import 'package:yourtj_api/yourtj_api.dart';

void main() {
  test('encodes a media delivery variant with its wire value', () async {
    final adapter = RecordingAdapter();
    final dio = Dio(BaseOptions(baseUrl: 'https://api.yourtj.de/api/v2'))
      ..httpClientAdapter = adapter;

    await MediaApi(
      dio,
    ).mediaIdUrlGet(id: 'asset-1', variant: MediaDeliveryVariant.display1280);

    expect(adapter.requestedUri?.queryParameters, <String, String>{
      'variant': 'display_1280',
    });
  });
}

class RecordingAdapter implements HttpClientAdapter {
  Uri? requestedUri;

  @override
  Future<ResponseBody> fetch(
    RequestOptions options,
    Stream<Uint8List>? requestStream,
    Future<void>? cancelFuture,
  ) async {
    requestedUri = options.uri;
    return ResponseBody.fromString(
      'null',
      200,
      headers: <String, List<String>>{
        Headers.contentTypeHeader: <String>[Headers.jsonContentType],
      },
    );
  }

  @override
  void close({bool force = false}) {}
}
