//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'media_retention_hold_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class MediaRetentionHoldInput {
  /// Returns a new [MediaRetentionHoldInput] instance.
  MediaRetentionHoldInput({
    required this.holdKind,

    required this.expiresAt,

    required this.reason,

    required this.expectedHoldId,
  });

  @JsonKey(
    name: r'holdKind',
    required: true,
    includeIfNull: false,
    unknownEnumValue: MediaRetentionHoldInputHoldKindEnum.unknownDefaultOpenApi,
  )
  final MediaRetentionHoldInputHoldKindEnum holdKind;

  /// Unix seconds, at least five minutes and no more than 365 days in the future.
  @JsonKey(name: r'expiresAt', required: true, includeIfNull: false)
  final int expiresAt;

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  /// Null creates only when no unreleased hold exists; a hold id renews exactly that reviewed record or returns conflict.
  @JsonKey(name: r'expectedHoldId', required: true, includeIfNull: true)
  final String? expectedHoldId;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is MediaRetentionHoldInput &&
          other.holdKind == holdKind &&
          other.expiresAt == expiresAt &&
          other.reason == reason &&
          other.expectedHoldId == expectedHoldId;

  @override
  int get hashCode =>
      holdKind.hashCode +
      expiresAt.hashCode +
      reason.hashCode +
      (expectedHoldId == null ? 0 : expectedHoldId.hashCode);

  factory MediaRetentionHoldInput.fromJson(Map<String, dynamic> json) =>
      _$MediaRetentionHoldInputFromJson(json);

  Map<String, dynamic> toJson() => _$MediaRetentionHoldInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum MediaRetentionHoldInputHoldKindEnum {
  @JsonValue(r'moderation')
  moderation(r'moderation'),
  @JsonValue(r'security')
  security(r'security'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const MediaRetentionHoldInputHoldKindEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
