//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'achievement_event.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AchievementEvent {
  /// Returns a new [AchievementEvent] instance.
  AchievementEvent({
    required this.id,

    required this.achievementId,

    required this.slug,

    required this.name,

    required this.action,

    required this.source_,

    required this.actorId,

    required this.reason,

    required this.createdAt,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'achievementId', required: true, includeIfNull: false)
  final String achievementId;

  @JsonKey(name: r'slug', required: true, includeIfNull: false)
  final String slug;

  @JsonKey(name: r'name', required: true, includeIfNull: false)
  final String name;

  @JsonKey(
    name: r'action',
    required: true,
    includeIfNull: false,
    unknownEnumValue: AchievementEventActionEnum.unknownDefaultOpenApi,
  )
  final AchievementEventActionEnum action;

  @JsonKey(
    name: r'source',
    required: true,
    includeIfNull: false,
    unknownEnumValue: AchievementEventSource_Enum.unknownDefaultOpenApi,
  )
  final AchievementEventSource_Enum source_;

  @JsonKey(name: r'actorId', required: true, includeIfNull: true)
  final String? actorId;

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AchievementEvent &&
          other.id == id &&
          other.achievementId == achievementId &&
          other.slug == slug &&
          other.name == name &&
          other.action == action &&
          other.source_ == source_ &&
          other.actorId == actorId &&
          other.reason == reason &&
          other.createdAt == createdAt;

  @override
  int get hashCode =>
      id.hashCode +
      achievementId.hashCode +
      slug.hashCode +
      name.hashCode +
      action.hashCode +
      source_.hashCode +
      (actorId == null ? 0 : actorId.hashCode) +
      reason.hashCode +
      createdAt.hashCode;

  factory AchievementEvent.fromJson(Map<String, dynamic> json) =>
      _$AchievementEventFromJson(json);

  Map<String, dynamic> toJson() => _$AchievementEventToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum AchievementEventActionEnum {
  @JsonValue(r'awarded')
  awarded(r'awarded'),
  @JsonValue(r'revoked')
  revoked(r'revoked'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const AchievementEventActionEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum AchievementEventSource_Enum {
  @JsonValue(r'automatic')
  automatic(r'automatic'),
  @JsonValue(r'manual')
  manual(r'manual'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const AchievementEventSource_Enum(this.value);

  final String value;

  @override
  String toString() => value;
}
