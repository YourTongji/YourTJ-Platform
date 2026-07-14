import 'dart:collection';
import 'dart:convert';

import 'package:cryptography/cryptography.dart';
import 'package:yourtj_api/yourtj_api.dart';

class OssSignedUploadRequest {
  const OssSignedUploadRequest({
    required this.url,
    required this.headers,
    required this.canonicalRequest,
  });

  final Uri url;
  final Map<String, String> headers;
  final String canonicalRequest;
}

class OssV4SigningFailure implements Exception {
  const OssV4SigningFailure(this.message);

  final String message;

  @override
  String toString() => message;
}

class OssV4Signer {
  OssV4Signer({Sha256? sha256, Hmac? hmac})
    : _sha256 = sha256 ?? Sha256(),
      _hmac = hmac ?? Hmac.sha256();

  final Sha256 _sha256;
  final Hmac _hmac;

  Future<OssSignedUploadRequest> signUpload({
    required UploadCredentials credentials,
    required String contentType,
    required String contentSha256,
    required DateTime requestTime,
  }) async {
    final String region = _normalizedRegion(credentials.region);
    _validateCredentials(credentials, region);
    if (!_isSha256(contentSha256)) {
      throw const OssV4SigningFailure('上传内容摘要无效');
    }
    final DateTime utc = requestTime.toUtc();
    final String signingDate = _date(utc);
    final String timestamp = '${signingDate}T${_time(utc)}Z';
    final String callback = base64Encode(
      utf8.encode(
        jsonEncode(<String, Object>{
          'callbackUrl': credentials.callbackUrl,
          'callbackBody': credentials.callbackBody,
          'callbackBodyType': 'application/json',
          'callbackSNI': true,
        }),
      ),
    );
    final String callbackVariables = base64Encode(
      utf8.encode(jsonEncode(<String, String>{'x:sha256': contentSha256})),
    );
    final SplayTreeMap<String, String> canonicalHeaders =
        SplayTreeMap<String, String>.from(<String, String>{
          'content-type': contentType.trim().toLowerCase(),
          'x-oss-callback': callback,
          'x-oss-callback-var': callbackVariables,
          'x-oss-content-sha256': 'UNSIGNED-PAYLOAD',
          'x-oss-date': timestamp,
          'x-oss-forbid-overwrite': 'true',
          'x-oss-security-token': credentials.securityToken,
        });
    final String canonicalHeadersText = canonicalHeaders.entries
        .map(
          (MapEntry<String, String> entry) =>
              '${entry.key}:${entry.value.trim()}\n',
        )
        .join();
    final String canonicalUri =
        '/${credentials.bucket}/${_encodeObjectKey(credentials.ossKey)}';
    final String canonicalRequest =
        'PUT\n$canonicalUri\n\n$canonicalHeadersText\n\nUNSIGNED-PAYLOAD';
    final String canonicalHash = _hex(
      (await _sha256.hash(utf8.encode(canonicalRequest))).bytes,
    );
    final String scope = '$signingDate/$region/oss/aliyun_v4_request';
    final String stringToSign =
        'OSS4-HMAC-SHA256\n$timestamp\n$scope\n$canonicalHash';
    final List<int> dateKey = await _mac(
      utf8.encode('aliyun_v4${credentials.accessKeySecret}'),
      signingDate,
    );
    final List<int> regionKey = await _mac(dateKey, region);
    final List<int> serviceKey = await _mac(regionKey, 'oss');
    final List<int> signingKey = await _mac(serviceKey, 'aliyun_v4_request');
    final String signature = _hex(await _mac(signingKey, stringToSign));
    final Map<String, String> headers = <String, String>{
      ...canonicalHeaders,
      'authorization':
          'OSS4-HMAC-SHA256 Credential=${credentials.accessKeyId}/$scope,Signature=$signature',
    };
    final Uri url = Uri(
      scheme: 'https',
      host: '${credentials.bucket}.oss-$region.aliyuncs.com',
      pathSegments: credentials.ossKey.split('/'),
    );
    return OssSignedUploadRequest(
      url: url,
      headers: Map<String, String>.unmodifiable(headers),
      canonicalRequest: canonicalRequest,
    );
  }

  Future<List<int>> _mac(List<int> key, String value) async {
    final Mac mac = await _hmac.calculateMac(
      utf8.encode(value),
      secretKey: SecretKey(key),
    );
    return mac.bytes;
  }
}

void _validateCredentials(UploadCredentials credentials, String region) {
  final RegExp bucketPattern = RegExp(r'^[a-z0-9][a-z0-9-]{1,61}[a-z0-9]$');
  final RegExp regionPattern = RegExp(r'^[a-z0-9][a-z0-9-]{1,62}$');
  if (!bucketPattern.hasMatch(credentials.bucket) ||
      !regionPattern.hasMatch(region)) {
    throw const OssV4SigningFailure('上传目标无效');
  }
  if (credentials.prefix.isEmpty ||
      credentials.ossKey.isEmpty ||
      !credentials.ossKey.startsWith(credentials.prefix) ||
      credentials.ossKey.startsWith('/') ||
      credentials.ossKey
          .split('/')
          .any(
            (String segment) =>
                segment.isEmpty || segment == '.' || segment == '..',
          )) {
    throw const OssV4SigningFailure('服务端返回的上传对象路径无效');
  }
  final Uri? callback = Uri.tryParse(credentials.callbackUrl);
  if (callback == null ||
      callback.scheme != 'https' ||
      callback.host.isEmpty ||
      callback.userInfo.isNotEmpty ||
      callback.fragment.isNotEmpty) {
    throw const OssV4SigningFailure('上传回调地址无效');
  }
  if (credentials.accessKeyId.isEmpty ||
      credentials.accessKeySecret.isEmpty ||
      credentials.securityToken.isEmpty) {
    throw const OssV4SigningFailure('临时上传凭证不完整');
  }
}

String _normalizedRegion(String value) {
  final String normalized = value.trim().toLowerCase();
  return normalized.startsWith('oss-') ? normalized.substring(4) : normalized;
}

String _encodeObjectKey(String value) {
  final StringBuffer encoded = StringBuffer();
  for (final int byte in utf8.encode(value)) {
    final bool isUnreserved =
        (byte >= 0x41 && byte <= 0x5a) ||
        (byte >= 0x61 && byte <= 0x7a) ||
        (byte >= 0x30 && byte <= 0x39) ||
        byte == 0x2d ||
        byte == 0x2e ||
        byte == 0x5f ||
        byte == 0x7e ||
        byte == 0x2f;
    if (isUnreserved) {
      encoded.writeCharCode(byte);
    } else {
      encoded.write('%${byte.toRadixString(16).padLeft(2, '0').toUpperCase()}');
    }
  }
  return encoded.toString();
}

bool _isSha256(String value) =>
    value.length == 64 && RegExp(r'^[0-9a-f]{64}$').hasMatch(value);

String _date(DateTime value) =>
    '${value.year.toString().padLeft(4, '0')}'
    '${value.month.toString().padLeft(2, '0')}'
    '${value.day.toString().padLeft(2, '0')}';

String _time(DateTime value) =>
    '${value.hour.toString().padLeft(2, '0')}'
    '${value.minute.toString().padLeft(2, '0')}'
    '${value.second.toString().padLeft(2, '0')}';

String _hex(List<int> value) =>
    value.map((int byte) => byte.toRadixString(16).padLeft(2, '0')).join();
