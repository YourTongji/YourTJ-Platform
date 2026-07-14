//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/achievement_icon.dart';
import 'package:yourtj_api/src/model/achievement_status.dart';
import 'package:json_annotation/json_annotation.dart';

part 'achievement_update_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AchievementUpdateInput {
  /// Returns a new [AchievementUpdateInput] instance.
  AchievementUpdateInput({
    required this.expectedVersion,

    required this.name,

    this.description,

    required this.icon,

    required this.status,

    required this.mintAmount,

    required this.reason,
  });

  // minimum: 1
  @JsonKey(name: r'expectedVersion', required: true, includeIfNull: false)
  final int expectedVersion;

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

  @JsonKey(
    name: r'status',
    required: true,
    includeIfNull: false,
    unknownEnumValue: AchievementStatus.unknownDefaultOpenApi,
  )
  final AchievementStatus status;

  // minimum: 0
  // maximum: 100000
  @JsonKey(name: r'mintAmount', required: true, includeIfNull: false)
  final int mintAmount;

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AchievementUpdateInput &&
          other.expectedVersion == expectedVersion &&
          other.name == name &&
          other.description == description &&
          other.icon == icon &&
          other.status == status &&
          other.mintAmount == mintAmount &&
          other.reason == reason;

  @override
  int get hashCode =>
      expectedVersion.hashCode +
      name.hashCode +
      (description == null ? 0 : description.hashCode) +
      icon.hashCode +
      status.hashCode +
      mintAmount.hashCode +
      reason.hashCode;

  factory AchievementUpdateInput.fromJson(Map<String, dynamic> json) =>
      _$AchievementUpdateInputFromJson(json);

  Map<String, dynamic> toJson() => _$AchievementUpdateInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
