//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/achievement_icon.dart';
import 'package:yourtj_api/src/model/achievement_status.dart';
import 'package:json_annotation/json_annotation.dart';

part 'achievement.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class Achievement {
  /// Returns a new [Achievement] instance.
  Achievement({
    required this.id,

    required this.slug,

    required this.name,

    required this.description,

    required this.icon,

    required this.status,

    required this.mintAmount,

    required this.version,

    required this.createdAt,

    required this.updatedAt,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'slug', required: true, includeIfNull: false)
  final String slug;

  @JsonKey(name: r'name', required: true, includeIfNull: false)
  final String name;

  @JsonKey(name: r'description', required: true, includeIfNull: true)
  final String? description;

  @JsonKey(
    name: r'icon',
    required: true,
    includeIfNull: false,
    unknownEnumValue: AchievementIcon.unknownDefaultOpenApi,
  )
  final AchievementIcon icon;

  @JsonKey(
    name: r'status',
    required: true,
    includeIfNull: false,
    unknownEnumValue: AchievementStatus.unknownDefaultOpenApi,
  )
  final AchievementStatus status;

  // minimum: 0
  @JsonKey(name: r'mintAmount', required: true, includeIfNull: false)
  final int mintAmount;

  // minimum: 1
  @JsonKey(name: r'version', required: true, includeIfNull: false)
  final int version;

  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @JsonKey(name: r'updatedAt', required: true, includeIfNull: false)
  final int updatedAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is Achievement &&
          other.id == id &&
          other.slug == slug &&
          other.name == name &&
          other.description == description &&
          other.icon == icon &&
          other.status == status &&
          other.mintAmount == mintAmount &&
          other.version == version &&
          other.createdAt == createdAt &&
          other.updatedAt == updatedAt;

  @override
  int get hashCode =>
      id.hashCode +
      slug.hashCode +
      name.hashCode +
      (description == null ? 0 : description.hashCode) +
      icon.hashCode +
      status.hashCode +
      mintAmount.hashCode +
      version.hashCode +
      createdAt.hashCode +
      updatedAt.hashCode;

  factory Achievement.fromJson(Map<String, dynamic> json) =>
      _$AchievementFromJson(json);

  Map<String, dynamic> toJson() => _$AchievementToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
