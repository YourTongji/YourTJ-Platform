//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'subscription.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class Subscription {
  /// Returns a new [Subscription] instance.
  Subscription({
    required this.targetType,

    required this.targetId,

    required this.level,

    required this.createdAt,
  });

  @JsonKey(
    name: r'targetType',
    required: true,
    includeIfNull: false,
    unknownEnumValue: SubscriptionTargetTypeEnum.unknownDefaultOpenApi,
  )
  final SubscriptionTargetTypeEnum targetType;

  @JsonKey(name: r'targetId', required: true, includeIfNull: false)
  final String targetId;

  @JsonKey(
    name: r'level',
    required: true,
    includeIfNull: false,
    unknownEnumValue: SubscriptionLevelEnum.unknownDefaultOpenApi,
  )
  final SubscriptionLevelEnum level;

  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is Subscription &&
          other.targetType == targetType &&
          other.targetId == targetId &&
          other.level == level &&
          other.createdAt == createdAt;

  @override
  int get hashCode =>
      targetType.hashCode +
      targetId.hashCode +
      level.hashCode +
      createdAt.hashCode;

  factory Subscription.fromJson(Map<String, dynamic> json) =>
      _$SubscriptionFromJson(json);

  Map<String, dynamic> toJson() => _$SubscriptionToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum SubscriptionTargetTypeEnum {
  @JsonValue(r'board')
  board(r'board'),
  @JsonValue(r'thread')
  thread(r'thread'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const SubscriptionTargetTypeEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum SubscriptionLevelEnum {
  @JsonValue(r'watching')
  watching(r'watching'),
  @JsonValue(r'tracking')
  tracking(r'tracking'),
  @JsonValue(r'muted')
  muted(r'muted'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const SubscriptionLevelEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
