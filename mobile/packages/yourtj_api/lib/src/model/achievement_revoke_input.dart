//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'achievement_revoke_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AchievementRevokeInput {
  /// Returns a new [AchievementRevokeInput] instance.
  AchievementRevokeInput({required this.reason});

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AchievementRevokeInput && other.reason == reason;

  @override
  int get hashCode => reason.hashCode;

  factory AchievementRevokeInput.fromJson(Map<String, dynamic> json) =>
      _$AchievementRevokeInputFromJson(json);

  Map<String, dynamic> toJson() => _$AchievementRevokeInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
