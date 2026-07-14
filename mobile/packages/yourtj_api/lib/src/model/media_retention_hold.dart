//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'media_retention_hold.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class MediaRetentionHold {
  /// Returns a new [MediaRetentionHold] instance.
  MediaRetentionHold({
    required this.id,

    required this.uploadId,

    required this.accountId,

    required this.uploadStatus,

    required this.holdKind,

    required this.reason,

    required this.placedBy,

    required this.expiresAt,

    required this.createdAt,

    required this.isExpired,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'uploadId', required: true, includeIfNull: false)
  final String uploadId;

  @JsonKey(name: r'accountId', required: true, includeIfNull: false)
  final String accountId;

  @JsonKey(
    name: r'uploadStatus',
    required: true,
    includeIfNull: false,
    unknownEnumValue: MediaRetentionHoldUploadStatusEnum.unknownDefaultOpenApi,
  )
  final MediaRetentionHoldUploadStatusEnum uploadStatus;

  @JsonKey(
    name: r'holdKind',
    required: true,
    includeIfNull: false,
    unknownEnumValue: MediaRetentionHoldHoldKindEnum.unknownDefaultOpenApi,
  )
  final MediaRetentionHoldHoldKindEnum holdKind;

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  @JsonKey(name: r'placedBy', required: true, includeIfNull: false)
  final String placedBy;

  /// Unix seconds
  @JsonKey(name: r'expiresAt', required: true, includeIfNull: false)
  final int expiresAt;

  /// Unix seconds
  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @JsonKey(name: r'isExpired', required: true, includeIfNull: false)
  final bool isExpired;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is MediaRetentionHold &&
          other.id == id &&
          other.uploadId == uploadId &&
          other.accountId == accountId &&
          other.uploadStatus == uploadStatus &&
          other.holdKind == holdKind &&
          other.reason == reason &&
          other.placedBy == placedBy &&
          other.expiresAt == expiresAt &&
          other.createdAt == createdAt &&
          other.isExpired == isExpired;

  @override
  int get hashCode =>
      id.hashCode +
      uploadId.hashCode +
      accountId.hashCode +
      uploadStatus.hashCode +
      holdKind.hashCode +
      reason.hashCode +
      placedBy.hashCode +
      expiresAt.hashCode +
      createdAt.hashCode +
      isExpired.hashCode;

  factory MediaRetentionHold.fromJson(Map<String, dynamic> json) =>
      _$MediaRetentionHoldFromJson(json);

  Map<String, dynamic> toJson() => _$MediaRetentionHoldToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum MediaRetentionHoldUploadStatusEnum {
  @JsonValue(r'pending')
  pending(r'pending'),
  @JsonValue(r'clean')
  clean(r'clean'),
  @JsonValue(r'quarantined')
  quarantined(r'quarantined'),
  @JsonValue(r'blocked')
  blocked(r'blocked'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const MediaRetentionHoldUploadStatusEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum MediaRetentionHoldHoldKindEnum {
  @JsonValue(r'moderation')
  moderation(r'moderation'),
  @JsonValue(r'security')
  security(r'security'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const MediaRetentionHoldHoldKindEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
