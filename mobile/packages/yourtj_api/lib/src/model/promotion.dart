//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/promotion_metric_summary.dart';
import 'package:yourtj_api/src/model/media_delivery.dart';
import 'package:json_annotation/json_annotation.dart';

part 'promotion.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class Promotion {
  /// Returns a new [Promotion] instance.
  Promotion({
    required this.id,

    required this.placement,

    required this.title,

    this.body,

    this.ctaLabel,

    required this.targetUrl,

    this.assetId,

    required this.assetDelivery,

    required this.status,

    required this.effectiveState,

    required this.priority,

    required this.audience,

    required this.version,

    this.startsAt,

    this.endsAt,

    this.archivedAt,

    required this.createdAt,

    required this.updatedAt,

    required this.trackingToken,

    required this.metrics,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(
    name: r'placement',
    required: true,
    includeIfNull: false,
    unknownEnumValue: PromotionPlacementEnum.unknownDefaultOpenApi,
  )
  final PromotionPlacementEnum placement;

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

  /// Current short-lived sanitized CDN projection; null when no asset is configured or publication is unavailable.
  @JsonKey(name: r'assetDelivery', required: true, includeIfNull: true)
  final MediaDelivery? assetDelivery;

  @JsonKey(
    name: r'status',
    required: true,
    includeIfNull: false,
    unknownEnumValue: PromotionStatusEnum.unknownDefaultOpenApi,
  )
  final PromotionStatusEnum status;

  @JsonKey(
    name: r'effectiveState',
    required: true,
    includeIfNull: false,
    unknownEnumValue: PromotionEffectiveStateEnum.unknownDefaultOpenApi,
  )
  final PromotionEffectiveStateEnum effectiveState;

  // minimum: -1000
  // maximum: 1000
  @JsonKey(name: r'priority', required: true, includeIfNull: false)
  final int priority;

  @JsonKey(
    name: r'audience',
    required: true,
    includeIfNull: false,
    unknownEnumValue: PromotionAudienceEnum.unknownDefaultOpenApi,
  )
  final PromotionAudienceEnum audience;

  // minimum: 1
  @JsonKey(name: r'version', required: true, includeIfNull: false)
  final int version;

  @JsonKey(name: r'startsAt', required: false, includeIfNull: false)
  final int? startsAt;

  @JsonKey(name: r'endsAt', required: false, includeIfNull: false)
  final int? endsAt;

  @JsonKey(name: r'archivedAt', required: false, includeIfNull: false)
  final int? archivedAt;

  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @JsonKey(name: r'updatedAt', required: true, includeIfNull: false)
  final int updatedAt;

  /// Short-lived anonymous presentation token returned only by the active public list.
  @JsonKey(name: r'trackingToken', required: true, includeIfNull: true)
  final String? trackingToken;

  /// Rolling 30-day aggregate returned only by the administration list.
  @JsonKey(name: r'metrics', required: true, includeIfNull: true)
  final PromotionMetricSummary? metrics;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is Promotion &&
          other.id == id &&
          other.placement == placement &&
          other.title == title &&
          other.body == body &&
          other.ctaLabel == ctaLabel &&
          other.targetUrl == targetUrl &&
          other.assetId == assetId &&
          other.assetDelivery == assetDelivery &&
          other.status == status &&
          other.effectiveState == effectiveState &&
          other.priority == priority &&
          other.audience == audience &&
          other.version == version &&
          other.startsAt == startsAt &&
          other.endsAt == endsAt &&
          other.archivedAt == archivedAt &&
          other.createdAt == createdAt &&
          other.updatedAt == updatedAt &&
          other.trackingToken == trackingToken &&
          other.metrics == metrics;

  @override
  int get hashCode =>
      id.hashCode +
      placement.hashCode +
      title.hashCode +
      (body == null ? 0 : body.hashCode) +
      (ctaLabel == null ? 0 : ctaLabel.hashCode) +
      targetUrl.hashCode +
      (assetId == null ? 0 : assetId.hashCode) +
      (assetDelivery == null ? 0 : assetDelivery.hashCode) +
      status.hashCode +
      effectiveState.hashCode +
      priority.hashCode +
      audience.hashCode +
      version.hashCode +
      (startsAt == null ? 0 : startsAt.hashCode) +
      (endsAt == null ? 0 : endsAt.hashCode) +
      (archivedAt == null ? 0 : archivedAt.hashCode) +
      createdAt.hashCode +
      updatedAt.hashCode +
      (trackingToken == null ? 0 : trackingToken.hashCode) +
      (metrics == null ? 0 : metrics.hashCode);

  factory Promotion.fromJson(Map<String, dynamic> json) =>
      _$PromotionFromJson(json);

  Map<String, dynamic> toJson() => _$PromotionToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum PromotionPlacementEnum {
  @JsonValue(r'home-left-primary')
  homeLeftPrimary(r'home-left-primary'),
  @JsonValue(r'home-left-secondary')
  homeLeftSecondary(r'home-left-secondary'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const PromotionPlacementEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum PromotionStatusEnum {
  @JsonValue(r'draft')
  draft(r'draft'),
  @JsonValue(r'scheduled')
  scheduled(r'scheduled'),
  @JsonValue(r'published')
  published(r'published'),
  @JsonValue(r'paused')
  paused(r'paused'),
  @JsonValue(r'archived')
  archived(r'archived'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const PromotionStatusEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum PromotionEffectiveStateEnum {
  @JsonValue(r'draft')
  draft(r'draft'),
  @JsonValue(r'scheduled')
  scheduled(r'scheduled'),
  @JsonValue(r'active')
  active(r'active'),
  @JsonValue(r'paused')
  paused(r'paused'),
  @JsonValue(r'expired')
  expired(r'expired'),
  @JsonValue(r'archived')
  archived(r'archived'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const PromotionEffectiveStateEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum PromotionAudienceEnum {
  @JsonValue(r'all')
  all(r'all'),
  @JsonValue(r'authenticated')
  authenticated(r'authenticated'),
  @JsonValue(r'staff')
  staff(r'staff'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const PromotionAudienceEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
