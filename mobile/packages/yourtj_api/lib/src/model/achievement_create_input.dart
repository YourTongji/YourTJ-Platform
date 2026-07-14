//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/achievement_icon.dart';
import 'package:json_annotation/json_annotation.dart';

part 'achievement_create_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AchievementCreateInput {
  /// Returns a new [AchievementCreateInput] instance.
  AchievementCreateInput({
    required this.slug,

    required this.name,

    this.description,

    required this.icon,

    required this.mintAmount,

    required this.reason,
  });

  @JsonKey(name: r'slug', required: true, includeIfNull: false)
  final String slug;

  @JsonKey(name: r'name', required: true, includeIfNull: false)
  final String name;

  @JsonKey(name: r'description', required: false, includeIfNull: false)
  final String? description;

  @JsonKey(
    name: r'icon',
    required: true,
    includeIfNull: false,
    unknownEnumValue: AchievementIcon.unknownDefaultOpenApi,
  )
  final AchievementIcon icon;

  // minimum: 0
  // maximum: 100000
  @JsonKey(name: r'mintAmount', required: true, includeIfNull: false)
  final int mintAmount;

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AchievementCreateInput &&
          other.slug == slug &&
          other.name == name &&
          other.description == description &&
          other.icon == icon &&
          other.mintAmount == mintAmount &&
          other.reason == reason;

  @override
  int get hashCode =>
      slug.hashCode +
      name.hashCode +
      (description == null ? 0 : description.hashCode) +
      icon.hashCode +
      mintAmount.hashCode +
      reason.hashCode;

  factory AchievementCreateInput.fromJson(Map<String, dynamic> json) =>
      _$AchievementCreateInputFromJson(json);

  Map<String, dynamic> toJson() => _$AchievementCreateInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
