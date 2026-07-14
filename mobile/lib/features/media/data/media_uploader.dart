import 'dart:convert';

import 'package:cryptography/cryptography.dart';
import 'package:dio/dio.dart';
import 'package:file_selector/file_selector.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../core/config/app_environment.dart';
import 'oss_v4_signer.dart';

enum MediaUploadKind { image, file }

class MediaUploadOwner {
  const MediaUploadOwner({required this.accountId, required this.generation});

  final String accountId;
  final int generation;
}

typedef MediaUploadOwnerReader = MediaUploadOwner? Function();

class CompletedMediaUpload {
  const CompletedMediaUpload({
    required this.uploadId,
    required this.originalName,
  });

  final String uploadId;
  final String originalName;
}

class MediaUploadFailure implements Exception {
  const MediaUploadFailure(this.message);

  final String message;

  @override
  String toString() => message;
}

class MediaUploader {
  factory MediaUploader({
    required MediaApi mediaApi,
    required AppEnvironment environment,
    required MediaUploadOwnerReader ownerReader,
    Dio? ossClient,
    OssV4Signer? signer,
    Sha256? sha256,
    DateTime Function()? now,
  }) {
    return MediaUploader._(
      mediaApi,
      environment,
      ownerReader,
      ossClient: ossClient,
      signer: signer,
      sha256: sha256,
      now: now,
    );
  }

  MediaUploader._(
    this._mediaApi,
    this._environment,
    this._ownerReader, {
    Dio? ossClient,
    OssV4Signer? signer,
    Sha256? sha256,
    DateTime Function()? now,
  }) : _ossClient =
           ossClient ??
           Dio(
             BaseOptions(
               connectTimeout: const Duration(seconds: 15),
               sendTimeout: const Duration(seconds: 60),
               receiveTimeout: const Duration(seconds: 30),
               followRedirects: false,
               maxRedirects: 0,
             ),
           ),
       _signer = signer ?? OssV4Signer(),
       _sha256 = sha256 ?? Sha256(),
       _now = now ?? DateTime.now;

  static const int maxUploadBytes = 20 * 1024 * 1024;

  final MediaApi _mediaApi;
  final AppEnvironment _environment;
  final MediaUploadOwnerReader _ownerReader;
  final Dio _ossClient;
  final OssV4Signer _signer;
  final Sha256 _sha256;
  final DateTime Function() _now;

  MediaUploadOwner captureOwner() {
    final MediaUploadOwner? owner = _ownerReader();
    if (owner == null) {
      throw const MediaUploadFailure('请先登录后上传媒体');
    }
    return owner;
  }

  void ensureCurrentOwner(MediaUploadOwner expected) {
    final MediaUploadOwner? current = _ownerReader();
    if (current == null ||
        current.accountId != expected.accountId ||
        current.generation != expected.generation) {
      throw const MediaUploadFailure('账号或登录状态已变化，已停止旧账号的媒体上传');
    }
  }

  Future<CompletedMediaUpload> upload({
    required MediaUploadOwner owner,
    required XFile file,
    required MediaUploadKind kind,
    MediaUsage? usage,
    ProgressCallback? onProgress,
  }) async {
    ensureCurrentOwner(owner);
    final int length = await file.length();
    ensureCurrentOwner(owner);
    final String contentType = _contentType(file, kind);
    _validateFile(length, contentType, kind);
    ensureCurrentOwner(owner);
    final List<int> bytes = await file.readAsBytes();
    ensureCurrentOwner(owner);
    if (bytes.length != length) {
      throw const MediaUploadFailure('读取文件时内容发生变化，请重新选择');
    }
    ensureCurrentOwner(owner);
    final Hash hash = await _sha256.hash(bytes);
    ensureCurrentOwner(owner);
    final String digest = _hex(hash.bytes);
    try {
      ensureCurrentOwner(owner);
      final Response<UploadCredentials> response = await _mediaApi
          .mediaUploadCredentialsPost(
            uploadIntentInput: UploadIntentInput(
              kind: kind == MediaUploadKind.image
                  ? UploadIntentInputKindEnum.image
                  : UploadIntentInputKindEnum.file,
              contentType: contentType,
              usage: usage,
            ),
          );
      ensureCurrentOwner(owner);
      final UploadCredentials? credentials = response.data;
      if (credentials == null) {
        throw const MediaUploadFailure('服务端没有返回完整的上传凭证');
      }
      _validateCredentials(credentials);
      ensureCurrentOwner(owner);
      final OssSignedUploadRequest request = await _signer.signUpload(
        credentials: credentials,
        contentType: contentType,
        contentSha256: digest,
        requestTime: _now(),
      );
      ensureCurrentOwner(owner);
      final Response<Object?> uploadResponse = await _ossClient.putUri<Object?>(
        request.url,
        data: Stream<List<int>>.value(bytes),
        options: Options(
          headers: <String, Object>{
            ...request.headers,
            Headers.contentLengthHeader: bytes.length,
          },
          contentType: contentType,
          responseType: ResponseType.json,
          validateStatus: (int? status) =>
              status != null && status >= 200 && status < 300,
        ),
        onSendProgress: onProgress,
      );
      ensureCurrentOwner(owner);
      final String uploadId = _parseUploadId(uploadResponse.data);
      ensureCurrentOwner(owner);
      return CompletedMediaUpload(uploadId: uploadId, originalName: file.name);
    } on MediaUploadFailure {
      rethrow;
    } on OssV4SigningFailure catch (failure) {
      throw MediaUploadFailure(failure.message);
    } on DioException catch (exception) {
      final bool isTimeout =
          exception.type == DioExceptionType.connectionTimeout ||
          exception.type == DioExceptionType.sendTimeout ||
          exception.type == DioExceptionType.receiveTimeout;
      throw MediaUploadFailure(
        isTimeout ? '上传超时，请检查网络后重新选择文件' : '上传失败，请稍后重新选择文件',
      );
    }
  }

