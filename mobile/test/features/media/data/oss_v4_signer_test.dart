import 'dart:convert';
import 'dart:io';

import 'package:cryptography/cryptography.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/features/media/data/oss_v4_signer.dart';

void main() {
  test('matches the shared OSS V4 callback upload vector', () async {
    final Map<String, dynamic> fixture =
        jsonDecode(
              File(
                '../contract/fixtures/oss-v4-signing-v1.json',
              ).readAsStringSync(),
            )
            as Map<String, dynamic>;
    final Map<String, dynamic> vector =
        (fixture['vectors']! as List<dynamic>).single as Map<String, dynamic>;
    final UploadCredentials credentials = _credentials(vector);

    final OssSignedUploadRequest request = await OssV4Signer().signUpload(
      credentials: credentials,
      contentType: vector['contentType']! as String,
      contentSha256: vector['contentSha256']! as String,
      requestTime: DateTime.parse(vector['requestTime']! as String),
    );
    final Hash canonicalHash = await Sha256().hash(
      utf8.encode(request.canonicalRequest),
    );

    expect(_hex(canonicalHash.bytes), vector['canonicalRequestSha256']);
    expect(request.headers['authorization'], vector['authorization']);
    expect(request.headers['x-oss-security-token'], 'temporary-token');
    expect(request.headers['x-oss-forbid-overwrite'], 'true');
    expect(
      request.url.toString(),
      'https://yourtj-test.oss-cn-shanghai.aliyuncs.com/'
      'uploads/1/image/%E4%B8%BB%E9%A2%98.png',
    );
  });

  test('rejects a key outside the server-issued prefix', () async {
    final UploadCredentials credentials = UploadCredentials(
      uploadIntentId: 'intent',
      accessKeyId: 'id',
      accessKeySecret: 'secret',
      securityToken: 'token',
      region: 'cn-shanghai',
      bucket: 'yourtj-test',
      prefix: 'uploads/account/image/',
      ossKey: 'uploads/other/image/file.png',
      callbackUrl: 'https://api.example.test/api/v2/media/callback',
      callbackBody: '{}',
      expiration: 2000000000,
    );

    expect(
      () => OssV4Signer().signUpload(
        credentials: credentials,
        contentType: 'image/png',
        contentSha256: 'a' * 64,
        requestTime: DateTime.utc(2026),
      ),
      throwsA(isA<OssV4SigningFailure>()),
    );
  });
}

UploadCredentials _credentials(Map<String, dynamic> vector) {
  return UploadCredentials(
    uploadIntentId: 'intent',
    accessKeyId: vector['accessKeyId']! as String,
    accessKeySecret: vector['accessKeySecret']! as String,
    securityToken: vector['securityToken']! as String,
    region: vector['region']! as String,
    bucket: vector['bucket']! as String,
    prefix: vector['prefix']! as String,
    ossKey: vector['ossKey']! as String,
    callbackUrl: vector['callbackUrl']! as String,
    callbackBody: vector['callbackBody']! as String,
    expiration: 2000000000,
  );
}

String _hex(List<int> bytes) =>
    bytes.map((int byte) => byte.toRadixString(16).padLeft(2, '0')).join();
