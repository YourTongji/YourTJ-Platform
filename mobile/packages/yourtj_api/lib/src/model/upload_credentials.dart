//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'upload_credentials.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class UploadCredentials {
  /// Returns a new [UploadCredentials] instance.
  UploadCredentials({
    required this.uploadIntentId,

    required this.accessKeyId,

    required this.accessKeySecret,

    required this.securityToken,

    required this.region,

    required this.bucket,

    required this.prefix,

    required this.ossKey,

    required this.callbackUrl,

    required this.callbackBody,

    required this.expiration,
  });

  @JsonKey(name: r'uploadIntentId', required: true, includeIfNull: false)
  final String uploadIntentId;

  @JsonKey(name: r'accessKeyId', required: true, includeIfNull: false)
  final String accessKeyId;

  @JsonKey(name: r'accessKeySecret', required: true, includeIfNull: false)
  final String accessKeySecret;

  @JsonKey(name: r'securityToken', required: true, includeIfNull: false)
  final String securityToken;

  @JsonKey(name: r'region', required: true, includeIfNull: false)
  final String region;

  @JsonKey(name: r'bucket', required: true, includeIfNull: false)
  final String bucket;

  @JsonKey(name: r'prefix', required: true, includeIfNull: false)
  final String prefix;

  @JsonKey(name: r'ossKey', required: true, includeIfNull: false)
  final String ossKey;

  @JsonKey(name: r'callbackUrl', required: true, includeIfNull: false)
  final String callbackUrl;

  @JsonKey(name: r'callbackBody', required: true, includeIfNull: false)
  final String callbackBody;

  /// Unix seconds
  @JsonKey(name: r'expiration', required: true, includeIfNull: false)
  final int expiration;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is UploadCredentials &&
          other.uploadIntentId == uploadIntentId &&
          other.accessKeyId == accessKeyId &&
          other.accessKeySecret == accessKeySecret &&
          other.securityToken == securityToken &&
          other.region == region &&
          other.bucket == bucket &&
          other.prefix == prefix &&
          other.ossKey == ossKey &&
          other.callbackUrl == callbackUrl &&
          other.callbackBody == callbackBody &&
          other.expiration == expiration;

  @override
  int get hashCode =>
      uploadIntentId.hashCode +
      accessKeyId.hashCode +
      accessKeySecret.hashCode +
      securityToken.hashCode +
      region.hashCode +
      bucket.hashCode +
      prefix.hashCode +
      ossKey.hashCode +
      callbackUrl.hashCode +
      callbackBody.hashCode +
      expiration.hashCode;

  factory UploadCredentials.fromJson(Map<String, dynamic> json) =>
      _$UploadCredentialsFromJson(json);

  Map<String, dynamic> toJson() => _$UploadCredentialsToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
