//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'promotion_update_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class PromotionUpdateInput {
  /// Returns a new [PromotionUpdateInput] instance.
  PromotionUpdateInput({
    required this.placement,

    required this.title,

    this.body,

    this.ctaLabel,

    required this.targetUrl,

    this.assetId,

    required this.status,

    required this.priority,

    required this.audience,

    this.startsAt,

    this.endsAt,

    required this.reason,

    required this.expectedVersion,
  });

  @JsonKey(
    name: r'placement',
    required: true,
    includeIfNull: false,
    unknownEnumValue: PromotionUpdateInputPlacementEnum.unknownDefaultOpenApi,
  )
  final PromotionUpdateInputPlacementEnum placement;

  @JsonKey(name: r'title', required: true, includeIfNull: false)
  final String title;

  @JsonKey(name: r'body', required: false, includeIfNull: false)
  final String? body;

  @JsonKey(name: r'ctaLabel', required: false, includeIfNull: false)
  final String? ctaLabel;

  /// Same-origin relative application path.
  @JsonKey(name: r'targetUrl', required: true, includeIfNull: false)
  final String targetUrl;

  @JsonKey(name: r'assetId', required: false, includeIfNull: false)
  final String? assetId;

  @JsonKey(
    name: r'status',
    required: true,
    includeIfNull: false,
    unknownEnumValue: PromotionUpdateInputStatusEnum.unknownDefaultOpenApi,
  )
  final PromotionUpdateInputStatusEnum status;

  // minimum: -1000
  // maximum: 1000
  @JsonKey(name: r'priority', required: true, includeIfNull: false)
  final int priority;

  @JsonKey(
    name: r'audience',
    required: true,
    includeIfNull: false,
    unknownEnumValue: PromotionUpdateInputAudienceEnum.unknownDefaultOpenApi,
  )
  final PromotionUpdateInputAudienceEnum audience;

  @JsonKey(name: r'startsAt', required: false, includeIfNull: false)
  final int? startsAt;

  @JsonKey(name: r'endsAt', required: false, includeIfNull: false)
  final int? endsAt;

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  // minimum: 1
  @JsonKey(name: r'expectedVersion', required: true, includeIfNull: false)
  final int expectedVersion;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is PromotionUpdateInput &&
          other.placement == placement &&
          other.title == title &&
          other.body == body &&
          other.ctaLabel == ctaLabel &&
          other.targetUrl == targetUrl &&
          other.assetId == assetId &&
          other.status == status &&
          other.priority == priority &&
          other.audience == audience &&
          other.startsAt == startsAt &&
          other.endsAt == endsAt &&
          other.reason == reason &&
          other.expectedVersion == expectedVersion;

  @override
  int get hashCode =>
      placement.hashCode +
      title.hashCode +
      (body == null ? 0 : body.hashCode) +
      (ctaLabel == null ? 0 : ctaLabel.hashCode) +
      targetUrl.hashCode +
      (assetId == null ? 0 : assetId.hashCode) +
      status.hashCode +
      priority.hashCode +
      audience.hashCode +
      (startsAt == null ? 0 : startsAt.hashCode) +
      (endsAt == null ? 0 : endsAt.hashCode) +
      reason.hashCode +
      expectedVersion.hashCode;

  factory PromotionUpdateInput.fromJson(Map<String, dynamic> json) =>
      _$PromotionUpdateInputFromJson(json);

  Map<String, dynamic> toJson() => _$PromotionUpdateInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum PromotionUpdateInputPlacementEnum {
  @JsonValue(r'home-left-primary')
  homeLeftPrimary(r'home-left-primary'),
  @JsonValue(r'home-left-secondary')
  homeLeftSecondary(r'home-left-secondary'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const PromotionUpdateInputPlacementEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum PromotionUpdateInputStatusEnum {
  @JsonValue(r'draft')
  draft(r'draft'),
  @JsonValue(r'scheduled')
  scheduled(r'scheduled'),
  @JsonValue(r'published')
  published(r'published'),
  @JsonValue(r'paused')
  paused(r'paused'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const PromotionUpdateInputStatusEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum PromotionUpdateInputAudienceEnum {
  @JsonValue(r'all')
  all(r'all'),
  @JsonValue(r'authenticated')
  authenticated(r'authenticated'),
  @JsonValue(r'staff')
  staff(r'staff'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const PromotionUpdateInputAudienceEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
