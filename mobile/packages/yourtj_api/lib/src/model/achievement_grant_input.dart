//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'achievement_grant_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AchievementGrantInput {
  /// Returns a new [AchievementGrantInput] instance.
  AchievementGrantInput({required this.achievementId, required this.reason});

  @JsonKey(name: r'achievementId', required: true, includeIfNull: false)
  final String achievementId;

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AchievementGrantInput &&
          other.achievementId == achievementId &&
          other.reason == reason;

  @override
  int get hashCode => achievementId.hashCode + reason.hashCode;

  factory AchievementGrantInput.fromJson(Map<String, dynamic> json) =>
      _$AchievementGrantInputFromJson(json);

  Map<String, dynamic> toJson() => _$AchievementGrantInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