  void _validateCredentials(UploadCredentials credentials) {
    final int nowSeconds = _now().toUtc().millisecondsSinceEpoch ~/ 1000;
    if (credentials.expiration <= nowSeconds) {
      throw const MediaUploadFailure('上传凭证已过期，请重新选择文件');
    }
    final Uri? callback = Uri.tryParse(credentials.callbackUrl);
    if (callback == null || !_environment.owns(callback)) {
      throw const MediaUploadFailure('上传回调不属于当前 YourTJ 环境');
    }
  }
}

void _validateFile(int length, String contentType, MediaUploadKind kind) {
  if (length <= 0 || length > MediaUploader.maxUploadBytes) {
    throw const MediaUploadFailure('文件大小必须在 1 B 到 20 MB 之间');
  }
  const Set<String> imageTypes = <String>{
    'image/jpeg',
    'image/png',
    'image/webp',
  };
  if (kind == MediaUploadKind.image && !imageTypes.contains(contentType)) {
    throw const MediaUploadFailure('仅支持静态 JPEG、PNG 或 WebP 图片；动图请转换后重新上传');
  }
  if (kind == MediaUploadKind.file && contentType != 'application/pdf') {
    throw const MediaUploadFailure('当前仅支持 PDF 文件');
  }
}

String _contentType(XFile file, MediaUploadKind kind) {
  final String? declared = file.mimeType?.trim().toLowerCase();
  if (declared != null && declared.isNotEmpty) {
    return declared;
  }
  final String lowerName = file.name.toLowerCase();
  if (lowerName.endsWith('.jpg') || lowerName.endsWith('.jpeg')) {
    return 'image/jpeg';
  }
  if (lowerName.endsWith('.png')) {
    return 'image/png';
  }
  if (lowerName.endsWith('.webp')) {
    return 'image/webp';
  }
  if (kind == MediaUploadKind.file && lowerName.endsWith('.pdf')) {
    return 'application/pdf';
  }
  return 'application/octet-stream';
}

String _parseUploadId(Object? value) {
  Object? decoded = value;
  if (decoded is String) {
    try {
      decoded = jsonDecode(decoded);
    } on FormatException {
      throw const MediaUploadFailure('OSS 回调返回了无效数据');
    }
  }
  if (decoded is! Map<String, dynamic>) {
    throw const MediaUploadFailure('OSS 回调没有返回上传记录');
  }
  final Object? uploadId = decoded['uploadId'];
  if (uploadId is! String && uploadId is! num) {
    throw const MediaUploadFailure('OSS 回调没有返回上传记录');
  }
  final String canonicalId = uploadId.toString();
  if (!RegExp(r'^[1-9][0-9]*$').hasMatch(canonicalId)) {
    throw const MediaUploadFailure('OSS 回调返回了无效的上传记录');
  }
  return canonicalId;
}

String _hex(List<int> value) =>
    value.map((int byte) => byte.toRadixString(16).padLeft(2, '0')).join();
